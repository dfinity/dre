use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::anyhow;
use clap::Parser;
use log::{error, info};
use pretty_env_logger::formatted_builder;
use reqwest::ClientBuilder;
use tokio::select;
use tokio_util::sync::CancellationToken;
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
        // If target cannot be reached within this timeout error.
        .connect_timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(anyhow!("Error while constructing the client: {:?}", e)),
    };

    let token = CancellationToken::new();
    let token_clone = token.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");
        info!("Sending Ctrl-C, exiting...");
        // Send a message through the channel to notify the main loop
        token_clone.cancel();
    });

    let mut cursor = fetch_cursor(&args.cursor_path)?;
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    let mut should_run = true;
    while should_run {
        let mut leftover_buffer = String::new();
        select! {
            biased;
            _ = token.cancelled() => {
                info!("Received Ctrl-C, exiting...");
                break;
            }
            tick = interval.tick() => {
                info!("Received tick: {:?}", tick);
            }
        }

        let range = format!("entries={}:0:", cursor);

        let response = client
            .get(args.url.clone())
            .header("Accept", "application/vnd.fdo.journal")
            .header("Range", range)
            .send()
            .await;

        if let Err(e) = response {
            error!("Couldn't parse body bytes due to error: {:?}", e);
            continue;
        };

        let mut response = response.unwrap();

        loop {
            let maybe_response = select! {
                biased;
                _ = token.cancelled() => {
                    info!("Received Ctrl-C while reading chunks Exiting...");
                    should_run = false;
                    break;
                },
                // If there were no new logs for 30 seconds, error
                maybe_response = tokio::time::timeout(Duration::from_secs(30), response.chunk()) => maybe_response,
            };

            let chunk = match maybe_response {
                Ok(maybe_chunk) => match maybe_chunk {
                    Ok(Some(chunk)) => chunk,
                    Ok(None) => {
                        info!("Exhausted the response");
                        break;
                    }
                    Err(e) => {
                        error!("Failed to fetch next chunk: {:?}", e);
                        break;
                    }
                },
                Err(e) => {
                    error!("Didn't receive a chunk within the expected timeframe. Error {:?}", e);
                    break;
                }
            };
            let chunk_str = String::from_utf8_lossy(&chunk);
            let combined = leftover_buffer.clone() + &chunk_str;

            let to_parse = if let Some(pos) = combined.rfind("\n\n") {
                // Splitting at the pos + two new lines to finish the entry
                let (to_parse, rest) = combined.split_at(pos + 2);

                leftover_buffer = rest.to_string();
                to_parse
            } else {
                leftover_buffer = combined;
                continue;
            };

            let entries = parse_journal_entries_new(to_parse.as_bytes());

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

                // If the struct is created ok, serialization should not
                // fail.
                let serialized = serde_json::to_string(&map).unwrap();
                println!("{}", serialized)
            }

            if let Some(entry) = entries.last() {
                let curr = entry.fields.iter().find(|(name, _)| name.as_str() == "__CURSOR");
                if let Some((_, JournalField::Utf8(val))) = curr {
                    cursor = val.to_string()
                }
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
