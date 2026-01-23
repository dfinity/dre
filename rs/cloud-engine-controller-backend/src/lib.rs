//! Cloud Engine Controller Backend
//!
//! A backend service for managing GCP VMs and their association with ICP nodes.
//! Supports Internet Identity authentication and subnet management via NNS proposals.

pub mod auth;
pub mod gcp;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod registry;
pub mod state;

pub use state::AppState;
