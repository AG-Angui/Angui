use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use sea_orm::DbErr;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("database operation failed")]
    Database(#[from] DbErr),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let code = match self {
            Self::Validation(_) => "validation_error",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Database(_) => "database_error",
        };

        HttpResponse::build(self.status_code()).json(ErrorResponse {
            error: ErrorDetail {
                code,
                message: self.to_string(),
            },
        })
    }
}
