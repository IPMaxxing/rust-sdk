use std::{
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use ipmax::Error;
use reqwest::StatusCode;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

use crate::support::{builder, direct, lookup, reply, unavailable};

#[tokio::test]
async fn retries_reuse_the_idempotency_key() {
    let server = MockServer::start().await;
    lookup("geoip", "8.8.8.8")
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    lookup("geoip", "8.8.8.8")
        .respond_with(reply("geoip-8-8-8-8"))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server).build().unwrap();
    let data = client.geoip("8.8.8.8").await.unwrap();
    assert_eq!(data.ip, "8.8.8.8");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].headers["idempotency-key"],
        requests[1].headers["idempotency-key"]
    );
}

#[tokio::test]
async fn retry_after_is_honoured() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(any())
        .respond_with(reply("catalog"))
        .mount(&server)
        .await;
    let client = builder(&server).build().unwrap();
    let started = Instant::now();
    client.catalog().await.unwrap();
    assert!(started.elapsed() >= Duration::from_secs(1));
}

#[tokio::test]
async fn retryable_statuses_are_retried() {
    for status in [408, 429, 500, 502, 503, 504] {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(status).insert_header("retry-after", "0"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(any())
            .respond_with(reply("account"))
            .mount(&server)
            .await;
        let client = builder(&server).build().unwrap();
        client.account().await.unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn retries_stop_after_max_retries() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(unavailable())
        .expect(3)
        .mount(&server)
        .await;
    let client = builder(&server).max_retries(2).build().unwrap();
    let Error::Api(error) = client.geoip("8.8.8.8").await.unwrap_err() else {
        panic!("expected an API error");
    };
    assert_eq!(error.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(error.retryable);
}

#[tokio::test]
async fn zero_retries_sends_once() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(unavailable())
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server).max_retries(0).build().unwrap();
    client.catalog().await.unwrap_err();
}

#[tokio::test]
async fn client_errors_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(reply("error-not-found"))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server).build().unwrap();
    client.geoip("203.0.113.9").await.unwrap_err();
}

#[tokio::test]
async fn network_errors_are_retried() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&accepted);
    thread::spawn(move || {
        for stream in listener.incoming() {
            counter.fetch_add(1, Ordering::SeqCst);
            drop(stream);
        }
    });
    let client = direct()
        .base_url(format!("http://{address}"))
        .max_retries(2)
        .build()
        .unwrap();
    assert!(matches!(
        client.catalog().await.unwrap_err(),
        Error::Http(_)
    ));
    assert_eq!(accepted.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn connection_failures_surface_as_http_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let client = direct()
        .base_url(format!("http://{address}"))
        .max_retries(0)
        .build()
        .unwrap();
    let Error::Http(error) = client.catalog().await.unwrap_err() else {
        panic!("expected a transport error");
    };
    assert!(error.is_connect());
}
