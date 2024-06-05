use slog::Logger;

use crate::{calculation::Index, Commands};

use self::{
    curl_fetcher::{CurlFetcher, CurlFetcherConfig},
    sparse_checkout_fetcher::{SparseCheckoutFetcher, SparseCheckoutFetcherConfig},
};

pub mod curl_fetcher;
pub mod sparse_checkout_fetcher;

pub trait RolloutScheduleFetcher {
    async fn fetch(&self) -> anyhow::Result<Index>;
}

pub enum RolloutScheduleFetcherImplementation {
    Curl(CurlFetcher),
    Git(SparseCheckoutFetcher),
}

pub async fn resolve(subcmd: Commands, logger: Logger) -> anyhow::Result<RolloutScheduleFetcherImplementation> {
    match subcmd {
        Commands::Git(SparseCheckoutFetcherConfig {
            repo_url,
            release_index,
            repo_path,
        }) => SparseCheckoutFetcher::new(logger, repo_path, repo_url, release_index)
            .await
            .map(RolloutScheduleFetcherImplementation::Git),
        Commands::Curl(CurlFetcherConfig { url }) => CurlFetcher::new(logger, url).map(RolloutScheduleFetcherImplementation::Curl),
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
