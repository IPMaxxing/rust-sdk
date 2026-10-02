use std::time::Duration;

use ipmax::Error;
use uuid::{Uuid, Variant, Version};
use wiremock::{
    Mock, MockServer, Request,
    matchers::{any, header, method, path},
};

use crate::support::{API_KEY, builder, client, lookup, reply};

const USER_AGENT: &str = concat!("ipmax-rust/", env!("CARGO_PKG_VERSION"));

fn idempotency_key(request: &Request) -> Uuid {
    let value = request.headers["idempotency-key"].to_str().unwrap();
    Uuid::parse_str(value).unwrap()
}

#[tokio::test]
async fn lookups_post_the_ip_with_auth_and_a_uuid_v4_idempotency_key() {
    let server = MockServer::start().await;
    for (product, fixture) in [
        ("geoip", "geoip-8-8-8-8"),
        ("intelligence", "intelligence-8-8-8-8"),
    ] {
        lookup(product, "8.8.8.8")
            .and(header("authorization", format!("Bearer {API_KEY}")))
            .and(header("accept", "application/json"))
            .and(header("content-type", "application/json"))
            .and(header("user-agent", USER_AGENT))
            .respond_with(reply(fixture))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    client.geoip("8.8.8.8").await.unwrap();
    client.intelligence(" 8.8.8.8 ").await.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let key = idempotency_key(request);
        assert_eq!(key.get_version(), Some(Version::Random));
        assert_eq!(key.get_variant(), Variant::RFC4122);
    }
    assert_ne!(idempotency_key(&requests[0]), idempotency_key(&requests[1]));
}

#[tokio::test]
async fn each_logical_lookup_gets_a_fresh_idempotency_key() {
    let server = MockServer::start().await;
    lookup("geoip", "8.8.8.8")
        .respond_with(reply("geoip-8-8-8-8"))
        .expect(2)
        .mount(&server)
        .await;
    let client = builder(&server).cache_capacity(0).build().unwrap();
    client.geoip("8.8.8.8").await.unwrap();
    client.geoip("8.8.8.8").await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_ne!(idempotency_key(&requests[0]), idempotency_key(&requests[1]));
}

#[tokio::test]
async fn explicit_idempotency_keys_are_sent_verbatim() {
    let server = MockServer::start().await;
    lookup("geoip", "8.8.8.8")
        .and(header("idempotency-key", "order-42-geoip-8.8.8.8"))
        .respond_with(reply("geoip-8-8-8-8"))
        .expect(1)
        .mount(&server)
        .await;
    lookup("intelligence", "8.8.8.8")
        .and(header("idempotency-key", "order-42-intelligence-8.8.8.8"))
        .respond_with(reply("intelligence-8-8-8-8"))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    client
        .geoip_with_key("8.8.8.8", "order-42-geoip-8.8.8.8")
        .await
        .unwrap();
    client
        .intelligence_with_key("8.8.8.8", "order-42-intelligence-8.8.8.8")
        .await
        .unwrap();
}

#[tokio::test]
async fn catalog_and_account_are_plain_gets() {
    let server = MockServer::start().await;
    for (name, fixture) in [("catalog", "catalog"), ("account", "account")] {
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/{name}")))
            .and(header("authorization", format!("Bearer {API_KEY}")))
            .and(header("accept", "application/json"))
            .and(header("user-agent", USER_AGENT))
            .respond_with(reply(fixture))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    client.catalog().await.unwrap();
    client.account().await.unwrap();
    for request in server.received_requests().await.unwrap() {
        assert!(request.body.is_empty());
        assert!(!request.headers.contains_key("idempotency-key"));
    }
}

#[tokio::test]
async fn base_url_tolerates_a_trailing_slash() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/catalog"))
        .respond_with(reply("catalog"))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server)
        .base_url(format!("{}/", server.uri()))
        .build()
        .unwrap();
    client.catalog().await.unwrap();
}

#[tokio::test]
async fn timeout_applies_per_attempt() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(reply("catalog").set_delay(Duration::from_millis(500)))
        .mount(&server)
        .await;
    let client = builder(&server)
        .timeout(Duration::from_millis(50))
        .max_retries(0)
        .build()
        .unwrap();
    let Error::Http(error) = client.catalog().await.unwrap_err() else {
        panic!("expected a transport error");
    };
    assert!(error.is_timeout());
}

#[tokio::test]
async fn injected_http_client_is_used() {
    let server = MockServer::start().await;
    Mock::given(any())
        .and(header("x-proxy-tag", "edge"))
        .respond_with(reply("catalog"))
        .expect(1)
        .mount(&server)
        .await;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-proxy-tag", "edge".parse().unwrap());
    let http = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();
    let client = builder(&server).http_client(http).build().unwrap();
    client.catalog().await.unwrap();
}

#[tokio::test]
async fn client_futures_are_send() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(reply("geoip-8-8-8-8"))
        .mount(&server)
        .await;
    let client = client(&server);
    let data = tokio::spawn(async move { client.geoip("8.8.8.8").await })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data.ip, "8.8.8.8");
}
