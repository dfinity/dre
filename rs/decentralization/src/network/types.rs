use super::*;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use std::fmt::Debug;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DataCenterInfo {
    area: String,
    country: String,
    continent: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CordonedFeature {
    pub feature: NodeFeature,
    pub value: String,
    pub explanation: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, strum_macros::Display)]
pub enum DecentralizationError {
    FeatureNotAvailable,
}

impl ResponseError for DecentralizationError {
    fn error_response(&self) -> HttpResponse {
        let out: serde_json::Value =
            serde_json::from_str("{\"message\": \"NodeFeature not available. For access contact the administrator\"}").unwrap();
        HttpResponse::BadRequest().json(out)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::FeatureNotAvailable => StatusCode::BAD_REQUEST,
        }
    }
}
