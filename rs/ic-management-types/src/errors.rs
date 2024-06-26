use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use ic_types::PrincipalId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone, Debug, strum_macros::Display)]
pub enum NetworkError {
    NodeNotFound(PrincipalId),
    SubnetNotFound(PrincipalId),
    ResizeFailed(String),
    DataRequestError(String),
    IllegalRequest(String),
}

impl ResponseError for NetworkError {
    fn error_response(&self) -> HttpResponse {
        match self {
            NetworkError::IllegalRequest(_input) => HttpResponse::build(StatusCode::BAD_REQUEST).json(self),
            NetworkError::ResizeFailed(_) => HttpResponse::InternalServerError().json(self),
            NetworkError::DataRequestError(_) => HttpResponse::build(StatusCode::FAILED_DEPENDENCY).json(self),
            NetworkError::SubnetNotFound(_) | NetworkError::NodeNotFound(_) => HttpResponse::NotFound().json(self),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::NodeNotFound(_) | Self::SubnetNotFound(_) => StatusCode::NOT_FOUND,
            Self::IllegalRequest(_) => StatusCode::BAD_REQUEST,
            Self::ResizeFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DataRequestError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for NetworkError {
    fn from(err: reqwest::Error) -> NetworkError {
        NetworkError::DataRequestError(err.to_string())
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> NetworkError {
        NetworkError::DataRequestError(err.to_string())
    }
}

impl From<NetworkError> for anyhow::Error {
    fn from(e: NetworkError) -> Self {
        match e {
            NetworkError::NodeNotFound(id) => anyhow::anyhow!("Node not found: {:?}", id),
            NetworkError::SubnetNotFound(id) => anyhow::anyhow!("Subnet not found: {:?}", id),
            NetworkError::ResizeFailed(msg) => anyhow::anyhow!("Resize Failed: {:?}", msg),
            NetworkError::DataRequestError(msg) => anyhow::anyhow!("Data request error: {:?}", msg),
            NetworkError::IllegalRequest(msg) => anyhow::anyhow!("Illegal request: {:?}", msg),
        }
    }
}
