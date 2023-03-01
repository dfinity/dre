use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use ic_types::PrincipalId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone, Debug, strum::Display)]
pub enum NetworkError {
    NodeNotFound(PrincipalId),
    SubnetNotFound(PrincipalId),
    ExtensionFailed(String),
    DataRequestError,
    IllegalRequest(String),
}

impl ResponseError for NetworkError {
    fn error_response(&self) -> HttpResponse {
        match self {
            NetworkError::IllegalRequest(_input) => HttpResponse::build(StatusCode::BAD_REQUEST).json(self),
            NetworkError::ExtensionFailed(_) => HttpResponse::InternalServerError().json(self),
            NetworkError::DataRequestError => HttpResponse::build(StatusCode::FAILED_DEPENDENCY).json(self),
            NetworkError::SubnetNotFound(_) | NetworkError::NodeNotFound(_) => HttpResponse::NotFound().json(self),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::NodeNotFound(_) | Self::SubnetNotFound(_) => StatusCode::NOT_FOUND,
            Self::IllegalRequest(_) => StatusCode::BAD_REQUEST,
            Self::ExtensionFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DataRequestError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for NetworkError {
    fn from(_: reqwest::Error) -> NetworkError {
        NetworkError::DataRequestError
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(_: serde_json::Error) -> NetworkError {
        NetworkError::DataRequestError
    }
}
