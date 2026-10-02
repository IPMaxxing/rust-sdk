use std::time::Duration;

use reqwest::{
    StatusCode,
    header::{HeaderMap, RETRY_AFTER},
};
use serde::Deserialize;

use crate::ErrorResponse;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
}

impl Error {
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            Self::Api(error) => is_retryable_status(error.status),
            Self::Http(error) => !error.is_builder(),
            Self::Decode(_) => false,
        }
    }

    pub(crate) fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Api(error) => error.retry_after,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, thiserror::Error)]
#[error("{status}: {message}")]
pub struct ApiError {
    #[serde(skip)]
    pub status: StatusCode,
    pub code: Option<ErrorCode>,
    pub message: String,
    pub request_id: String,
    pub retryable: bool,
    #[serde(skip)]
    pub retry_after: Option<Duration>,
}

impl ApiError {
    pub(crate) fn from_response(status: StatusCode, headers: &HeaderMap, body: &[u8]) -> Self {
        let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
        let retry_after = header(RETRY_AFTER.as_str())
            .and_then(|value| value.trim().parse().ok())
            .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
        let error = serde_json::from_slice::<ErrorResponse>(body).map_or_else(
            |_| Self {
                status,
                code: None,
                message: status
                    .canonical_reason()
                    .unwrap_or("Unexpected response")
                    .to_owned(),
                request_id: header("x-request-id").unwrap_or_default().to_owned(),
                retryable: is_retryable_status(status),
                retry_after: None,
            },
            |response| response.error,
        );
        Self {
            status,
            retry_after,
            ..error
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct ErrorCode(pub u32);

impl ErrorCode {
    pub const INVALID_REQUEST: Self = Self(1000);
    pub const INVALID_PAYLOAD: Self = Self(1001);
    pub const INVALID_IP: Self = Self(1002);
    pub const IP_NOT_FOUND: Self = Self(1003);
    pub const ROUTE_NOT_FOUND: Self = Self(1004);
    pub const INVALID_API_KEY: Self = Self(1010);
    pub const RATE_LIMIT_EXCEEDED: Self = Self(1011);
    pub const SERVICE_UNAVAILABLE: Self = Self(1400);
    pub const INTERNAL_ERROR: Self = Self(1401);
    pub const INSUFFICIENT_BALANCE: Self = Self(1500);
    pub const IDEMPOTENCY_CONFLICT: Self = Self(1501);
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}
