use clap::Parser;
use reqwest::Client;
use slog::{debug, Logger};

use super::RolloutScheduleFetcher;

#[derive(Parser, Clone, Debug)]
pub struct CurlFetcherConfig {
    #[clap(
        long = "url",
        default_value = "https://raw.githubusercontent.com/dfinity/dre/main/release-index.yaml",
        help = r#"
The url of the raw file in github

"#
    )]
    pub url: String,
}

#[derive(Clone)]
pub struct CurlFetcher {
    logger: Logger,
    client: Client,
    url: String,
}

impl CurlFetcher {
    pub fn new(logger: Logger, url: String) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::new(),
            logger,
            url,
        })
    }
}

impl RolloutScheduleFetcher for CurlFetcher {
    async fn fetch(&self) -> anyhow::Result<crate::calculation::Index> {
        debug!(self.logger, "Fetching rollout index");

        let response = self
            .client
            .get(&self.url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Error fetching rollout schedule: {:?}", e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Error converting body to bytes: {:?}", e))?;

        serde_yaml::from_slice(bytes.to_vec().as_slice()).map_err(|e| anyhow::anyhow!("Couldn't parse release index: {:?}", e))
    }
}
