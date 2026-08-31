mod cordoned_feature_fetcher;
// The DRE context unit tests have been moved to a submodule
// of the ctx module.  This was accomplished to reduce the
// visibility of methods of ctx structs.
mod add_nodes;
mod args_parse;
mod health_client;
mod node_labels;
mod registry_versions;
mod replace;
mod update_unassigned_nodes;
mod version;

/// Installs the process-level rustls `CryptoProvider` (aws-lc-rs) for tests.
///
/// The `dre` binary installs this in `main()`, but unit tests do not go through
/// `main()`. Any test that ends up constructing a rustls TLS client (e.g. via
/// `reqwest`) would otherwise panic because rustls cannot automatically choose
/// between the multiple enabled providers. Idempotent and safe to call from
/// multiple tests/threads.
pub(crate) fn install_default_crypto_provider() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}
