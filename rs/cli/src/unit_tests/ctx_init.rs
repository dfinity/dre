use std::path::PathBuf;

use crate::commands::AuthOpts;
use ic_canisters::governance::governance_canister_version;
use ic_management_types::Network;

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

struct TestScenario<'a> {
    name: &'static str,
    version: IcAdminVersion,
    should_delete_status_file: bool,
    should_contain: Option<&'a str>,
}

impl<'a> TestScenario<'a> {
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
        TestScenario::new("match governance canister")
            .delete_status_file()
            .should_contain(&governance_version.stringified_hash),
        TestScenario::new("use default version")
            .delete_status_file()
            .version(IcAdminVersion::Fallback)
            .should_contain(FALLBACK_IC_ADMIN_VERSION),
        TestScenario::new("existing version on s3")
            .delete_status_file()
            .version(IcAdminVersion::Strict(version_on_s3.to_string()))
            .should_contain(version_on_s3),
        TestScenario::new("random version not present on s3").version(IcAdminVersion::Strict("random-version".to_string())),
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

            let ic_admin_path = ctx.ic_admin().await.ic_admin_path().unwrap();
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
                "Test `{}`: expected ok but got err: {:?}",
                test.name,
                maybe_ctx.err().unwrap()
            );
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
