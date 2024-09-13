use std::{path::PathBuf, str::FromStr};

use crate::{
    auth::{Auth, Neuron, STAGING_KEY_PATH_FROM_HOME, STAGING_NEURON_ID},
    commands::{AuthOpts, AuthRequirement, HsmOpts},
};
use clio::{ClioPath, InputPath};
use ic_canisters::governance::governance_canister_version;
use ic_management_types::Network;
use itertools::Itertools;

use crate::{commands::IcAdminVersion, ctx::DreContext, ic_admin::FALLBACK_IC_ADMIN_VERSION};

fn status_file_path() -> PathBuf {
    dirs::home_dir().unwrap().join("bin").join("ic-admin.revisions").join("ic-admin.status")
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
        network.name.clone(),
        network.nns_urls.clone(),
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
        true,
        false,
        true,
        crate::commands::AuthRequirement::Anonymous,
        None,
        version,
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

#[tokio::test]
async fn init_tests_ic_admin_version() {
    let version_on_s3 = "e47293c0bd7f39540245913f7f75be3d6863183c";
    let mainnet = Network::mainnet_unchecked().unwrap();
    let governance_version = governance_canister_version(&mainnet.nns_urls).await.unwrap();

    let tests = &[
        AdminVersionTestScenario::new("match governance canister")
            .delete_status_file()
            .should_contain(&governance_version.stringified_hash),
        AdminVersionTestScenario::new("use default version")
            .delete_status_file()
            .version(IcAdminVersion::Fallback)
            .should_contain(FALLBACK_IC_ADMIN_VERSION),
        AdminVersionTestScenario::new("existing version on s3")
            .delete_status_file()
            .version(IcAdminVersion::Strict(version_on_s3.to_string()))
            .should_contain(version_on_s3),
        AdminVersionTestScenario::new("random version not present on s3").version(IcAdminVersion::Strict("random-version".to_string())),
    ];

    for test in tests {
        let mut deleted_status_file: PathBuf = dirs::home_dir().unwrap();
        if test.should_delete_status_file {
            deleted_status_file = get_deleted_status_file();
        }

        let maybe_ctx = get_context(&mainnet, test.version.clone()).await;

        if let Some(ver) = test.should_contain {
            assert!(
                maybe_ctx.is_ok(),
                "Test `{}`: expected to create DreContext, but got error: {:?}",
                test.name,
                maybe_ctx.err().unwrap()
            );
            let ctx = maybe_ctx.unwrap();

            let ic_admin_path = ctx.ic_admin().await;
            assert!(ic_admin_path.is_ok());
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
            let maybe_ic_admin = ctx.ic_admin().await;
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
) -> anyhow::Result<DreContext> {
    DreContext::new(
        network,
        vec![],
        auth,
        neuron_id,
        true,
        false,
        false,
        dry_run,
        requirement,
        None,
        IcAdminVersion::Strict("Shouldn't get to here".to_string()),
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
        }
    }

    fn is_dry_run(self) -> Self {
        Self { dry_run: true, ..self }
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
        get_ctx_for_neuron_test(
            AuthOpts {
                private_key_pem: self
                    .private_key_pem
                    .as_ref()
                    .map(|path| InputPath::new(ClioPath::new(path).unwrap()).unwrap()),
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
        )
        .await
        .map(|ctx| ctx.neuron())
    }
}

fn get_staging_key_path() -> PathBuf {
    PathBuf::from_str(&std::env::var("HOME").unwrap())
        .unwrap()
        .join(STAGING_KEY_PATH_FROM_HOME)
}

fn ensure_testing_pem(name: &str) -> PathBuf {
    let path = PathBuf::from_str(&std::env::var("HOME").unwrap())
        .unwrap()
        .join(format!(".config/dfx/identity/{}/identity.pem", name));

    let parent = path.parent().unwrap();
    if !parent.exists() {
        std::fs::create_dir_all(parent).unwrap()
    }

    if !path.exists() {
        std::fs::write(&path, "Some private key").unwrap();
    }
    path
}

#[tokio::test]
async fn init_test_neuron_and_auth() {
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
        NeuronAuthTestScenarion::new("Dry running commands shouldn't fail if neuron cannot be detected")
            .with_network("mainnet")
            .is_dry_run()
            .want(Ok(Neuron {
                auth: Auth::Anonymous,
                neuron_id: 0,
                include_proposer: false,
            }))
            .when_requirement(AuthRequirement::Neuron),
        // Failing scenarios
        //
        NeuronAuthTestScenarion::new("Detecting neuron id for random private key")
            .with_network("mainnet")
            .with_private_key(ensure_testing_pem("testing").to_str().unwrap().to_string())
            .want(Err(anyhow::anyhow!("Will not be able to detect neuron id")))
            .when_requirement(AuthRequirement::Neuron),
    ];

    let mut outcomes = vec![];
    for test in scenarios {
        let got = test.get_neuron().await;
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
