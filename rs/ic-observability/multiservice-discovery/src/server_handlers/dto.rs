use base64::{engine::general_purpose as b64, Engine as _};
use ic_crypto_utils_threshold_sig_der::parse_threshold_sig_key_from_der;
use ic_registry_client::client::ThresholdSigPublicKey;
use service_discovery::registry_sync::nns_reachable;

use serde::{Deserialize, Serialize};
use slog::Logger;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

use crate::definition::Definition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionDto {
    pub nns_urls: Vec<Url>,
    pub name: String,
    pub public_key: Option<String>,
}

#[derive(Debug)]
pub(crate) enum BadDtoError {
    InvalidPublicKey(String, std::io::Error),
    AlreadyExists(String),
    NNSUnreachable(String),
}

impl Error for BadDtoError {}

impl Display for BadDtoError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::InvalidPublicKey(name, e) => {
                write!(f, "public key of definition {} is invalid: {}", name, e)
            }
            Self::AlreadyExists(name) => write!(f, "definition {} already exists", name),
            Self::NNSUnreachable(name) => {
                write!(f, "cannot reach any of the NNS nodes specified in definition {}", name)
            }
        }
    }
}

impl DefinitionDto {
    pub(crate) async fn try_into_definition(
        self,
        log: Logger,
        registry_path: PathBuf,
        poll_interval: Duration,
        registry_query_timeout: Duration,
    ) -> Result<Definition, BadDtoError> {
        if !nns_reachable(self.nns_urls.clone()).await {
            return Err(BadDtoError::NNSUnreachable(self.name));
        }

        let (stop_signal_sender, stop_signal_rcv) = crossbeam::channel::bounded::<()>(0);
        Ok(Definition::new(
            self.nns_urls.clone(),
            registry_path,
            self.name.clone(),
            log,
            self.decode_public_key()?,
            poll_interval,
            stop_signal_rcv,
            registry_query_timeout,
            stop_signal_sender,
        ))
    }

    fn decode_public_key(self) -> Result<Option<ThresholdSigPublicKey>, BadDtoError> {
        match self.public_key {
            Some(pk) => {
                let decoded = b64::STANDARD.decode(pk).unwrap();

                match parse_threshold_sig_key_from_der(&decoded) {
                    Ok(key) => Ok(Some(key)),
                    Err(e) => Err(BadDtoError::InvalidPublicKey(self.name, e)),
                }
            }
            None => Ok(None),
        }
    }
}

impl From<&Definition> for DefinitionDto {
    fn from(value: &Definition) -> Self {
        Self {
            name: value.name.clone(),
            nns_urls: value.nns_urls.clone(),
            public_key: value.public_key.map(|pk| b64::STANDARD.encode(pk.into_bytes())),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundaryNodeDto {
    pub name: String,
    pub ic_name: String,
    pub custom_labels: BTreeMap<String, String>,
    pub targets: BTreeSet<SocketAddr>,
    pub job_type: String,
}
