use std::sync::Arc;

use ic_canisters::registry::RegistryCanisterWrapper;
use ic_interfaces_registry::RegistryTransportRecord;
use ic_management_backend::{health::MockHealthStatusQuerier, lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::Network;
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_types::RegistryVersion;

use crate::{
    artifact_downloader::MockArtifactDownloader, auth::Neuron, cordoned_feature_fetcher::MockCordonedFeatureFetcher, ctx::tests::get_mocked_ctx,
    ic_admin::MockIcAdmin,
};

#[tokio::test]
async fn registry_canister_calls() {
    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(MockLazyRegistry::new()),
        Arc::new(MockIcAdmin::new()),
        Arc::new(MockLazyGit::new()),
        Arc::new(MockProposalAgent::new()),
        Arc::new(MockArtifactDownloader::new()),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
    );

    let client = ctx.create_ic_agent_canister_client(None).await.unwrap();
    let client = RegistryCanisterWrapper::from(client);

    let certified_changes = client.get_certified_changes_since(0).await.unwrap();
    let changes = client.get_changes_since(0).await.unwrap();

    println!("Certified changes:\n{:?}", certified_changes.first().unwrap());
    println!(
        "Changes:\n{:?}",
        registry_deltas_to_registry_transport_records(changes.deltas).unwrap().first()
    );
}

pub fn registry_deltas_to_registry_transport_records(deltas: Vec<RegistryDelta>) -> anyhow::Result<Vec<RegistryTransportRecord>> {
    let mut records = Vec::new();
    for delta in deltas.into_iter() {
        let string_key = std::str::from_utf8(&delta.key[..])
            .map_err(|_| anyhow::anyhow!("Failed to convert key {:?} to string", delta.key))?
            .to_string();

        for value in delta.values.into_iter() {
            records.push(RegistryTransportRecord {
                key: string_key.clone(),
                value: if value.deletion_marker { None } else { Some(value.value) },
                version: RegistryVersion::new(value.version),
            });
        }
    }
    records.sort_by(|lhs, rhs| lhs.version.cmp(&rhs.version).then_with(|| lhs.key.cmp(&rhs.key)));
    Ok(records)
}
