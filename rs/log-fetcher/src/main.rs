use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::anyhow;
use clap::Parser;
use log::{error, info, warn};
use pretty_env_logger::formatted_builder;
use reqwest::ClientBuilder;
use tokio::select;
use tokio::sync::mpsc;
use url::Url;

use crate::journald_parser::{parse_journal_entries_new, JournalField};

mod journald_parser;

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

    let mut cursor = fetch_cursor(&args.cursor_path)?;
    let mut should_sleep = false;
    loop {
        let sleep = tokio::time::sleep(Duration::from_secs(match should_sleep {
            true => {
                should_sleep = false;
                15
            }
            false => 0,
        }));
        select! {
            _ = ctrl_c_receiver.recv() => {
                info!("Received Ctrl-C, exiting...");
                break;
            }
            _ = sleep => {}
        }

        let range = format!(
            "entries={}:{}:{}",
            cursor,
            match cursor.is_empty() {
                false => 1,
                true => 0,
            },
            30
        );

        let response = client
            .get(args.url.clone())
            .header("Accept", "application/vnd.fdo.journal")
            .header("Range", range)
            .send()
            .await;

        let body = match response {
            Ok(res) => match res.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Couldn't parse body bytes due to error: {:?}", e);
                    should_sleep = true;
                    continue;
                }
            },
            Err(e) => {
                error!("Couldn't parse body bytes due to error: {:?}", e);
                should_sleep = true;
                continue;
            }
        };

        let entries = parse_journal_entries_new(&body);

        for entry in &entries {
            let map: BTreeMap<String, String> = entry
                .fields
                .iter()
                .map(|(name, val)| match val {
                    JournalField::Binary(value) | JournalField::Utf8(value) => (name.clone(), value.clone()),
                })
                .collect();

            if map["__CURSOR"] == cursor {
                continue;
            }

            let serialized = match serde_json::to_string(&map) {
                Ok(ser) => ser,
                Err(e) => {
                    warn!("Failed to serialize entry: {:?}, Error was: {:?}", entry, e);
                    should_sleep = true;
                    continue;
                }
            };
            println!("{}", serialized)
        }

        if let Some(entry) = entries.last() {
            let curr = entry.fields.iter().find(|(name, _)| name.as_str() == "__CURSOR");
            if let Some((_, JournalField::Utf8(val))) = curr {
                cursor = val.to_string()
            }
        }
    }

    info!("Writing cursor {}...", cursor);
    write_cursor(&args.cursor_path, cursor)
}

fn fetch_cursor(path: &PathBuf) -> Result<String, anyhow::Error> {
    let dir = path.parent().ok_or_else(|| anyhow!("Invalid path {}", path.display()))?;

    fs::create_dir_all(dir).map_err(|e| anyhow!("Error while creating directories: {:?}", e))?;

    if !path.exists() {
        File::create(path).map_err(|e| anyhow!("Error while creating file: {:?}", e))?;
    }

    let mut contents = String::new();
    File::open(path)
        .unwrap()
        .read_to_string(&mut contents)
        .map_err(|e| anyhow!("Error while reading file: {:?}", e))?;

    Ok(contents.trim().to_string())
}

fn write_cursor(path: &PathBuf, cursor: String) -> Result<(), anyhow::Error> {
    File::create(path)
        .unwrap()
        .write_all(cursor.as_bytes())
        .map_err(|e| anyhow!("Error while writing to file: {:?}", e))
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

    #[clap(help = "Cursor path for this target", short, long)]
    cursor_path: PathBuf,

    #[clap(
        help = "Name of the instance, if left out the name will be 'default'. Used for logging",
        short,
        long,
        default_value = "default"
    )]
    name: String,
}
