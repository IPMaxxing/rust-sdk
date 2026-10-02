use ipmax::Client;
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

#[tokio::test]
async fn api_key_falls_back_to_the_environment() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": true,
            "data": {
                "currency": "CNY",
                "prices": [],
                "purchaseEmail": "sales@ipmax.example",
                "available": true
            }
        })))
        .mount(&server)
        .await;

    unsafe { std::env::remove_var("IPMAX_API_KEY") };
    let anonymous = Client::builder().base_url(server.uri()).build().unwrap();
    unsafe { std::env::set_var("IPMAX_API_KEY", "ipmax_env_key") };
    let from_env = Client::builder().base_url(server.uri()).build().unwrap();
    let explicit = Client::builder()
        .api_key("ipmax_explicit_key")
        .base_url(server.uri())
        .build()
        .unwrap();

    anonymous.catalog().await.unwrap();
    from_env.catalog().await.unwrap();
    explicit.catalog().await.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].headers.contains_key("authorization"));
    assert_eq!(requests[1].headers["authorization"], "Bearer ipmax_env_key");
    assert_eq!(
        requests[2].headers["authorization"],
        "Bearer ipmax_explicit_key"
    );
}
