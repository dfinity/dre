use std::sync::Arc;

use ic_management_backend::{lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::Network;

use crate::{ctx::tests::get_mocked_ctx, ic_admin::MockIcAdmin};
#[test]
fn testing() {
    let mut ic_admin = MockIcAdmin::new();
    let mut registry = MockLazyRegistry::new();
    let mut git = MockLazyGit::new();
    let mut proposal_agent = MockProposalAgent::new();

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(git),
        Arc::new(proposal_agent),
    );
}
