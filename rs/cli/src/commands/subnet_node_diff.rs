use crate::auth::AuthRequirement;
use crate::ctx::DreContext;
use crate::exe::args::GlobalArgs;
use crate::exe::ExecutableCommand;
use anyhow::bail;
use clap::Args;
use ic_base_types::PrincipalId;
use serde::Serialize;
use std::collections::{HashMap, HashSet}; // Added HashSet
use std::path::PathBuf;
// Added for error handling

// Structs defined in the previous step (ChangeEntry, SubnetChangeLog, SubnetNodeDiffOutput)
// should be present above this. (They are already here)

#[derive(Serialize, Debug, Clone)]
struct ChangeEntry {
    registry_version: u64,
    node_count: usize,
}

#[derive(Serialize, Debug, Clone)]
struct SubnetChangeLog {
    subnet_id: String, // Store PrincipalId as String for straightforward serialization
    changes: Vec<ChangeEntry>,
}

// This will be the top-level structure for the JSON output
type SubnetNodeDiffOutput = Vec<SubnetChangeLog>;

// Helper function to get node counts for all subnets at a given registry version
async fn get_subnet_node_counts(ctx: &DreContext, version: u64) -> anyhow::Result<HashMap<PrincipalId, usize>> {
    // The implementation below uses `ctx.registry_provider().get_registry_at_version(version).await?`
    // and then `local_registry.subnets().await?`.
    // `ic_management_types::Subnet` has a `nodes: Vec<PrincipalId>` field, so `subnet_data.nodes.len()` is correct.
    let local_registry = ctx.registry_provider().get_registry_at_version(version).await?;
    let subnets = local_registry.subnets().await?; // This gets `HashMap<PrincipalId, ic_management_types::Subnet>`

    let mut subnet_node_counts = HashMap::new();
    for (subnet_id, subnet_data) in subnets {
        subnet_node_counts.insert(subnet_id, subnet_data.nodes.len());
    }
    Ok(subnet_node_counts)
}

#[derive(Args, Debug)]
pub struct SubnetNodeDiff {
    /// Registry version 1 for comparison
    #[clap(long)]
    pub version1: u64,

    /// Registry version 2 for comparison
    #[clap(long)]
    pub version2: u64,

    /// Output file for the JSON dump (default is stdout)
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
}

impl ExecutableCommand for SubnetNodeDiff {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn fetch_and_collate_subnet_changes(&self, ctx: &DreContext) -> anyhow::Result<SubnetNodeDiffOutput> {
        if self.version1 >= self.version2 {
            bail!("version1 ({}) must be strictly less than version2 ({}).", self.version1, self.version2);
        }

        let mut all_changes: HashMap<PrincipalId, Vec<ChangeEntry>> = HashMap::new();
        let mut previous_subnet_node_counts = get_subnet_node_counts(ctx, self.version1).await?;

        // Iterate from version1 + 1 up to version2
        for current_version in (self.version1 + 1)..=self.version2 {
            let current_subnet_node_counts = get_subnet_node_counts(ctx, current_version).await?;
            let mut checked_subnets = HashSet::new();

            for (subnet_id, current_count) in &current_subnet_node_counts {
                checked_subnets.insert(*subnet_id);
                let previous_count = previous_subnet_node_counts.get(subnet_id).copied();

                if previous_count.is_none() || previous_count.unwrap() != *current_count {
                    let change_entry = ChangeEntry {
                        registry_version: current_version,
                        node_count: *current_count,
                    };
                    all_changes.entry(*subnet_id).or_default().push(change_entry);
                }
            }

            for (subnet_id, _previous_count) in &previous_subnet_node_counts {
                if !checked_subnets.contains(subnet_id) {
                    let change_entry = ChangeEntry {
                        registry_version: current_version,
                        node_count: 0,
                    };
                    all_changes.entry(*subnet_id).or_default().push(change_entry);
                }
            }
            previous_subnet_node_counts = current_subnet_node_counts;
        }

        let output_data: SubnetNodeDiffOutput = all_changes
            .into_iter()
            .filter(|(_, changes)| !changes.is_empty())
            .map(|(subnet_id, changes)| SubnetChangeLog {
                subnet_id: subnet_id.to_string(),
                changes,
            })
            .collect();
        Ok(output_data)
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        let output_data = self.fetch_and_collate_subnet_changes(&ctx).await?;

        let writer: Box<dyn std::io::Write> = match &self.output {
            Some(path) => {
                // Ensure parent directory exists
                if let Some(parent_dir) = path.parent() {
                    fs_err::create_dir_all(parent_dir)?;
                }
                let file = fs_err::File::create(path)?;
                Box::new(std::io::BufWriter::new(file))
            }
            None => Box::new(std::io::stdout()),
        };
        serde_json::to_writer_pretty(writer, &output_data)?;

        Ok(())
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {
        // Validation moved to the beginning of `execute`
    }
}

// TODO: Add unit tests module
// Required imports for the new helper and logic:
// use std::collections::{HashMap, HashSet}; // HashSet for checked_subnets
// use anyhow::bail; // For error handling in execute
// These have been added at the top.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::{DreContext, RegistryProvider};
    use ic_management_types::{Network, Node, Operator, Provider, Subnet};
    use ic_registry_local_store::LocalStoreImpl; // For creating a mock LocalStore
    use std::collections::{HashMap, VecDeque};
    use std::sync::Arc;
    // Removed: use tempfile::NamedTempFile;
    // Removed: use assert_json_diff::assert_json_eq;
    use serde_json::json; // Used for creating expected values
    use tokio::runtime::Runtime; // For running async tests
                                 // std::io::Write is implicitly used by Box<dyn Write> in main code, not directly in tests now
    use std::str::FromStr; // For PrincipalId::from_str

    // Mock RegistryProvider
    #[derive(Clone)]
    struct MockRegistryProvider {
        versions: Arc<HashMap<u64, Arc<dyn LocalStoreImpl>>>,
    }

    #[async_trait::async_trait]
    impl RegistryProvider for MockRegistryProvider {
        async fn get_registry_at_version(&self, version: u64) -> anyhow::Result<Arc<dyn LocalStoreImpl>> {
            self.versions
                .get(&version)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Mock version not found: {}", version))
        }
        async fn get_latest_version(&self) -> anyhow::Result<u64> {
            unimplemented!("get_latest_version not implemented for mock")
        }
        fn get_network(&self) -> Network {
            unimplemented!("get_network not implemented for mock")
        }
        fn new_replica_agent(&self, _url: String, _maybe_fetch_root_key: bool) -> anyhow::Result<ic_agent::Agent> {
            unimplemented!("new_replica_agent not implemented for mock")
        }
        fn get_local_store_path(&self) -> PathBuf {
            unimplemented!("get_local_store_path not implemented for mock")
        }
    }

    #[derive(Default)]
    struct MockLocalStore {
        subnets_data: HashMap<PrincipalId, Subnet>,
    }

    #[async_trait::async_trait]
    impl LocalStoreImpl for MockLocalStore {
        async fn nodes(&self) -> anyhow::Result<HashMap<PrincipalId, Node>> {
            Ok(HashMap::new())
        }
        async fn subnets(&self) -> anyhow::Result<HashMap<PrincipalId, Subnet>> {
            Ok(self.subnets_data.clone())
        }
        async fn unassigned_nodes(&self) -> anyhow::Result<Vec<PrincipalId>> {
            Ok(vec![])
        }
        async fn operators(&self) -> anyhow::Result<HashMap<PrincipalId, Operator>> {
            Ok(HashMap::new())
        }
        async fn providers(&self) -> anyhow::Result<HashMap<PrincipalId, Provider>> {
            Ok(HashMap::new())
        }
        fn get_latest_version(&self) -> u64 {
            0
        }
        fn get_path(&self) -> PathBuf {
            PathBuf::new()
        }
        async fn get_updates_since(&self, _version: u64) -> anyhow::Result<Vec<ic_registry_local_store::RegistryValue>> {
            Ok(vec![])
        }
        async fn get_certified_updates_since(
            &self,
            _version: u64,
            _canister_id: PrincipalId,
        ) -> anyhow::Result<(
            Vec<ic_registry_local_store::RegistryValue>,
            u64,
            Arc<VecDeque<(u64, ic_registry_local_store::RegistryValue)>>,
        )> {
            anyhow::bail!("Unimplemented")
        }
        async fn get_changelog_since_version(&self, _version: u64) -> anyhow::Result<Vec<(u64, String, String, String)>> {
            anyhow::bail!("Unimplemented")
        }
        async fn get_diff(&self, _from: u64, _to: u64) -> anyhow::Result<Vec<(u64, String, String, String)>> {
            anyhow::bail!("Unimplemented")
        }
    }

    fn create_mock_dre_context(versions_data: HashMap<u64, HashMap<PrincipalId, Subnet>>) -> DreContext {
        let mut mock_versions = HashMap::new();
        for (version, subnets_data) in versions_data {
            let mock_store = Arc::new(MockLocalStore { subnets_data });
            mock_versions.insert(version, mock_store as Arc<dyn LocalStoreImpl>);
        }
        let mock_registry_provider = Arc::new(MockRegistryProvider {
            versions: Arc::new(mock_versions),
        });
        DreContext::new_anonymous_for_tests(mock_registry_provider, Network::Mainnet)
    }

    async fn run_and_get_json_output(cmd: &SubnetNodeDiff, ctx: DreContext) -> serde_json::Value {
        // Calls the refactored core logic function directly
        let output_data = cmd.fetch_and_collate_subnet_changes(&ctx).await.unwrap();
        serde_json::to_value(output_data).unwrap()
    }

    #[test]
    fn test_version1_ge_version2() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let ctx = create_mock_dre_context(HashMap::new());
            let cmd = SubnetNodeDiff {
                version1: 2,
                version2: 1,
                output: None,
            };
            // Test the fetch_and_collate_subnet_changes directly as it contains the validation
            let result = cmd.fetch_and_collate_subnet_changes(&ctx).await;
            assert!(result.is_err());
            assert_eq!(result.err().unwrap().to_string(), "version1 (2) must be strictly less than version2 (1).");
        });
    }

    #[test]
    fn test_no_changes() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let subnet1_id = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vbq6s-eqe").unwrap();
            let mut versions_data = HashMap::new();
            versions_data.insert(
                1,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(1)],
                        ..Subnet::default()
                    },
                )]),
            );
            versions_data.insert(
                2,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(1)],
                        ..Subnet::default()
                    },
                )]),
            );

            let ctx = create_mock_dre_context(versions_data);
            let cmd = SubnetNodeDiff {
                version1: 1,
                version2: 2,
                output: None,
            };

            let output = run_and_get_json_output(&cmd, ctx).await;
            let expected_val = json!([]);
            assert_eq!(output, expected_val);
        });
    }

    #[test]
    fn test_node_count_increases() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let subnet1_id = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vbq6s-eqe").unwrap();
            let mut versions_data = HashMap::new();
            versions_data.insert(
                1,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(10)],
                        ..Subnet::default()
                    },
                )]),
            );
            versions_data.insert(
                2,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(10)],
                        ..Subnet::default()
                    },
                )]),
            );
            versions_data.insert(
                3,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(10), PrincipalId::new_user_test_id(11)],
                        ..Subnet::default()
                    },
                )]),
            );

            let ctx = create_mock_dre_context(versions_data);
            let cmd = SubnetNodeDiff {
                version1: 1,
                version2: 3,
                output: None,
            };
            let output = run_and_get_json_output(&cmd, ctx).await;

            let expected_val = json!([
                {
                    "subnet_id": subnet1_id.to_string(),
                    "changes": [
                        { "registry_version": 3, "node_count": 2 }
                    ]
                }
            ]);
            assert_eq!(output, expected_val);
        });
    }

    #[test]
    fn test_subnet_added() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let subnet1_id = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vbq6s-eqe").unwrap();
            let mut versions_data = HashMap::new();
            versions_data.insert(5, HashMap::new());
            versions_data.insert(
                6,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(1)],
                        ..Subnet::default()
                    },
                )]),
            );

            let ctx = create_mock_dre_context(versions_data);
            let cmd = SubnetNodeDiff {
                version1: 5,
                version2: 6,
                output: None,
            };
            let output = run_and_get_json_output(&cmd, ctx).await;

            let expected_val = json!([
                {
                    "subnet_id": subnet1_id.to_string(),
                    "changes": [
                        { "registry_version": 6, "node_count": 1 }
                    ]
                }
            ]);
            assert_eq!(output, expected_val);
        });
    }

    #[test]
    fn test_subnet_removed() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let subnet1_id = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vbq6s-eqe").unwrap();
            let mut versions_data = HashMap::new();
            versions_data.insert(
                10,
                HashMap::from([(
                    subnet1_id,
                    Subnet {
                        nodes: vec![PrincipalId::new_user_test_id(1)],
                        ..Subnet::default()
                    },
                )]),
            );
            versions_data.insert(11, HashMap::new());

            let ctx = create_mock_dre_context(versions_data);
            let cmd = SubnetNodeDiff {
                version1: 10,
                version2: 11,
                output: None,
            };
            let output = run_and_get_json_output(&cmd, ctx).await;

            let expected_val = json!([
                {
                    "subnet_id": subnet1_id.to_string(),
                    "changes": [
                        { "registry_version": 11, "node_count": 0 }
                    ]
                }
            ]);
            assert_eq!(output, expected_val);
        });
    }

    #[test]
    fn test_multiple_changes_multiple_subnets() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let s1_id = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vbq6s-eqe").unwrap();
            let s2_id = PrincipalId::from_str("uzr34-ryaaa-aaaaa-aaada-cai").unwrap();
            let node1 = PrincipalId::new_user_test_id(1);
            let node2 = PrincipalId::new_user_test_id(2);
            let node3 = PrincipalId::new_user_test_id(3);

            let mut versions_data = HashMap::new();
            versions_data.insert(
                20,
                HashMap::from([
                    (
                        s1_id,
                        Subnet {
                            nodes: vec![node1],
                            ..Subnet::default()
                        },
                    ),
                    (
                        s2_id,
                        Subnet {
                            nodes: vec![node1, node2],
                            ..Subnet::default()
                        },
                    ),
                ]),
            );
            versions_data.insert(
                21,
                HashMap::from([
                    (
                        s1_id,
                        Subnet {
                            nodes: vec![node1],
                            ..Subnet::default()
                        },
                    ),
                    (
                        s2_id,
                        Subnet {
                            nodes: vec![node1],
                            ..Subnet::default()
                        },
                    ),
                ]),
            );
            versions_data.insert(
                22,
                HashMap::from([(
                    s1_id,
                    Subnet {
                        nodes: vec![node1, node2, node3],
                        ..Subnet::default()
                    },
                )]),
            );
            let s3_id = PrincipalId::from_str("xkbj4-xqaaa-aaaam-qafda-cai").unwrap();
            versions_data.insert(
                23,
                HashMap::from([
                    (
                        s1_id,
                        Subnet {
                            nodes: vec![node1, node2, node3],
                            ..Subnet::default()
                        },
                    ),
                    (
                        s3_id,
                        Subnet {
                            nodes: vec![node1],
                            ..Subnet::default()
                        },
                    ),
                ]),
            );

            let ctx = create_mock_dre_context(versions_data);
            let cmd = SubnetNodeDiff {
                version1: 20,
                version2: 23,
                output: None,
            };
            let output = run_and_get_json_output(&cmd, ctx).await; // Pass cmd by reference

            let expected_val = json!([ // Renamed to expected_val for clarity
                {
                    "subnet_id": s1_id.to_string(),
                    "changes": [ { "registry_version": 22, "node_count": 3 } ]
                },
                {
                    "subnet_id": s2_id.to_string(),
                    "changes": [
                        { "registry_version": 21, "node_count": 1 },
                        { "registry_version": 22, "node_count": 0 }
                    ]
                },
                {
                    "subnet_id": s3_id.to_string(),
                    "changes": [ { "registry_version": 23, "node_count": 1 } ]
                }
            ]);

            // The output is already serde_json::Value, but if we want to sort, we need to deserialize, sort, then re-serialize to json for comparison.
            let mut output_vec: Vec<SubnetChangeLog> = serde_json::from_value(output).unwrap();
            output_vec.sort_by(|a, b| a.subnet_id.cmp(&b.subnet_id));
            for change_log in &mut output_vec {
                change_log.changes.sort_by_key(|c| c.registry_version);
            }

            let mut expected_vec: Vec<SubnetChangeLog> = serde_json::from_value(expected_val.clone()).unwrap(); // Clone expected_val before moving it
            expected_vec.sort_by(|a, b| a.subnet_id.cmp(&b.subnet_id));
            for change_log in &mut expected_vec {
                change_log.changes.sort_by_key(|c| c.registry_version);
            }

            assert_eq!(json!(output_vec), json!(expected_vec));
        });
    }
}
