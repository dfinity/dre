//! Authentication module for Internet Identity integration

pub mod ii_delegation;
pub mod middleware;

pub use ii_delegation::{DelegationChain, verify_delegation};
pub use middleware::{AuthError, Session};
