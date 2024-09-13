use std::{io::Write, path::PathBuf, process::Command, str::FromStr};

use crate::{
    auth::{Auth, Neuron, STAGING_KEY_PATH_FROM_HOME, STAGING_NEURON_ID},
    commands::{AuthOpts, AuthRequirement, HsmOpts},
};
use clio::{ClioPath, InputPath};
use hex;
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
                hsm_so_module: None,
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
) -> anyhow::Result<DreContext> {
    DreContext::new(
        network,
        vec![],
        auth,
        neuron_id,
        true,
        false,
        false,
        true,
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
    requirement: AuthRequirement,
    network: String,
    want: anyhow::Result<Neuron>,
}

// Must be left here until we add HSM simulator
#[allow(dead_code)]
impl<'a> NeuronAuthTestScenarion<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            neuron_id: None,
            private_key_pem: None,
            requirement: AuthRequirement::Anonymous,
            network: "".to_string(),
            want: Ok(Neuron::anonymous_neuron()),
        }
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
                    hsm_pin: None,
                    hsm_params: crate::commands::HsmParams {
                        hsm_slot: None,
                        hsm_key_id: None,
                    },
                    hsm_so_module: None,
                },
            },
            self.neuron_id,
            self.requirement.clone(),
            self.network.clone(),
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

fn ensure_slot_exists(slot: u64, label: &str, pin: &str) -> u64 {
    // If run multiple times for an existing slot it will fail but we don't care
    // since its a test
    let output = Command::new("softhsm2-util")
        .arg("--init-token")
        .arg("--slot")
        .arg(slot.to_string())
        .arg("--label")
        .arg(label)
        .arg("--so-pin")
        .arg(pin)
        .arg("--pin")
        .arg(pin)
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("Failed to create token: {}", String::from_utf8_lossy(&output.stderr))
    }
    let slot = String::from_utf8_lossy(&output.stdout);
    slot.trim().split(' ').last().unwrap().parse().unwrap()
}

fn delete_test_slot(slot: u64, label: &str) {
    // Cleanup. Again we don't care if it fails or not
    Command::new("softhsm2-util")
        .arg("--delete-token")
        .arg(slot.to_string())
        .arg("--token")
        .arg(label)
        .output()
        .unwrap();
}

fn generate_password_for_test_hsm(pin: &str, key_id: u8, label: &str) {
    Command::new("pkcs11-tool")
        .arg("--module")
        .arg(get_softhsm2_module())
        .arg("-l")
        .arg("-p")
        .arg(pin)
        .arg("-k")
        .arg("--id")
        .arg(key_id.to_string())
        .arg("--label")
        .arg(label)
        .arg("--key-type")
        .arg("EC:prime256v1")
        .output()
        .unwrap();
}

fn import_pem_as_pkey(pin: &str, key_id: u8, label: &str, module: &PathBuf) {
    // Convert the staging .pem file to have only private key
    let pem = std::fs::read_to_string(get_staging_key_path()).unwrap();
    let pem_lines = pem.lines().collect_vec();
    let pem_without_markings = pem_lines[1..pem_lines.len() - 1].join("");
    println!("PEM: {}", pem_without_markings);
    let hex = hex::encode(&openssl::base64::decode_block(&pem_without_markings).unwrap());
    let sub_hex_data = &hex[10..96];
    let final_hex_string = format!("302E020100{}", sub_hex_data);
    let binary_data = hex::decode(final_hex_string).unwrap();
    let only_priv = get_staging_key_path().parent().unwrap().join("identity-only-priv-key.pem");
    let pkey = openssl::pkey::PKey::private_key_from_der(&binary_data).unwrap();
    let mut file = std::fs::File::create(&only_priv).unwrap();
    file.write_all(&pkey.private_key_to_pem_pkcs8().unwrap()).unwrap();

    let output = Command::new("pkcs11-tool")
        .arg("--module")
        .arg(module.display().to_string())
        .arg("-l")
        .arg("-p")
        .arg(pin)
        .arg("--write-object")
        .arg(&only_priv.display().to_string())
        .arg("--type")
        .arg("privkey")
        .arg("--id")
        .arg(key_id.to_string())
        .arg("--label")
        .arg(label)
        .output()
        .unwrap();

    if !output.status.success() {
        panic!("Failed to create token: {}", String::from_utf8_lossy(&output.stderr))
    }
}

fn get_softhsm2_module() -> PathBuf {
    PathBuf::from_str(&std::env::var("SOFTHSM2_MODULE").unwrap_or("/usr/lib/softhsm/libsofthsm2.so".to_string())).unwrap()
}

struct HsmTestScenario<'a> {
    name: &'a str,
    pin: &'a str,
    slot: Option<u64>,
    key_id: Option<u8>,
    so_module: PathBuf,
    network: Network,
    requirement: AuthRequirement,
    neuron_id: Option<u64>,
    use_random_key: bool,
}

#[allow(dead_code)]
impl<'a> HsmTestScenario<'a> {
    fn new(name: &'a str, pin: &'a str) -> Self {
        Self {
            requirement: AuthRequirement::Anonymous,
            name,
            pin,
            slot: None,
            key_id: None,
            so_module: get_softhsm2_module(),
            network: Network::mainnet_unchecked().unwrap(),
            neuron_id: None,
            use_random_key: true,
        }
    }

    fn with_slot(self, slot: u64) -> Self {
        Self { slot: Some(slot), ..self }
    }

    fn with_key_id(self, key_id: u8) -> Self {
        Self {
            key_id: Some(key_id),
            ..self
        }
    }

    fn with_neuron_id(self, neuron_id: u64) -> Self {
        Self {
            neuron_id: Some(neuron_id),
            ..self
        }
    }

    fn for_network(self, network: Network) -> Self {
        Self { network, ..self }
    }

    fn when_required(self, requirement: AuthRequirement) -> Self {
        Self { requirement, ..self }
    }

    fn import_pem_as_hsm(self) -> Self {
        Self {
            use_random_key: false,
            ..self
        }
    }

    async fn build_neuron(&self) -> anyhow::Result<Neuron> {
        let auth_opts = AuthOpts {
            private_key_pem: None,
            hsm_opts: HsmOpts {
                hsm_pin: Some(self.pin.to_string()),
                hsm_so_module: Some(self.so_module.clone()),
                hsm_params: crate::commands::HsmParams {
                    hsm_slot: self.slot,
                    hsm_key_id: self.key_id,
                },
            },
        };

        Neuron::from_opts_and_req(auth_opts, self.requirement.clone(), &self.network, self.neuron_id).await
    }
}

// For some reason left.eq(&right) doesn't work.
fn compare_neurons(left: &Neuron, right: &Neuron) -> bool {
    if !left.include_proposer.eq(&right.include_proposer) || !left.neuron_id.eq(&right.neuron_id) {
        return false;
    }

    match (left.auth.clone(), right.auth.clone()) {
        (
            Auth::Hsm {
                pin: left_pin,
                slot: _,
                key_id: left_key_id,
                so_path: left_so_path,
            },
            Auth::Hsm {
                pin: right_pin,
                slot: _,
                key_id: right_key_id,
                so_path: right_so_path,
            },
        ) if PartialEq::eq(&left_pin, &right_pin) && left_key_id.eq(&right_key_id) && left_so_path.eq(&right_so_path) => true,
        (Auth::Keyfile { path: left_path }, Auth::Keyfile { path: right_path }) if left_path.eq(&right_path) => true,
        (Auth::Anonymous, Auth::Anonymous) => true,
        _ => false,
    }
}

#[tokio::test]
async fn hsm_neuron_tests() {
    let test_label = "Test HSM";
    let pin = "1234";
    let key_id = 1;
    let mut test_slot = 0;

    let scenarios = &[
        (
            HsmTestScenario::new("Detect slot and key id", pin).when_required(AuthRequirement::Signer),
            Ok(Neuron {
                auth: Auth::Hsm {
                    pin: pin.to_string(),
                    slot: 0,
                    key_id,
                    so_path: get_softhsm2_module(),
                },
                include_proposer: false,
                neuron_id: 0,
            }),
        ),
        (
            HsmTestScenario::new("Can't detect neuron_id", pin).when_required(AuthRequirement::Neuron),
            Err(anyhow::anyhow!("Error because private key doesn't control any neurons")),
        ),
        (
            HsmTestScenario::new("Detect only slot", pin)
                .when_required(AuthRequirement::Signer)
                .with_key_id(key_id),
            Ok(Neuron {
                auth: Auth::Hsm {
                    pin: pin.to_string(),
                    slot: 0,
                    key_id,
                    so_path: get_softhsm2_module(),
                },
                neuron_id: 0,
                include_proposer: false,
            }),
        ),
        (
            HsmTestScenario::new("Should be able to fetch neuron_id", pin)
                .when_required(AuthRequirement::Neuron)
                .import_pem_as_hsm(),
            Ok(Neuron {
                auth: Auth::Hsm {
                    pin: pin.to_string(),
                    slot: 0,
                    key_id,
                    so_path: get_softhsm2_module(),
                },
                neuron_id: 0,
                include_proposer: true,
            }),
        ),
    ];

    let mut failed = vec![];
    for (scenario, want) in scenarios {
        delete_test_slot(test_slot, test_label);
        test_slot = ensure_slot_exists(0, test_label, pin);
        match scenario.use_random_key {
            true => generate_password_for_test_hsm(pin, key_id, test_label),
            false => import_pem_as_pkey(pin, key_id, test_label, &get_softhsm2_module()),
        }
        let maybe_neuron = scenario.build_neuron().await;
        if (want.is_err() && maybe_neuron.is_ok()) || (want.is_ok() && maybe_neuron.is_err()) {
            failed.push((scenario.name, maybe_neuron, want));
            continue;
        }

        if want.is_err() && maybe_neuron.is_err() {
            continue;
        }

        let neuron = maybe_neuron.unwrap();
        let wanted = want.as_ref().unwrap();

        if !compare_neurons(&neuron, wanted) {
            failed.push((scenario.name, Ok(neuron), want))
        }
    }

    assert!(
        failed.is_empty(),
        "Failed scenarios:\n{}",
        failed
            .iter()
            .map(|(name, neuron, want)| format!("Scenario `{}`\nExpected:\n\t{:?}\nGot:\n\t{:?}", name, want, neuron))
            .join("\n")
    )
}
