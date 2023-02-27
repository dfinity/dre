pub mod nodes;
pub mod query_decentralization;
pub mod subnet;

use crate::registry::RegistryState;
use actix_web::{error, get, post, web, Error, HttpResponse, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::Serialize;

fn response_from_result<T: Serialize, E: std::fmt::Debug + std::fmt::Display + 'static>(
    result: Result<T, E>,
) -> Result<HttpResponse, Error> {
    match result {
        Ok(data) => Ok(HttpResponse::Ok().json(data)),
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}
