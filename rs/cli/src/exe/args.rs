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

    /// Path to an explicit ic-admin binary to use. When set, dre will not
    /// download ic-admin and will use this binary instead (overrides
    /// --ic-admin-version). A bare command name is resolved via $PATH.
    #[clap(long, global = true, env = "IC_ADMIN")]
    pub ic_admin: Option<String>,

    #[clap(long, global = true, env = "IC_ADMIN_VERSION", default_value = "from-registry", value_parser = clap::value_parser!(IcAdminVersion), help = r#"Specify the version of ic-admin to use:
    1. from-registry, registry, reg, r, governance, g => version matching the NNS registry canister (default)
    2. fallback, default, f, d                         => fallback version embedded at build time
    3. <commit>                                        => specific commit/release (scans all IC releases)"#)]
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
    #[clap(long, global = true, visible_aliases = &["cf-file", "cfff", "cordone"])]
    pub cordoned_features_file: Option<String>,

    /// Override health by data center ID or node ID, e.g. "sg2:healthy". Accepts multiple entries.
    #[clap(long, global = true, num_args(1..), visible_aliases = &["oh", "override-healths"], help = r#"Override health for nodes. Matches exact data center ID (dc_id) or full node principal.
Examples:
    --override-health sg2:healthy
    --override-health mn2:healthy de1:dead
    --override-health bo1:degraded z6jp6-245uu-...:healthy

Accepted health values: healthy, degraded, dead, unknown (case-insensitive)."#)]
    pub override_health: Vec<String>,
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
            // The registry canister version is the version the NNS/governance has
            // deployed, hence the `governance` aliases are accepted here too.
            "from-registry" | "registry" | "reg" | "r" | "from-governance" | "governance" | "govn" | "g" => Self::FromRegistry,
            "fallback" | "f" | "default" | "d" => Self::Fallback,
            s => Self::Strict(s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IcAdminVersion;

    #[test]
    fn ic_admin_version_aliases() {
        for alias in ["from-registry", "registry", "reg", "r", "from-governance", "governance", "govn", "g"] {
            assert!(
                matches!(IcAdminVersion::from(alias), IcAdminVersion::FromRegistry),
                "`{}` should parse as FromRegistry",
                alias
            );
        }
        for alias in ["fallback", "f", "default", "d"] {
            assert!(
                matches!(IcAdminVersion::from(alias), IcAdminVersion::Fallback),
                "`{}` should parse as Fallback",
                alias
            );
        }
        // Anything else is treated as a specific commit/release.
        assert!(matches!(
            IcAdminVersion::from("fb721da900b9e9219773ee312f987971338f7c62"),
            IcAdminVersion::Strict(c) if c == "fb721da900b9e9219773ee312f987971338f7c62"
        ));
    }
}
