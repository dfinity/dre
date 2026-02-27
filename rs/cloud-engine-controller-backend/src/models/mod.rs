//! Data models for the Cloud Engine Controller Backend

pub mod subnet;
pub mod vm;

pub use subnet::{SubnetInfo, SubnetProposal, SubnetProposalRequest};
pub use vm::{IcpNodeMapping, Vm, VmProvisionRequest, VmStatus};
