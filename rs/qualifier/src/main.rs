#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

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
