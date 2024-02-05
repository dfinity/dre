use std::{path::PathBuf, str::FromStr, time::Duration};

use anyhow::Error;
use ic_base_types::RegistryVersion;
use ic_interfaces_registry::RegistryClient;
use ic_management_backend::registry::{local_registry_path, sync_local_store, RegistryFamilyEntries};
use ic_management_types::Network;
use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord,
};
use ic_registry_local_registry::LocalRegistry;
use uuid::Uuid;

pub async fn dump_registry(path: &Option<PathBuf>, version: &i64, network: Network) -> Result<(), Error> {
    let (path, should_dispose) = match path {
        Some(p) => (p.clone(), false),
        None => {
            let uuid = Uuid::new_v4();
            let binding = format!("~/tmp/{}", uuid);
            let local_temp = shellexpand::tilde(&binding);
            (
                PathBuf::from_str(&local_temp).map_err(|e| anyhow::anyhow!("Couldn't create path: {:?}", e))?,
                true,
            )
        }
    };

    std::env::set_var("LOCAL_REGISTRY_PATH", path.clone());

    sync_local_store(network.clone()).await?;

    let local_registry = LocalRegistry::new(local_registry_path(network), Duration::from_secs(10))
        .map_err(|e| anyhow::anyhow!("Couldn't create local registry client instance: {:?}", e))?;

    // determine desired version
    let version = {
        if *version >= 0 {
            RegistryVersion::new(*version as u64)
        } else {
            local_registry.get_latest_version()
        }
    };

    let nodes = local_registry
        .get_family_entries_of_version::<NodeRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let nodes = serde_json::to_string(&nodes).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    let subnets = local_registry
        .get_family_entries_of_version::<SubnetRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let subnets = serde_json::to_string(&subnets).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    let dcs = local_registry
        .get_family_entries_of_version::<DataCenterRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let dcs = serde_json::to_string(&dcs).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    let node_operators = local_registry
        .get_family_entries_of_version::<NodeOperatorRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?;
    let node_operators =
        serde_json::to_string(&node_operators).map_err(|e| anyhow::anyhow!("Couldn't convert to JSON: {:?}", e))?;

    println!(
        "{{ \"nodes\": {}, \"subnets\": {}, \"dcs\": {}, \"node_operators\": {} }}",
        nodes, subnets, dcs, node_operators
    );

    if should_dispose {
        std::fs::remove_dir_all(path).map_err(|e| anyhow::anyhow!("Error removing created dir: {:?}", e))?
    }
    Ok(())
}
