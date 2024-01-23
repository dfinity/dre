use std::cmp::Ordering;
use std::io::Write;
use std::time::Duration;

use anyhow::anyhow;
use clap::Parser;
use log::{error, info, warn};
use pretty_env_logger::formatted_builder;
use reqwest::{ClientBuilder, StatusCode};
use tokio::select;
use tokio::sync::mpsc;
use url::Url;

use crate::entry::Entry;

mod entry;

#[allow(clippy::iter_skip_zero)]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();
    init_logger(args.name.clone());

    info!("Running with args: {:?}", args);

    let client = match ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(anyhow!("Error while constructing the client: {:?}", e)),
    };

    let (ctrl_c_sender, mut ctrl_c_receiver) = mpsc::channel::<()>(1);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");
        info!("Received Ctrl-C, exiting...");
        // Send a message through the channel to notify the main loop
        let _ = ctrl_c_sender.send(()).await;
    });

    let mut timestamp: u64 = 0;
    let mut first = true;
    let mut counter: Option<u64> = None;
    loop {
        let sleep = tokio::time::sleep(match first {
            true => {
                first = false;
                humantime::parse_duration("0s")?
            }
            false => args.backoff.into(),
        });
        select! {
            _ = ctrl_c_receiver.recv() => {
                info!("Received Ctrl-C, exiting...");
                break;
            }
            _ = sleep => {}
        }

        let response = client
            .get(args.url.clone())
            .query(&[("time", timestamp.to_string().as_str())])
            .header("Accept", "application/json")
            .send()
            .await;

        let entries = match response {
            Ok(res) if res.status() == StatusCode::OK => match res.json().await {
                Ok(serde_json::Value::Object(val)) if val.contains_key("entries") && val["entries"].is_array() => {
                    val["entries"].as_array().unwrap().to_vec()
                }
                Ok(val) => {
                    warn!("Parsed response doesn't comply with expected schema: {}", val);
                    continue;
                }
                Err(e) => {
                    error!("Parsing of response to json failed: {:?}", e);
                    continue;
                }
            },
            Ok(res) => {
                warn!(
                    "Unexpected status code {} received with message: {}",
                    res.status(),
                    res.text().await.unwrap()
                );
                continue;
            }
            Err(e) => {
                error!("Couldn't parse body bytes due to error: {:?}", e);
                continue;
            }
        };

        let mut correct_entries = vec![];

        for entry in &entries {
            match Entry::try_from(entry) {
                Ok(val) => correct_entries.push(val),
                Err(e) => {
                    warn!("{:?}", e);
                }
            }
        }

        correct_entries.sort_by(|a, b| match a.timestamp.cmp(&b.timestamp) {
            Ordering::Equal => a.counter.cmp(&b.counter),
            a => a,
        });

        let iter = match correct_entries
            .iter()
            .position(|elem| elem.counter == counter && elem.timestamp == timestamp)
        {
            Some(i) => correct_entries.iter().skip(i + 1),
            None => correct_entries.iter().skip(0),
        };

        for entry in iter {
            println!("{}", serde_json::to_string(&entry).unwrap());
            timestamp = entry.timestamp;
            counter = entry.counter;
        }
    }

    info!("Stopping on last timestamp: {}...", timestamp);
    Ok(())
}

fn init_logger(name: String) {
    formatted_builder()
        .format(move |buf, record| {
            let level = record.level();
            let time = buf.timestamp();

            writeln!(buf, "[{} {} {}] {}", time, level, name, record.args())
        })
        .filter(None, log::LevelFilter::Info)
        .init();
}

#[derive(Debug, Parser)]
struct Cli {
    #[clap(help = "Url of the target to be scraped", short, long)]
    url: Url,

    #[clap(
        help = "Name of the instance, if left out the name will be 'default'. Used for logging",
        short,
        long,
        default_value = "default"
    )]
    name: String,

    #[clap(help = "Sleep duration in between scrapes", short, long, default_value = "15s")]
    backoff: humantime::Duration,
}
