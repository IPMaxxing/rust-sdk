use std::{fs, path::PathBuf};

use ipmax::{Client, ClientBuilder};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};

pub const API_KEY: &str = "ipmax_test_key";

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixtures_dir())
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            path.file_stem().unwrap().to_str().unwrap().to_owned()
        })
        .collect();
    names.sort();
    names
}

pub fn fixture(name: &str) -> Value {
    let bytes = fs::read(fixtures_dir().join(format!("{name}.json"))).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

pub fn body(name: &str) -> Value {
    fixture(name)["body"].clone()
}

pub fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

pub fn reply(name: &str) -> ResponseTemplate {
    let fixture = fixture(name);
    let status = u16::try_from(fixture["status"].as_u64().unwrap()).unwrap();
    fixture["headers"].as_object().unwrap().iter().fold(
        ResponseTemplate::new(status).set_body_json(&fixture["body"]),
        |template, (name, value)| template.insert_header(name.as_str(), value.as_str().unwrap()),
    )
}

pub fn unavailable() -> ResponseTemplate {
    ResponseTemplate::new(503)
        .insert_header("retry-after", "0")
        .set_body_json(json!({
            "status": false,
            "data": null,
            "error": {
                "code": 1400,
                "message": "Service unavailable",
                "request_id": "ENT-unavailable",
                "retryable": true
            }
        }))
}

pub fn lookup(product: &str, ip: &str) -> wiremock::MockBuilder {
    Mock::given(method("POST"))
        .and(path(format!("/api/v1/{product}")))
        .and(body_json(json!({ "ip": ip })))
}

pub fn direct() -> ClientBuilder {
    Client::builder()
        .api_key(API_KEY)
        .http_client(reqwest::Client::builder().no_proxy().build().unwrap())
}

pub fn builder(server: &MockServer) -> ClientBuilder {
    direct().base_url(server.uri())
}

pub fn client(server: &MockServer) -> Client {
    builder(server).build().unwrap()
}
