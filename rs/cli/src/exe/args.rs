use crate::auth::AuthOpts;
use clap::Parser;
use strum::Display;
use url::Url;

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, author)]
pub struct GlobalArgs {
    #[clap(flatten)]
    pub(crate) auth_opts: AuthOpts,

    /// Neuron ID
    #[clap(long, global = true, env = "NEURON_ID", visible_aliases = &["neuron", "proposer"])]
    pub neuron_id: Option<u64>,

    /// Path to explicitly state ic-admin path to use
    #[clap(long, global = true, env = "IC_ADMIN")]
    pub ic_admin: Option<String>,

    #[clap(long, global = true, env = "IC_ADMIN_VERSION", default_value = "from-registry", value_parser = clap::value_parser!(IcAdminVersion), help = r#"Specify the version of ic admin to use
Options:
    1. from-governance, governance, govn, g => same as governance canister
    2. default, d => strict default version, embedded at build time
    3. <commit> => specific commit"#)]
    pub ic_admin_version: IcAdminVersion,

    #[clap(
        long,
        env = "NETWORK",
        default_value = "mainnet",
        help = r#"Specify the target network:
    - "mainnet",
    - "staging",
    - "<testnet>"
"#
    )]
    pub network: String,

    #[clap(long, env = "NNS_URLS", value_delimiter = ',', aliases = ["registry-url", "nns-url"], help = r#"NNS_URLs for target network, comma separated.
The argument is mandatory for testnets, and is optional for mainnet and staging"#)]
    pub nns_urls: Vec<Url>,

    /// To print as much information as possible
    #[clap(long, env = "VERBOSE", global = true)]
    pub verbose: bool,

    /// Run the tool offline when possible, i.e., do not sync registry and public dashboard data before the run
    ///
    /// Useful for when the NNS or Public dashboard are unreachable
    #[clap(long)]
    pub offline: bool,

    /// Path to file which contains cordoned features
    #[clap(long, global = true, visible_aliases = &["cf-file", "cfff"])]
    pub cordoned_features_file: Option<String>,
}

#[derive(Debug, Display, Clone)]
pub enum IcAdminVersion {
    FromRegistry,
    Fallback,
    Strict(String),
}

impl From<&str> for IcAdminVersion {
    fn from(value: &str) -> Self {
        match value {
            "from-registry" | "registry" | "r" | "reg" => Self::FromRegistry,
            "fallback" | "f" => Self::Fallback,
            s => Self::Strict(s.to_string()),
        }
    }
}
