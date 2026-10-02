use std::time::Duration;

use ipmax::{ApiError, Client, Error, ErrorCode};
use reqwest::StatusCode;
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

use crate::support::{builder, reply};

async fn api_error(response: ResponseTemplate, call: Call) -> ApiError {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(response)
        .mount(&server)
        .await;
    let client = builder(&server).max_retries(0).build().unwrap();
    match call.run(&client).await.unwrap_err() {
        Error::Api(error) => error,
        other => panic!("expected an API error, got {other:?}"),
    }
}

enum Call {
    Account,
    GeoIp,
    Intelligence,
}

impl Call {
    async fn run(self, client: &Client) -> Result<(), Error> {
        match self {
            Self::Account => client.account().await.map(drop),
            Self::GeoIp => client.geoip("203.0.113.9").await.map(drop),
            Self::Intelligence => client.intelligence("203.0.113.9").await.map(drop),
        }
    }
}

#[tokio::test]
async fn not_found_maps_every_field() {
    let error = api_error(reply("error-not-found"), Call::GeoIp).await;
    assert_eq!(
        error,
        ApiError {
            status: StatusCode::NOT_FOUND,
            code: Some(ErrorCode::IP_NOT_FOUND),
            message: "This IP address is not in the database".to_owned(),
            request_id: "ENT-f1b619f1-4da9-48c6-bb41-8ac819ea863a".to_owned(),
            retryable: false,
            retry_after: None,
        }
    );
    assert_eq!(
        error.to_string(),
        "404 Not Found: This IP address is not in the database"
    );
}

#[tokio::test]
async fn unauthorized_maps_to_invalid_api_key() {
    let error = api_error(reply("error-unauthorized"), Call::Account).await;
    assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    assert_eq!(error.code, Some(ErrorCode::INVALID_API_KEY));
    assert_eq!(error.message, "Invalid API key");
    assert_eq!(error.request_id, "ENT-706ff971-cb85-4933-8b6a-c4e42e0ecce1");
}

#[tokio::test]
async fn invalid_request_maps_to_bad_request() {
    let error = api_error(reply("error-invalid-request"), Call::Intelligence).await;
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.code, Some(ErrorCode::INVALID_REQUEST));
    assert!(!error.retryable);
}

#[tokio::test]
async fn rate_limits_expose_retry_after() {
    let response = ResponseTemplate::new(429)
        .insert_header("retry-after", "7")
        .set_body_json(json!({
            "status": false,
            "data": null,
            "error": {
                "code": 1011,
                "message": "Rate limit exceeded",
                "request_id": "ENT-rate",
                "retryable": true
            }
        }));
    let error = api_error(response, Call::GeoIp).await;
    assert_eq!(error.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(error.code, Some(ErrorCode::RATE_LIMIT_EXCEEDED));
    assert!(error.retryable);
    assert_eq!(error.retry_after, Some(Duration::from_secs(7)));
}

#[tokio::test]
async fn non_json_error_bodies_still_become_api_errors() {
    let response = ResponseTemplate::new(502)
        .insert_header("x-request-id", "ENT-gateway")
        .set_body_string("<html>Bad gateway</html>");
    let error = api_error(response, Call::GeoIp).await;
    assert_eq!(
        error,
        ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: None,
            message: "Bad Gateway".to_owned(),
            request_id: "ENT-gateway".to_owned(),
            retryable: true,
            retry_after: None,
        }
    );
}

#[tokio::test]
async fn unexpected_json_error_bodies_keep_the_status() {
    let response = ResponseTemplate::new(418).set_body_json(json!({ "detail": "teapot" }));
    let error = api_error(response, Call::Account).await;
    assert_eq!(error.status, StatusCode::IM_A_TEAPOT);
    assert_eq!(error.code, None);
    assert!(!error.retryable);
}

#[tokio::test]
async fn malformed_success_bodies_are_decode_errors() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;
    let client = builder(&server).build().unwrap();
    assert!(matches!(
        client.catalog().await.unwrap_err(),
        Error::Decode(_)
    ));
}
