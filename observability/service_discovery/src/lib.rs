//! IcServiceDiscovery
//!
//! The target IP addresses and labels are the same for all endpoints, except
//! that the host IPv6 addresses for `host_node_exporter` are inferred from the
//! the one used for `replica` according to a fixed address schema. The ports
//! are set as per job_types.rs:
//!
#![allow(clippy::await_holding_lock, clippy::result_large_err)]
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet, HashMap},
    convert::TryFrom,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Result;
use ic_interfaces_registry::{RegistryClient, RegistryClientResult};
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_registry_client_helpers::{
    node::NodeRegistry,
    node_operator::NodeOperatorRegistry,
    subnet::{SubnetListRegistry, SubnetTransportRegistry},
};
use ic_registry_local_registry::{LocalRegistry, LocalRegistryError};
use ic_types::{registry::RegistryClientError, NodeId, PrincipalId, RegistryVersion, SubnetId};
use job_types::{JobType, NodeOS};
use regex::Regex;
use serde::{Deserialize, Serialize};
use slog::{warn, Logger};
use thiserror::Error;

pub mod file_sd;
pub mod job_types;
pub mod jobs;
pub mod mainnet_registry;
pub mod metrics;
pub mod poll_loop;
pub mod registry_sync;
pub mod rest_api;
pub mod service_discovery_record;

/// Provide service discovery for a set of Internet Computers.
pub trait IcServiceDiscovery: Send + Sync {
    /// Returns a list of [TargetGroup] containing all targets
    /// for the given `job_name`.
    ///
    /// The job name must be one of `replica`, `orchestrator`, `node_exporter`,
    /// or `host_node_exporter`.
    fn get_target_groups(
        &self,
        job_name: JobType,
    ) -> Result<BTreeSet<TargetGroup>, IcServiceDiscoveryError>;
}

/// A [TargetGroup] associates a set of scrape targets with
/// a set of labels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TargetGroup {
    pub node_id: NodeId,
    pub ic_name: String,
    /// A set of targets to be scraped that share the same labels.
    pub targets: BTreeSet<SocketAddr>,
    /// A set of labels that are associated with the targets listed in
    /// `socket_addr`.
    pub subnet_id: Option<SubnetId>,

    pub dc_id: String,
    pub operator_id: PrincipalId,
    pub node_provider_id: PrincipalId,
}

impl TargetGroup {
    pub fn get_ip_as_str(&self) -> Option<String> {
        let regex = Regex::new(r"\[.*\]").unwrap();

        match self.targets.iter().next() {
            Some(addr) => match regex.find(&addr.to_string()) {
                Some(mat) => {
                    let binding = mat.as_str().to_string();
                    let mut chars = binding.chars();
                    chars.next().unwrap();
                    chars.next_back().unwrap();
                    Some(chars.as_str().to_string())
                }
                None => None,
            },
            None => None,
        }
    }
}

/// Exposes service discovery data for a set of Internet Computers. Manages a
/// directory containing a registry local store for every Internet Computer
/// whose discovery data is exposed. Each local store is updated on a regular
/// basis.
#[derive(Clone)]
pub struct IcServiceDiscoveryImpl {
    /// The directory containing one local store per observed Internet Computer.
    ic_scraping_targets_dir: PathBuf,
    /// The http request timeout that is used when updating the local stores.
    registry_query_timeout: Duration,
    /// An in-memory representation of the registries that is updated when
    /// calling `load_new_scraping_targets`.
    registries: Arc<RwLock<BTreeMap<String, LocalRegistry>>>,

    jobs: HashMap<JobType, u16>,
}

impl IcServiceDiscoveryImpl {
    /// Create new instance of [IcServiceDiscoveryImpl]. The
    /// `ic_scraping_targets_dir` must point to a directory that contains the
    /// local stores of the Internet Computer instances to be scraped.
    pub fn new<P: AsRef<Path>>(
        log: Logger,
        ic_scraping_targets_dir: P,
        registry_query_timeout: Duration,
        jobs: HashMap<JobType, u16>,
    ) -> Result<Self, IcServiceDiscoveryError> {
        let ic_scraping_targets_dir = PathBuf::from(ic_scraping_targets_dir.as_ref());
        if !ic_scraping_targets_dir.is_dir() {
            return Err(IcServiceDiscoveryError::NotADirectory {
                path: ic_scraping_targets_dir,
            });
        }
        let registries = Arc::new(RwLock::new(Default::default()));
        let self_ = Self {
            ic_scraping_targets_dir,
            registry_query_timeout,
            registries,
            jobs,
        };
        self_.load_new_ics(log)?;
        Ok(self_)
    }

    /// Update each scraping target by fetching update for the respective
    /// registry.
    ///
    /// If all updates succeed, returns `Ok(())`. Otherwise an error is returned
    /// containing all failed update attempts.
    pub async fn update_registries(&self) -> Result<(), IcServiceDiscoveryError> {
        let cache = self.registries.read().unwrap();
        let mut failures = vec![];
        for (ic_name, registry) in cache.iter() {
            if let Err(e) = registry.sync_with_nns().await {
                failures.push((ic_name.to_string(), e));
            }
        }
        if !failures.is_empty() {
            return Err(IcServiceDiscoveryError::SyncWithNnsFailed { failures });
        }
        Ok(())
    }

    /// Synchronizes the in-memory cache with the state on disk.
    ///
    /// # Known Limitations
    ///
    /// * If a directory is replaced with another directory with the same name,
    /// the content of the corresponding cached registry is not updated.
    ///
    /// * The set of scraped ICs currently strictly grows throughout the
    /// lifetime of a service instance. I.e., removing an IC as a scrape target
    /// requires rebooting the service.
    pub fn load_new_ics(&self, log: Logger) -> Result<(), IcServiceDiscoveryError> {
        let paths = std::fs::read_dir(&self.ic_scraping_targets_dir)?;
        let mut registries_lock_guard = self.registries.write().unwrap();
        for path in paths {
            let path = path?;
            if !path.path().is_dir() {
                // If it's not a directory, it cannot be a local store.
                continue;
            }
            let ic_name = path.file_name().to_str().unwrap().to_string();
            if let Entry::Vacant(e) = registries_lock_guard.entry(ic_name) {
                // if the path does not contain a correct cache, and
                // if it cannot fetch it, the SD will not start.
                // This is a temporary fix which will solve the issue by ignoring
                // the directories it cannot create a registry from.
                // The downside is that it will try to do it everytime the
                // registry is being polled
                match LocalRegistry::new(path.path(), self.registry_query_timeout) {
                    Ok(ic_registry) => {
                        e.insert(ic_registry);
                    }
                    Err(e) => warn!(
                        log,
                        "Could not load create a registry from {}: {}, skipping.",
                        path.path().canonicalize().unwrap().to_str().unwrap(),
                        e
                    ),
                };
            }
        }
        Ok(())
    }

    fn get_targets(
        reg_client: &dyn RegistryClient,
        ic_name: &str,
    ) -> Result<BTreeSet<TargetGroup>, IcServiceDiscoveryError> {
        let latest_version = reg_client.get_latest_version();

        let mut unassigned_node_ids = reg_client
            .get_node_ids(latest_version)?
            .into_iter()
            .collect::<BTreeSet<_>>();

        let mut node_targets = BTreeSet::new();
        let subnet_ids = reg_client
            .get_subnet_ids(latest_version)
            .map_registry_err(latest_version, "get_subnet_ids")?;

        for subnet_id in subnet_ids {
            let t_infos = reg_client
                .get_subnet_node_records(subnet_id, latest_version)
                .map_registry_err(latest_version, "get_subnet_transport_info")?;

            for (node_id, node_record) in t_infos {
                Self::add_node_to_node_targets(
                    node_id,
                    latest_version,
                    node_record,
                    reg_client,
                    &mut node_targets,
                    Some(subnet_id),
                    ic_name,
                )?;
                unassigned_node_ids.remove(&node_id);
            }
        }

        // collect information about unassigned nodes
        for node_id in unassigned_node_ids {
            let node_record = reg_client
                .get_node_record(node_id, latest_version)
                .map_registry_err(latest_version, "get_transport_info")?;

            Self::add_node_to_node_targets(
                node_id,
                latest_version,
                node_record,
                reg_client,
                &mut node_targets,
                None,
                ic_name,
            )?;
        }
        Ok(node_targets)
    }

    fn add_node_to_node_targets(
        node_id: NodeId,
        latest_version: RegistryVersion,
        node_record: NodeRecord,
        reg_client: &dyn RegistryClient,
        node_targets: &mut BTreeSet<TargetGroup>,
        subnet_id: Option<SubnetId>,
        ic_name: &str,
    ) -> Result<(), IcServiceDiscoveryError> {
        let socket_addr =
            Self::node_record_to_target_addr(node_id, latest_version, node_record.clone())?;

        let operator_id = PrincipalId::try_from(node_record.node_operator_id).unwrap_or_default();

        let node_operator = reg_client
            .get_node_operator_record(operator_id, latest_version)
            .unwrap_or_default()
            .unwrap_or_default();

        (*node_targets).insert(TargetGroup {
            targets: vec![socket_addr].into_iter().collect(),
            subnet_id,
            node_id,
            ic_name: ic_name.into(),
            dc_id: node_operator.dc_id,
            operator_id,
            node_provider_id: PrincipalId::try_from(node_operator.node_provider_principal_id)
                .unwrap_or_default(),
        });

        Ok(())
    }

    fn node_record_to_target_addr(
        node_id: NodeId,
        registry_version: RegistryVersion,
        node_record: NodeRecord,
    ) -> Result<SocketAddr, IcServiceDiscoveryError> {
        use IcServiceDiscoveryError::*;
        let addr = SocketAddr::new(node_record.http.unwrap().ip_addr.parse().unwrap(), 9090);
        // Seen bogus registry entries where the connection endpoint exists
        // but is 0.0.0.0
        if addr.ip().is_unspecified() {
            return Err(ConnectionEndpointIsAllBalls {
                node_id,
                registry_version,
            });
        }

        Ok(addr)
    }
}

impl IcServiceDiscovery for IcServiceDiscoveryImpl {
    fn get_target_groups(
        &self,
        job: JobType,
    ) -> Result<BTreeSet<TargetGroup>, IcServiceDiscoveryError> {
        let mut mapping: Option<Box<dyn Fn(SocketAddr) -> Option<SocketAddr>>> = None;

        if job == JobType::NodeExporter(NodeOS::Host) {
            mapping = Some(Box::new(|sockaddr: SocketAddr| {
                guest_to_host_address((set_port(job.port()))(sockaddr))
            }));
        } else if job == JobType::MetricsProxy {
            mapping = Some(Box::new(|sockaddr: SocketAddr| {
                guest_to_host_address((set_port(job.port()))(sockaddr))
            }));
        }

        for (listed_job, port) in &self.jobs {
            if mapping.is_some() {
                break;
            }

            if *listed_job == job {
                mapping = Some(some_after(set_port(*port)));
                break;
            }
        }

        if mapping.is_none() {
            return Err(IcServiceDiscoveryError::JobNameNotFound {
                job_name: job.to_string(),
            });
        }

        let registries_lock_guard = self.registries.read().unwrap();
        let target_list = registries_lock_guard.iter().try_fold(
            BTreeSet::new(),
            |mut a, (ic_name, registry)| {
                a.append(&mut Self::get_targets(registry, ic_name)?);
                Ok::<_, IcServiceDiscoveryError>(a)
            },
        )?;

        Ok(target_list
            .into_iter()
            .filter_map(|target_group| {
                // replica targets are only exposed if they are assigned to a
                // subnet (i.e. if the subnet id is set)
                if job != JobType::Replica || target_group.subnet_id.is_some() {
                    let targets: BTreeSet<_> = target_group
                        .targets
                        .into_iter()
                        .filter_map(&mapping.as_ref().unwrap())
                        .collect();
                    if !targets.is_empty() {
                        return Some(TargetGroup {
                            targets,
                            ..target_group
                        });
                    }
                }
                None
            })
            .collect::<BTreeSet<_>>())
    }
}

fn set_port(port: u16) -> Box<dyn Fn(SocketAddr) -> SocketAddr> {
    Box::new(move |mut sockaddr: SocketAddr| {
        sockaddr.set_port(port);
        sockaddr
    })
}

/// Take a function f and return `Some . f`
fn some_after(
    f: Box<dyn Fn(SocketAddr) -> SocketAddr>,
) -> Box<dyn Fn(SocketAddr) -> Option<SocketAddr>> {
    Box::new(move |s| Some(f(s)))
}

/// By convention, the first two bytes of the host-part of the replica's IP
/// address are 0x6801. The corresponding segment for the host is 0x6800.
///
/// (The MAC starts with 0x6a00. The 7'th bit of the first byte is flipped. See
/// https://en.wikipedia.org/wiki/MAC_address)
pub fn guest_to_host_address(sockaddr: SocketAddr) -> Option<SocketAddr> {
    match sockaddr.ip() {
        IpAddr::V6(a) if a.segments()[4] == 0x6801 => {
            let s = a.segments();
            let new_addr = Ipv6Addr::new(s[0], s[1], s[2], s[3], 0x6800, s[5], s[6], s[7]);
            let ip = IpAddr::V6(new_addr);
            Some(SocketAddr::new(ip, sockaddr.port()))
        }
        _ip => None,
    }
}

trait MapRegistryClientErr<T> {
    fn map_registry_err(
        self,
        version: RegistryVersion,
        context: &str,
    ) -> Result<T, IcServiceDiscoveryError>;
}

impl<T> MapRegistryClientErr<T> for RegistryClientResult<T> {
    fn map_registry_err(
        self,
        version: RegistryVersion,
        context: &str,
    ) -> Result<T, IcServiceDiscoveryError> {
        use IcServiceDiscoveryError::*;
        match self {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Err(MissingRegistryValue {
                version,
                context: context.into(),
            }),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Error, Debug)]
pub enum IcServiceDiscoveryError {
    #[error("Local registry error.")]
    LocalRegistryError {
        #[from]
        source: LocalRegistryError,
    },
    #[error("Provided path `{path}` is not a directory.")]
    NotADirectory { path: PathBuf },
    #[error("IoError")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("Missing registry value. context: {context} version: {version}")]
    MissingRegistryValue {
        version: RegistryVersion,
        context: String,
    },
    #[error("RegistryClientError")]
    RegistryClient {
        #[from]
        source: RegistryClientError,
    },
    #[error("failed to get endpoint for node {node_id}")]
    NoConnectionEndpoint {
        node_id: NodeId,
        registry_version: RegistryVersion,
    },
    #[error("metrics connection endpoint for {node_id} has 0.0.0.0 addr at {registry_version}")]
    ConnectionEndpointIsAllBalls {
        node_id: NodeId,
        registry_version: RegistryVersion,
    },
    #[error("updating the local store from the NNS failed")]
    SyncWithNnsFailed {
        failures: Vec<(String, LocalRegistryError)>,
    },
    #[error("job name not found: {job_name}")]
    JobNameNotFound { job_name: String },

    #[error("couldn't find from {information} from local registry")]
    NotFoundInRegistry { information: String },
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use slog::o;
    use tempfile::TempDir;

    use super::*;
    use crate::mainnet_registry::{create_local_store_from_changelog, get_mainnet_delta_6d_c1};
    use itertools::Itertools; // for the function [unique_by]

    const QUERY_TIMEOUT: Duration = Duration::from_secs(5);
    #[test]
    fn can_get_nns_targets_for() {
        let mainnet_prefix = "tdb26";
        let tempdir = TempDir::new().unwrap();
        let ic_dir = PathBuf::from(tempdir.path()).join("mainnet");
        let _store = create_local_store_from_changelog(ic_dir, get_mainnet_delta_6d_c1());
        let mut jobs: HashMap<JobType, u16> = HashMap::new();
        jobs.insert(JobType::Replica, 9090);

        let log = slog::Logger::root(slog::Discard, o!());

        let ic_scraper =
            IcServiceDiscoveryImpl::new(log.clone(), tempdir.path(), QUERY_TIMEOUT, jobs).unwrap();
        ic_scraper.load_new_ics(log.clone()).unwrap();
        let target_groups = ic_scraper.get_target_groups(JobType::Replica).unwrap();

        let nns_targets: HashSet<_> = target_groups
            .iter()
            .filter(|g| {
                g.subnet_id
                    .map(|s| s.to_string().starts_with(mainnet_prefix))
                    .unwrap_or(false)
                    && g.ic_name == "mainnet"
                    && g.targets.len() == 1
            })
            .unique_by(|g| g.node_id)
            .unique_by(|g| &g.targets)
            .map(|g| g.targets.iter().next().unwrap().to_string())
            .collect();

        let expected_nns_targets: HashSet<_> = [
            "[2001:920:401a:1706:5000:87ff:fe11:a9a0]:9090",
            "[2001:920:401a:1708:5000:4fff:fe92:48f1]:9090",
            "[2001:920:401a:1708:5000:5fff:fec1:9ddb]:9090",
            "[2001:920:401a:1710:5000:28ff:fe36:512b]:9090",
            "[2001:920:401a:1710:5000:d7ff:fe6f:fde7]:9090",
            "[2401:3f00:1000:22:5000:c3ff:fe44:36f4]:9090",
            "[2401:3f00:1000:23:5000:80ff:fe84:91ad]:9090",
            "[2401:3f00:1000:24:5000:deff:fed6:1d7]:9090",
            "[2600:2c01:21:0:5000:27ff:fe23:4839]:9090",
            "[2600:3000:6100:200:5000:c4ff:fe43:3d8a]:9090",
            "[2600:3004:1200:1200:5000:59ff:fe54:4c4b]:9090",
            "[2600:3006:1400:1500:5000:95ff:fe94:c948]:9090",
            "[2600:c02:b002:15:5000:22ff:fe65:e916]:9090",
            "[2600:c02:b002:15:5000:53ff:fef7:d3c0]:9090",
            "[2600:c02:b002:15:5000:ceff:fecc:d5cd]:9090",
            "[2604:3fc0:2001:0:5000:b0ff:fe7b:ff55]:9090",
            "[2604:3fc0:3002:0:5000:acff:fe31:12e8]:9090",
            "[2604:7e00:50:0:5000:20ff:fea7:efee]:9090",
            "[2607:f1d0:10:1:5000:a7ff:fe91:44e]:9090",
            "[2607:f758:1220:0:5000:12ff:fe0c:8a57]:9090",
            "[2607:f758:1220:0:5000:3aff:fe16:7aec]:9090",
            "[2607:f758:1220:0:5000:bfff:feb9:6794]:9090",
            "[2607:f758:c300:0:5000:3eff:fe6d:af08]:9090",
            "[2607:f758:c300:0:5000:72ff:fe35:3797]:9090",
            "[2607:f758:c300:0:5000:8eff:fe8b:d68]:9090",
            "[2a00:fa0:3:0:5000:5aff:fe89:b5fc]:9090",
            "[2a00:fa0:3:0:5000:68ff:fece:922e]:9090",
            "[2a00:fb01:400:100:5000:5bff:fe6b:75c6]:9090",
            "[2a00:fb01:400:100:5000:61ff:fe2c:14ac]:9090",
            "[2a00:fb01:400:100:5000:ceff:fea2:bb0]:9090",
            "[2a01:138:900a:0:5000:2aff:fef4:c47e]:9090",
            "[2a01:138:900a:0:5000:5aff:fece:cf05]:9090",
            "[2a04:9dc0:0:108:5000:6bff:fe08:5f57]:9090",
            "[2a04:9dc0:0:108:5000:7cff:fece:97d]:9090",
            "[2a04:9dc0:0:108:5000:96ff:fe4a:be10]:9090",
            "[2a0f:cd00:2:1:5000:3fff:fe36:cab8]:9090",
            "[2a0f:cd00:2:1:5000:87ff:fe58:ceba]:9090",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();

        assert_eq!(nns_targets.len(), 37);
        assert_eq!(nns_targets, expected_nns_targets);

        let subnet_count = target_groups.iter().unique_by(|g| g.subnet_id).count();
        // there are 29 subnets at version 0x6dc1, and unassigned nodes belong to `None`
        assert_eq!(subnet_count, 29);
    }
}
