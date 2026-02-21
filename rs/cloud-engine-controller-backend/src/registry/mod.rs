//! Registry integration module for ICP node data

pub mod node_mapping;
pub mod sync;

pub use node_mapping::NodeMapper;
pub use sync::RegistryManager;
