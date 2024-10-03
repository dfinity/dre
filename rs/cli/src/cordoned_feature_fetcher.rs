use std::{path::PathBuf, time::Duration};

use decentralization::network::NodeFeaturePair;
use futures::future::BoxFuture;
use ic_management_types::NodeFeature;
use itertools::Itertools;
use log::warn;
use mockall::automock;
use reqwest::{Client, ClientBuilder};
use strum::VariantNames;

#[automock]
pub trait CordonedFeatureFetcher: Sync + Send {
    fn fetch(&self) -> BoxFuture<'_, anyhow::Result<Vec<NodeFeaturePair>>>;
}

pub struct CordonedFeatureFetcherImpl {
    client: Client,
    fallback_file: Option<PathBuf>,
    offline: bool,
}

const CORDONED_FEATURES_FILE_URL: &str = "https://raw.githubusercontent.com/dfinity/dre/refs/heads/main/cordoned_features.yaml";

impl CordonedFeatureFetcherImpl {
    pub fn new(offline: bool, fallback_file: Option<PathBuf>) -> anyhow::Result<Self> {
        let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build()?;

        Ok(Self {
            client,
            fallback_file,
            offline,
        })
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

        self.parse(&bytes)
    }

    fn fetch_from_file(&self) -> anyhow::Result<Vec<NodeFeaturePair>> {
        let contents = std::fs::read(self.fallback_file.as_ref().unwrap())?;

        self.parse(&contents)
    }

    // Write tests for this
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
            match (self.offline, self.fallback_file.is_some()) {
                (true, true) => self.fetch_from_file(),
                (true, false) => Err(anyhow::anyhow!("Cannot fetch cordoned features offline without a fallback file")),
                (false, true) => match self.fetch_from_git().await {
                    Ok(from_git) => Ok(from_git),
                    Err(e_from_git) => {
                        warn!("Failed to fetch cordoned features from git: {:?}", e_from_git);
                        warn!("Falling back to fetching from file");
                        match self.fetch_from_file() {
                            Ok(from_file) => Ok(from_file),
                            Err(e_from_file) => Err(anyhow::anyhow!(
                                "Failed to fetch cordoned features both from file and from git.\nError from git: {:?}\nError from file: {:?}",
                                e_from_git,
                                e_from_file
                            )),
                        }
                    }
                },
                (false, false) => self.fetch_from_git().await,
            }
        })
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

        let fetcher = CordonedFeatureFetcherImpl::new(true, None).unwrap();

        let parsed = fetcher.parse(contents).unwrap();

        assert_eq!(parsed.len(), 6)
    }

    #[test]
    fn valid_empty_file() {
        let contents = br#"
features:"#;

        let fetcher = CordonedFeatureFetcherImpl::new(true, None).unwrap();

        let maybe_parsed = fetcher.parse(contents);
        assert!(maybe_parsed.is_ok());
        let parsed = maybe_parsed.unwrap();

        assert_eq!(parsed.len(), 0)
    }
}
