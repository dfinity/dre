use std::{collections::BTreeMap, fs, path::PathBuf, str::FromStr, time::Duration};

use ic_base_types::RegistryVersion;
use ic_registry_keys::{
    DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};

use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord,
};

use anyhow::Error;
use ic_interfaces_registry::{RegistryClient, RegistryValue};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;
use service_discovery::registry_sync::sync_local_registry;
use slog::{o, Drain, Logger};
use uuid::Uuid;

pub async fn dump_registry(path: &Option<PathBuf>, version: &i64, network: &Network) -> Result<(), Error> {
    let (path, should_dispose) = match path {
        Some(val) => (val.clone(), false),
        None => {
            let uuid = Uuid::new_v4();
            let binding = format!("~/tmp/{}", uuid);
            let local_temp = shellexpand::tilde(&binding);
            (
                PathBuf::from_str(&local_temp).map_err(|e| anyhow::anyhow!("Error converting to path: {:?}", e))?,
                true,
            )
        }
    };

    let logger = make_logger();
    sync_local_registry(logger.clone(), path.clone(), vec![network.get_url()], false, None).await;

    // nodes,dc,node_operators,node_providers,
    let client = LocalRegistry::new(path.clone(), Duration::from_secs(10))
        .map_err(|e| anyhow::anyhow!("Couldn't create local registry instance: {:?}", e))?;

    // determine desired version
    let version = {
        if *version >= 0 {
            *version as u64
        } else {
            client.get_latest_version().get()
        }
    };
    let version = RegistryVersion::new(version);

    let nodes = client
        .get_family_entries_versioned::<NodeRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let nodes = serde_json::to_string(&nodes).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    let subnets = client
        .get_family_entries_versioned::<SubnetRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let subnets = serde_json::to_string(&subnets).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    let dcs = client
        .get_family_entries_versioned::<DataCenterRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let dcs = serde_json::to_string(&dcs).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    let node_operators = client
        .get_family_entries_versioned::<NodeOperatorRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let node_operators =
        serde_json::to_string(&node_operators).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    println!(
        "{{ \"nodes\": {}, \"subnets\": {}, \"dcs\": {}, \"node_operators\": {} }}",
        nodes, subnets, dcs, node_operators
    );

    if should_dispose {
        fs::remove_dir_all(path).map_err(|e| anyhow::anyhow!("Error removing created dir: {:?}", e))?
    }
    Ok(())
}

fn make_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).chan_size(8192).build();
    Logger::root(drain.fuse(), o!())
}

trait RegistryEntry: RegistryValue {
    const KEY_PREFIX: &'static str;
}

impl RegistryEntry for DataCenterRecord {
    const KEY_PREFIX: &'static str = DATA_CENTER_KEY_PREFIX;
}

impl RegistryEntry for NodeOperatorRecord {
    const KEY_PREFIX: &'static str = NODE_OPERATOR_RECORD_KEY_PREFIX;
}

impl RegistryEntry for NodeRecord {
    const KEY_PREFIX: &'static str = NODE_RECORD_KEY_PREFIX;
}

impl RegistryEntry for SubnetRecord {
    const KEY_PREFIX: &'static str = SUBNET_RECORD_KEY_PREFIX;
}

trait RegistryFamilyEntries {
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, T>, Error>;
    fn get_family_entries_versioned<T: RegistryEntry + Default>(
        &self,
        version: RegistryVersion,
    ) -> Result<BTreeMap<String, (u64, T)>, Error>;
}

impl RegistryFamilyEntries for LocalRegistry {
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, T>, Error> {
        let prefix_length = T::KEY_PREFIX.len();
        Ok(self
            .get_key_family(T::KEY_PREFIX, self.get_latest_version())?
            .iter()
            .filter_map(|key| {
                self.get_value(key, self.get_latest_version())
                    .unwrap_or_else(|_| panic!("failed to get entry {} for type {}", key, std::any::type_name::<T>()))
                    .map(|v| {
                        (
                            key[prefix_length..].to_string(),
                            T::decode(v.as_slice()).expect("invalid registry value"),
                        )
                    })
            })
            .collect::<BTreeMap<_, _>>())
    }

    fn get_family_entries_versioned<T: RegistryEntry + Default>(
        &self,
        version: RegistryVersion,
    ) -> Result<BTreeMap<String, (u64, T)>, Error> {
        let prefix_length = T::KEY_PREFIX.len();
        Ok(self
            .get_key_family(T::KEY_PREFIX, self.get_latest_version())?
            .iter()
            .filter_map(|key| {
                self.get_versioned_value(key, version)
                    .map(|r| {
                        r.value.as_ref().map(|v| {
                            (
                                key[prefix_length..].to_string(),
                                (
                                    r.version.get(),
                                    T::decode(v.as_slice()).expect("invalid registry value"),
                                ),
                            )
                        })
                    })
                    .unwrap_or_else(|_| panic!("failed to get entry {} for type {}", key, std::any::type_name::<T>()))
            })
            .collect::<BTreeMap<_, _>>())
    }
}
