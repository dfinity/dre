use std::path::PathBuf;

use slog::Logger;

use crate::rollout_schedule::Index;

use self::{curl_fetcher::CurlFetcher, sparse_checkout_fetcher::SparseCheckoutFetcher};

pub mod curl_fetcher;
pub mod sparse_checkout_fetcher;

pub trait RolloutScheduleFetcher {
    async fn fetch(&self) -> anyhow::Result<Index>;
}

pub enum RolloutScheduleFetcherImplementation {
    Curl(CurlFetcher),
    Git(SparseCheckoutFetcher),
}

pub async fn resolve(
    mode: String,
    logger: Logger,
    path: PathBuf,
    url: String,
    release_index: String,
) -> anyhow::Result<RolloutScheduleFetcherImplementation> {
    match mode.to_lowercase().as_str() {
        "git" => SparseCheckoutFetcher::new(logger, path, url, release_index)
            .await
            .map(RolloutScheduleFetcherImplementation::Git),
        "curl" => CurlFetcher::new(logger, url).map(RolloutScheduleFetcherImplementation::Curl),
        _ => Err(anyhow::anyhow!("Couldn't construct index fetcher for mode '{}'", mode)),
    }
}

impl RolloutScheduleFetcherImplementation {
    pub async fn fetch(&self) -> anyhow::Result<Index> {
        match self {
            RolloutScheduleFetcherImplementation::Curl(implementation) => implementation.fetch().await,
            RolloutScheduleFetcherImplementation::Git(implementation) => implementation.fetch().await,
        }
    }
}
