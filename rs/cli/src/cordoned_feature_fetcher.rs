use std::{path::PathBuf, time::Duration};

use decentralization::network::NodeFeaturePair;
use futures::future::BoxFuture;
use ic_management_types::NodeFeature;
use itertools::Itertools;
use log::{info, warn};
use mockall::automock;
use reqwest::{Client, ClientBuilder};
use strum::VariantNames;

#[automock]
pub trait CordonedFeatureFetcher: Sync + Send {
    fn fetch(&self) -> BoxFuture<'_, anyhow::Result<Vec<NodeFeaturePair>>>;

    #[cfg(test)]
    fn parse_outer(&self, contents: &[u8]) -> anyhow::Result<Vec<NodeFeaturePair>>;
}

pub struct CordonedFeatureFetcherImpl {
    client: Client,
    // Represents a local store which will
    // be overwritten with every successful
    // fetch from github. If github is
    // unreachable, this cache will be used
    local_copy: PathBuf,
    offline: bool,
}

const CORDONED_FEATURES_FILE_URL: &str = "https://raw.githubusercontent.com/dfinity/dre/refs/heads/main/cordoned_features.yaml";

impl CordonedFeatureFetcherImpl {
    pub fn new(local_copy: PathBuf, offline: bool) -> anyhow::Result<Self> {
        let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build()?;

        Ok(Self { client, local_copy, offline })
    }

    async fn fetch_from_git(&self) -> anyhow::Result<Vec<NodeFeaturePair>> {
        let bytes = self
            .client
            .get(CORDONED_FEATURES_FILE_URL)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        if let Err(e) = std::fs::write(&self.local_copy, &bytes) {
            warn!(
                "Failed to update cordoned features cache on path `{}` due to: {:?}",
                self.local_copy.display(),
                e
            );
            warn!("This is not critical since the cordoned features are fetched from github");
        }

        self.parse(&bytes)
    }

    fn fetch_from_file(&self) -> anyhow::Result<Vec<NodeFeaturePair>> {
        let contents = std::fs::read(&self.local_copy)?;

        self.parse(&contents)
    }

    fn parse(&self, contents: &[u8]) -> anyhow::Result<Vec<NodeFeaturePair>> {
        let valid_yaml = serde_yaml::from_slice::<serde_yaml::Value>(contents)?;

        let features = match valid_yaml.get("features") {
            Some(serde_yaml::Value::Sequence(features)) => features.clone(),
            Some(serde_yaml::Value::Null) => vec![],
            n => anyhow::bail!(
                "Failed to parse contents. Expected to have top-level key `features` with an array of node features. Got: \n{:?}",
                n
            ),
        };

        let mut valid_features = vec![];
        for feature in features {
            valid_features.push(NodeFeaturePair {
                feature: feature
                    .get("feature")
                    .map(|value| {
                        serde_yaml::from_value(value.clone()).map_err(|_| {
                            anyhow::anyhow!(
                                "Failed to parse feature `{}`. Expected one of: [{}]",
                                serde_yaml::to_string(value).unwrap(),
                                NodeFeature::VARIANTS.iter().join(",")
                            )
                        })
                    })
                    .ok_or(anyhow::anyhow!("Expected `feature` key to be present. Got: \n{:?}", feature))??,
                value: feature
                    .get("value")
                    .map(|value| {
                        value
                            .as_str()
                            .ok_or(anyhow::anyhow!(
                                "Failed to parse value `{}`. Expected string",
                                serde_yaml::to_string(value).unwrap(),
                            ))
                            .map(|s| s.to_string())
                    })
                    .ok_or(anyhow::anyhow!("Expected `value` key to be present. Got: \n{:?}", feature))??,
            });
        }

        Ok(valid_features)
    }
}

impl CordonedFeatureFetcher for CordonedFeatureFetcherImpl {
    fn fetch(&self) -> BoxFuture<'_, anyhow::Result<Vec<NodeFeaturePair>>> {
        Box::pin(async {
            if self.offline {
                // Offline mode specified, use cache
                info!("In offline mode, cordoned features will be loaded from cache");
                info!("Cache path for cordoned features: {}", self.local_copy.display());
                self.fetch_from_file()
            } else {
                self.fetch_from_git().await
            }
        })
    }

    #[cfg(test)]
    fn parse_outer(&self, contents: &[u8]) -> anyhow::Result<Vec<NodeFeaturePair>> {
        self.parse(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_parsing() {
        let contents = br#"
features:
    - feature: data_center
      value: mu1
    - feature: node_provider
      value: some-np
    - feature: data_center_owner
      value: some-dco
    - feature: area
      value: some-area
    - feature: area
      value: another-area
    - feature: country
      value: some-country
      "#;

        let fetcher = CordonedFeatureFetcherImpl::new(PathBuf::new(), true).unwrap();

        let parsed = fetcher.parse(contents).unwrap();

        assert_eq!(parsed.len(), 6)
    }

    #[test]
    fn valid_empty_file() {
        let contents = br#"
features:"#;

        let fetcher = CordonedFeatureFetcherImpl::new(PathBuf::new(), true).unwrap();

        let maybe_parsed = fetcher.parse(contents);
        assert!(maybe_parsed.is_ok());
        let parsed = maybe_parsed.unwrap();

        assert_eq!(parsed.len(), 0)
    }
}
