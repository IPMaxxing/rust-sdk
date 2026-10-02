use std::time::Duration;

use wiremock::{
    Mock, MockServer,
    matchers::{any, method, path},
};

use crate::support::{builder, client, lookup, reply};

const GOOGLE: &str = "8.8.8.8";
const PRIVATE: &str = "10.1.2.3";
const TELECOM: &str = "240e:390:a1:3cd0:be24:11ff:fe46:aca3";

async fn mount_geoip(server: &MockServer, ip: &str, fixture: &str, times: u64) {
    lookup("geoip", ip)
        .respond_with(reply(fixture))
        .expect(times)
        .mount(server)
        .await;
}

#[tokio::test]
async fn repeated_lookups_hit_the_server_once_per_product() {
    let server = MockServer::start().await;
    mount_geoip(&server, GOOGLE, "geoip-8-8-8-8", 1).await;
    lookup("intelligence", GOOGLE)
        .respond_with(reply("intelligence-8-8-8-8"))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    let first = client.geoip(GOOGLE).await.unwrap();
    assert_eq!(client.geoip(GOOGLE).await.unwrap(), first);
    assert_eq!(client.geoip(" 8.8.8.8\n").await.unwrap(), first);
    client.intelligence(GOOGLE).await.unwrap();
    client.intelligence(GOOGLE).await.unwrap();
}

#[tokio::test]
async fn cache_is_shared_between_clones() {
    let server = MockServer::start().await;
    mount_geoip(&server, GOOGLE, "geoip-8-8-8-8", 1).await;
    let client = client(&server);
    client.geoip(GOOGLE).await.unwrap();
    client.clone().geoip(GOOGLE).await.unwrap();
}

#[tokio::test]
async fn expired_entries_are_refetched() {
    let server = MockServer::start().await;
    mount_geoip(&server, GOOGLE, "geoip-8-8-8-8", 2).await;
    let client = builder(&server)
        .cache_ttl(Duration::from_millis(50))
        .build()
        .unwrap();
    client.geoip(GOOGLE).await.unwrap();
    client.geoip(GOOGLE).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    client.geoip(GOOGLE).await.unwrap();
}

#[tokio::test]
async fn least_recently_used_entry_is_evicted() {
    let server = MockServer::start().await;
    mount_geoip(&server, GOOGLE, "geoip-8-8-8-8", 1).await;
    mount_geoip(&server, PRIVATE, "geoip-10-1-2-3", 2).await;
    mount_geoip(
        &server,
        TELECOM,
        "geoip-240e-390-a1-3cd0-be24-11ff-fe46-aca3",
        1,
    )
    .await;
    let client = builder(&server).cache_capacity(2).build().unwrap();
    client.geoip(GOOGLE).await.unwrap();
    client.geoip(PRIVATE).await.unwrap();
    client.geoip(GOOGLE).await.unwrap();
    client.geoip(TELECOM).await.unwrap();
    client.geoip(GOOGLE).await.unwrap();
    client.geoip(PRIVATE).await.unwrap();
}

#[tokio::test]
async fn zero_capacity_disables_the_cache() {
    let server = MockServer::start().await;
    mount_geoip(&server, GOOGLE, "geoip-8-8-8-8", 2).await;
    let client = builder(&server).cache_capacity(0).build().unwrap();
    client.geoip(GOOGLE).await.unwrap();
    client.geoip(GOOGLE).await.unwrap();
}

#[tokio::test]
async fn errors_are_not_cached() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(reply("error-not-found"))
        .expect(2)
        .mount(&server)
        .await;
    let client = client(&server);
    client.geoip(GOOGLE).await.unwrap_err();
    client.geoip(GOOGLE).await.unwrap_err();
}

#[tokio::test]
async fn explicit_idempotency_key_bypasses_the_read_but_stores_the_result() {
    let server = MockServer::start().await;
    mount_geoip(&server, GOOGLE, "geoip-8-8-8-8", 2).await;
    let client = client(&server);
    client
        .geoip_with_key(GOOGLE, "order-1-geoip-8.8.8.8")
        .await
        .unwrap();
    client.geoip(GOOGLE).await.unwrap();
    client
        .geoip_with_key(GOOGLE, "order-2-geoip-8.8.8.8")
        .await
        .unwrap();
}

#[tokio::test]
async fn catalog_and_account_are_never_cached() {
    let server = MockServer::start().await;
    for name in ["catalog", "account"] {
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/{name}")))
            .respond_with(reply(name))
            .expect(2)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    client.catalog().await.unwrap();
    client.catalog().await.unwrap();
    client.account().await.unwrap();
    client.account().await.unwrap();
}
