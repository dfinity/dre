//! Cloud Engine Controller Backend
//!
//! A backend service for managing GCP VMs and their association with ICP nodes.
//! Supports subnet management via NNS proposals.

pub mod config;
pub mod gcp;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod registry;
pub mod state;

pub use config::AppConfig;
pub use state::AppState;
