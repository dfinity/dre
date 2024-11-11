use std::{path::PathBuf, sync::Arc};

use crate::{
    auth::{Auth, Neuron, STAGING_KEY_PATH_FROM_HOME, STAGING_NEURON_ID},
    commands::{AuthOpts, AuthRequirement, HsmOpts},
    cordoned_feature_fetcher::MockCordonedFeatureFetcher,
    store::{Store, FALLBACK_IC_ADMIN_VERSION},
};
use ic_canisters::governance::governance_canister_version;
use ic_management_backend::health::MockHealthStatusQuerier;
use ic_management_types::Network;
use itertools::Itertools;

use crate::{commands::IcAdminVersion, ctx::DreContext};

fn status_file_path() -> PathBuf {
    Store::new(true).unwrap().ic_admin_status_file_outer().unwrap()
}

fn get_deleted_status_file() -> PathBuf {
    let status_file = status_file_path();
    if status_file.exists() {
        std::fs::remove_file(&status_file).unwrap()
    }
    status_file
}

async fn get_context(network: &Network, version: IcAdminVersion) -> anyhow::Result<DreContext> {
    DreContext::new(
        network.clone(),
        AuthOpts {
            private_key_pem: None,
            hsm_opts: crate::commands::HsmOpts {
                hsm_pin: None,
                hsm_params: crate::commands::HsmParams {
                    hsm_slot: None,
                    hsm_key_id: None,
                },
            },
        },
        None,
        false,
        false,
        true,
        crate::commands::AuthRequirement::Anonymous,
        None,
        version,
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
        Store::new(false)?,
    )
    .await
}

struct AdminVersionTestScenario<'a> {
    name: &'static str,
    version: IcAdminVersion,
    should_delete_status_file: bool,
    should_contain: Option<&'a str>,
}

impl<'a> AdminVersionTestScenario<'a> {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            version: IcAdminVersion::FromGovernance,
            should_delete_status_file: false,
            should_contain: None,
        }
    }

    fn version(self, version: IcAdminVersion) -> Self {
        Self { version, ..self }
    }

    fn delete_status_file(self) -> Self {
        Self {
            should_delete_status_file: true,
            ..self
        }
    }

    fn should_contain(self, ver: &'a str) -> Self {
        Self {
            should_contain: Some(ver),
            ..self
        }
    }
}

#[test]
fn init_tests_ic_admin_version() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let version_on_s3 = "e47293c0bd7f39540245913f7f75be3d6863183c";
    let mainnet = Network::mainnet_unchecked().unwrap();
    let governance_version = runtime.block_on(governance_canister_version(&mainnet.nns_urls)).unwrap();

    let tests = &[
        AdminVersionTestScenario::new("match governance canister")
            .delete_status_file()
            .should_contain(&governance_version.stringified_hash),
        AdminVersionTestScenario::new("use default version")
            .version(IcAdminVersion::Fallback)
            .should_contain(FALLBACK_IC_ADMIN_VERSION),
        AdminVersionTestScenario::new("existing version on s3")
            .version(IcAdminVersion::Strict(version_on_s3.to_string()))
            .should_contain(version_on_s3),
        AdminVersionTestScenario::new("random version not present on s3").version(IcAdminVersion::Strict("random-version".to_string())),
    ];

    for test in tests {
        let mut deleted_status_file: PathBuf = dirs::home_dir().unwrap();
        if test.should_delete_status_file {
            deleted_status_file = get_deleted_status_file();
        }

        let maybe_ctx = runtime.block_on(get_context(&mainnet, test.version.clone()));

        if let Some(ver) = test.should_contain {
            assert!(
                maybe_ctx.is_ok(),
                "Test `{}`: expected to create DreContext, but got error: {:?}",
                test.name,
                maybe_ctx.err().unwrap()
            );
            let ctx = maybe_ctx.unwrap();

            let ic_admin_path = runtime.block_on(ctx.ic_admin());
            assert!(ic_admin_path.is_ok(), "Expected Ok, but was: {:?}", ic_admin_path);
            let ic_admin_path = ic_admin_path.unwrap().ic_admin_path().unwrap_or_default();
            assert!(
                ic_admin_path.contains(ver),
                "Test `{}`: ic_admin_path `{}`, expected version `{}`",
                test.name,
                ic_admin_path,
                ver
            )
        } else {
            assert!(
                maybe_ctx.is_ok(),
                "Test `{}`: expected ok for ctx but got err: {:?}",
                test.name,
                maybe_ctx.err().unwrap()
            );

            let ctx = maybe_ctx.unwrap();
            let maybe_ic_admin = runtime.block_on(ctx.ic_admin());
            assert!(
                maybe_ic_admin.is_err(),
                "Test `{}`: expected err for ic-admin but got ok with path: {}",
                test.name,
                maybe_ic_admin.unwrap().ic_admin_path().unwrap_or_default()
            )
        }

        if test.should_delete_status_file {
            assert!(
                deleted_status_file.exists(),
                "Test `{}`: expected ic-admin.status file to be recreated, but it wasn't",
                test.name
            )
        }
    }
}

async fn get_ctx_for_neuron_test(
    auth: AuthOpts,
    neuron_id: Option<u64>,
    requirement: AuthRequirement,
    network: String,
    dry_run: bool,
    offline: bool,
) -> anyhow::Result<DreContext> {
    DreContext::new(
        ic_management_types::Network::new_unchecked(network, &[]).unwrap(),
        auth,
        neuron_id,
        true,
        false,
        dry_run,
        requirement,
        None,
        IcAdminVersion::Strict("Shouldn't get to here".to_string()),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
        Store::new(offline)?,
    )
    .await
}

struct NeuronAuthTestScenarion<'a> {
    name: &'a str,
    neuron_id: Option<u64>,
    private_key_pem: Option<String>,
    hsm_pin: Option<String>,
    hsm_key_id: Option<u8>,
    hsm_slot: Option<u64>,
    requirement: AuthRequirement,
    network: String,
    want: anyhow::Result<Neuron>,
    dry_run: bool,
    offline: bool,
}

// Must be left here until we add HSM simulator
#[allow(dead_code)]
impl<'a> NeuronAuthTestScenarion<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            neuron_id: None,
            private_key_pem: None,
            hsm_pin: None,
            hsm_key_id: None,
            hsm_slot: None,
            requirement: AuthRequirement::Anonymous,
            network: "".to_string(),
            want: Ok(Neuron::anonymous_neuron()),
            dry_run: false,
            offline: false,
        }
    }

    // It really is self so that we can use
    // `test.is_dry_run().with_neuron_id(...)`
    #[allow(clippy::wrong_self_convention)]
    fn dry_run(self) -> Self {
        Self { dry_run: true, ..self }
    }

    fn offline(self) -> Self {
        Self { offline: true, ..self }
    }

    fn with_neuron_id(self, neuron_id: u64) -> Self {
        Self {
            neuron_id: Some(neuron_id),
            ..self
        }
    }

    fn with_private_key(self, private_key_path: String) -> Self {
        Self {
            private_key_pem: Some(private_key_path),
            ..self
        }
    }

    fn with_pin(self, hsm_pin: &'a str) -> Self {
        Self {
            hsm_pin: Some(hsm_pin.to_string()),
            ..self
        }
    }

    fn with_key_id(self, hsm_key_id: u8) -> Self {
        Self {
            hsm_key_id: Some(hsm_key_id),
            ..self
        }
    }

    fn with_slot(self, hsm_slot: u64) -> Self {
        Self {
            hsm_slot: Some(hsm_slot),
            ..self
        }
    }

    fn when_requirement(self, auth: AuthRequirement) -> Self {
        Self { requirement: auth, ..self }
    }

    fn with_network(self, network: &'a str) -> Self {
        Self {
            network: network.to_string(),
            ..self
        }
    }

    fn want(self, neuron: anyhow::Result<Neuron>) -> Self {
        Self { want: neuron, ..self }
    }

    async fn get_neuron(&self) -> anyhow::Result<Neuron> {
        let ctx = get_ctx_for_neuron_test(
            AuthOpts {
                private_key_pem: self.private_key_pem.clone(),
                hsm_opts: HsmOpts {
                    hsm_pin: self.hsm_pin.clone(),
                    hsm_params: crate::commands::HsmParams {
                        hsm_slot: self.hsm_slot,
                        hsm_key_id: self.hsm_key_id,
                    },
                },
            },
            self.neuron_id,
            self.requirement.clone(),
            self.network.clone(),
            self.dry_run,
            self.offline,
        )
        .await?;
        ctx.neuron().await
    }
}

fn get_staging_key_path() -> PathBuf {
    dirs::home_dir().unwrap().join(STAGING_KEY_PATH_FROM_HOME)
}

#[test]
fn init_test_neuron_and_auth() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let scenarios = &[
        // Successful scenarios
        //
        // Should use the known neuron for staging
        // If run in CI it will require the key to be there
        NeuronAuthTestScenarion::new("Staging signer")
            .with_network("staging")
            .want(Ok(Neuron {
                auth: Auth::Keyfile {
                    path: get_staging_key_path(),
                },
                ..Neuron::anonymous_neuron()
            }))
            .when_requirement(AuthRequirement::Signer),
        NeuronAuthTestScenarion::new("Staging anonymous")
            .with_network("staging")
            .want(Ok(Neuron::anonymous_neuron()))
            .when_requirement(AuthRequirement::Anonymous),
        NeuronAuthTestScenarion::new("Staging neuron")
            .with_network("staging")
            .want(Ok(Neuron {
                auth: Auth::Keyfile {
                    path: get_staging_key_path(),
                },
                neuron_id: STAGING_NEURON_ID,
                include_proposer: true,
            }))
            .when_requirement(AuthRequirement::Neuron),
        NeuronAuthTestScenarion::new("Mainnet neuron when offline")
            .with_network("mainnet")
            .with_private_key(Neuron::ensure_fake_pem_outer("test_neuron_1").unwrap().to_str().unwrap().to_string())
            .offline()
            .want(Ok(Neuron {
                auth: Auth::Keyfile {
                    path: Neuron::ensure_fake_pem_outer("test_neuron_1").unwrap(),
                },
                neuron_id: 0,
                include_proposer: true,
            }))
            .when_requirement(AuthRequirement::Neuron),
        NeuronAuthTestScenarion::new("Dry running commands shouldn't fail if neuron cannot be detected")
            .with_network("mainnet")
            .dry_run()
            .want(Ok(Neuron::dry_run_fake_neuron().unwrap()))
            .when_requirement(AuthRequirement::Neuron),
    ];

    let mut outcomes = vec![];
    for test in scenarios {
        let got = runtime.block_on(test.get_neuron());
        outcomes.push((
            test.name,
            format!("{:?}", test.want),
            format!("{:?}", got),
            (match (&test.want, got) {
                (Ok(want), Ok(got)) => want.eq(&got),
                (Ok(_), Err(_)) => false,
                (Err(_), Ok(_)) => false,
                (Err(_), Err(_)) => true,
            }),
        ))
    }

    assert!(
        outcomes.iter().map(|(_, _, _, is_successful)| is_successful).all(|o| *o),
        "{}",
        outcomes
            .iter()
            .filter_map(|(name, wanted, got, is_successful)| match is_successful {
                true => None,
                false => Some(format!("Test `{}` failed:\nWanted:\n\t{}\nGot:\n\t{}\n", name, wanted, got)),
            })
            .join("\n\n")
    )
}
