use std::{env::home_dir, path::PathBuf, str::FromStr};

use ic_nervous_system_common_test_keys::TEST_NEURON_1_OWNER_KEYPAIR;

const TEST_NEURON_1_IDENTITY_PATH: &str = ".config/dfx/identity/test_neuron_1/identity.pem";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    let key_pair = &TEST_NEURON_1_OWNER_KEYPAIR;
    let path = home_dir()
        .ok_or(anyhow::anyhow!("No home dir present"))?
        .join(PathBuf::from_str(TEST_NEURON_1_IDENTITY_PATH)?);
    let dir = path.parent().ok_or(anyhow::anyhow!("No parent dir for path: {}", path.display()))?;
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }

    std::fs::write(path, key_pair.to_pem())?;

    // Take in one version and figure out what is the base version
    //
    // To find the initial version we could take NNS version?

    // Generate configuration for `ict` including the initial version
    //
    // We could take in a file and mutate it and copy it to /tmp folder

    // Run ict and capture its output
    //
    // Its important to parse the output correctly so we get the path to
    // log of the tool if something fails, on top of that we should
    // aggregate the output of the command which contains the json dump
    // of topology to parse it and get the nns urls and other links. Also
    // we have to extract the neuron pem file to use with dre

    // Run dre to qualify with correct parameters

    Ok(())
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Default logging level is: info generally, warn for mio and actix_server
                // You can override defaults by setting environment variables
                // RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info,mio::=warn,actix_server::=warn")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}
