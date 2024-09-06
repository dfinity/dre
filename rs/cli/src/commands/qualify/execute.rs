use std::path::PathBuf;

use clap::Args;
use ic_management_types::Network;
use serde_json::Value;

use crate::{commands::ExecutableCommand, ic_admin::IcAdmin, qualification::QualificationExecutorBuilder};

#[derive(Args, Debug)]
pub struct Execute {
    /// Version which is to be qualified
    #[clap(long, short)]
    pub version: String,

    /// Starting version for the network.
    ///
    /// If left empty, the tool will use the current NNS version
    #[clap(long, short)]
    pub from_version: Option<String>,

    /// Specify the steps to run
    /// A range can be: `4`, `3..`, `..3, `1..3`
    #[clap(long)]
    pub step_range: Option<String>,

    /// Name of the deployment used for prometheus querying of `ic` label: `staging`, `from-config`...
    #[clap(long)]
    pub deployment_name: String,

    /// Prometheus compliant endpoint
    #[clap(long)]
    pub prometheus_endpoint: String,

    /// Artifacts path
    #[clap(long)]
    pub artifacts: Option<PathBuf>,

    /// Grafana url, needed if `artifacts` are present
    pub grafana_url: Option<String>,
}

impl ExecutableCommand for Execute {
    fn require_auth(&self) -> crate::commands::AuthRequirement {
        crate::commands::AuthRequirement::Neuron
    }

    fn validate(&self, cmd: &mut clap::Command) {
        if self.artifacts.is_some() && self.grafana_url.is_none() {
            cmd.error(
                clap::error::ErrorKind::InvalidValue,
                "`grafana_url` is mandatory if `artifacts` are to be exported",
            )
            .exit()
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        if ctx.network().eq(&Network::mainnet_unchecked().unwrap()) {
            anyhow::bail!("Qualification is not allowed on mainnet.")
        }

        let from_version = match &self.from_version {
            Some(v) => v.to_string(),
            None => {
                let anonymous_admin_wrapper_for_mainnet = ctx.readonly_ic_admin_for_other_network(Network::mainnet_unchecked().unwrap());

                let subnets = ctx.registry().await.subnets().await?;
                let nns_subnet_id = subnets.keys().next().unwrap();

                let output = anonymous_admin_wrapper_for_mainnet
                    .run_passthrough_get(&["subnet".to_string(), nns_subnet_id.to_string()], true)
                    .await?;

                let output = serde_json::from_str::<Value>(&output)?;
                output["records"][0]["value"]["replica_version_id"]
                    .as_str()
                    .ok_or(anyhow::anyhow!("Failed to get replica version id for nns"))?
                    .to_string()
            }
        };

        let mut qualification_executor = QualificationExecutorBuilder::new(ctx)
            .with_step_range(self.step_range.clone().unwrap_or_default())
            .with_from_version(from_version)
            .with_to_version(self.version.clone())
            .with_deployment_name(self.deployment_name.clone())
            .with_prometheus_endpoint(self.prometheus_endpoint.clone());
        if let Some(path) = &self.artifacts {
            qualification_executor = qualification_executor.with_artifacts(path.to_owned());
        };
        qualification_executor.build()?.execute().await
    }
}
