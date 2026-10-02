use ipmax::{
    AccountResponse, ApiError, AsnExchangeType, CatalogResponse, CpePart, Error, ErrorCode,
    ErrorResponse, GeoIpResponse, IntelligenceResponse, LedgerEntryKind, LiveIntelligenceTag,
    NetworkClass, NetworkClassConfidence, NetworkClassScope, NetworkClassSource, Product,
    PtrConfidenceLevel, PtrDatasetName, PtrHintKind, PtrIntelligence, RevokedNetworkTag,
    RpkiRtrSource, RpkiSource, RpkiStatus,
};
use reqwest::StatusCode;
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

use crate::support::{body, builder, client, decode, fixture, fixture_names, reply};

#[tokio::test]
async fn every_fixture_replays_through_the_client() {
    let names = fixture_names();
    assert_eq!(names.len(), 12);
    for name in names {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(reply(&name))
            .mount(&server)
            .await;
        let client = builder(&server)
            .max_retries(0)
            .cache_capacity(0)
            .build()
            .unwrap();
        let body = body(&name);
        let ip = body["data"]["ip"].as_str().unwrap_or("8.8.8.8");
        match name.split('-').next().unwrap() {
            "catalog" => {
                let envelope: CatalogResponse = decode(&body);
                assert!(envelope.status);
                assert_eq!(client.catalog().await.unwrap(), envelope.data);
            }
            "account" => {
                let envelope: AccountResponse = decode(&body);
                assert!(envelope.status);
                assert_eq!(client.account().await.unwrap(), envelope.data);
            }
            "geoip" => {
                let envelope: GeoIpResponse = decode(&body);
                assert!(envelope.status);
                assert_eq!(client.geoip(ip).await.unwrap(), envelope.data);
            }
            "intelligence" => {
                let envelope: IntelligenceResponse = decode(&body);
                assert!(envelope.status);
                assert_eq!(client.intelligence(ip).await.unwrap(), envelope.data);
            }
            "error" => {
                let envelope: ErrorResponse = decode(&body);
                assert!(!envelope.status);
                let Error::Api(error) = client.geoip(ip).await.unwrap_err() else {
                    panic!("{name} did not produce an API error");
                };
                let status = u16::try_from(fixture(&name)["status"].as_u64().unwrap()).unwrap();
                assert_eq!(
                    error,
                    ApiError {
                        status: StatusCode::from_u16(status).unwrap(),
                        retry_after: None,
                        ..envelope.error
                    }
                );
            }
            other => panic!("unhandled fixture kind {other}"),
        }
    }
}

#[tokio::test]
async fn catalog_carries_fractional_prices() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(reply("catalog"))
        .mount(&server)
        .await;
    let catalog = client(&server).catalog().await.unwrap();
    assert_eq!(catalog.currency, "CNY");
    assert!(catalog.available);
    assert_eq!(catalog.purchase_email, "sales@ipmax.example");
    assert_eq!(catalog.prices[0].product, Product::GeoIp);
    assert_eq!(catalog.prices[0].unit_micros, 437.5);
    assert_eq!(catalog.prices[1].product, Product::Intelligence);
    assert_eq!(catalog.prices[1].unit_micros, 1920.0);
}

#[tokio::test]
async fn account_carries_wallets_and_fractional_ledger_amounts() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(reply("account"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let mut charged = body("account");
    charged["data"]["wallets"][0]["balanceMicros"] = json!(999_562.5);
    charged["data"]["entries"] = json!([{
        "id": "entry-1",
        "product": "geoip",
        "kind": "charge",
        "amountMicros": 437.5,
        "balanceMicros": 999_562.5,
        "reference": "receipt-1",
        "createdAt": "2026-09-29T08:30:00.000Z"
    }]);
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_json(charged))
        .mount(&server)
        .await;
    let client = client(&server);

    let account = client.account().await.unwrap();
    assert_eq!(account.id, "00000000-0000-4000-8000-000000000042");
    assert_eq!(account.key_prefix, "ipmax_test");
    assert_eq!(account.wallets[1].product, Product::Intelligence);
    assert_eq!(account.wallets[1].balance_micros, 1_000_000.0);
    assert!(account.entries.is_empty());

    let account = client.account().await.unwrap();
    assert_eq!(account.wallets[0].balance_micros, 999_562.5);
    let entry = &account.entries[0];
    assert_eq!(entry.kind, LedgerEntryKind::Charge);
    assert_eq!(entry.amount_micros, 437.5);
    assert_eq!(entry.created_at, "2026-09-29T08:30:00.000Z");
}

#[test]
fn geoip_decodes_country_enrichment() {
    let data = decode::<GeoIpResponse>(&body("geoip-8-8-8-8")).data;
    assert_eq!(data.autonomous_system.asn, "AS15169");
    assert_eq!(data.autonomous_system.country_code, None);
    assert_eq!(data.geo.country_code, "US");
    assert_eq!(data.geo.country_zh.as_deref(), Some("美国"));
    assert_eq!(data.geo.city_en, None);
    assert_eq!(data.geo.latitude, Some(37.4056));
    assert_eq!(data.geo.radius, Some(25.0));
    assert_eq!(data.prefix_length, Some(24));
    let currency = data.currency.flatten().unwrap();
    assert_eq!(currency.code, "USD");
    assert_eq!(currency.symbol, "$");
    let time_zone = data.time_zone.flatten().unwrap();
    assert_eq!(time_zone.abbr, "PDT");
    assert!(time_zone.is_dst);
    assert_eq!(data.calling_code, Some(Some("1".to_owned())));
}

#[test]
fn geoip_distinguishes_null_from_absent() {
    let bogon = decode::<GeoIpResponse>(&body("geoip-10-1-2-3")).data;
    assert!(bogon.is_bogon);
    assert_eq!(bogon.currency, Some(None));
    assert_eq!(bogon.time_zone, Some(None));
    assert_eq!(bogon.calling_code, Some(None));

    let mut trimmed = body("geoip-10-1-2-3");
    let data = trimmed["data"].as_object_mut().unwrap();
    data.remove("currency");
    data.remove("time_zone");
    data.remove("calling_code");
    let trimmed = decode::<GeoIpResponse>(&trimmed).data;
    assert_eq!(trimmed.currency, None);
    assert_eq!(trimmed.time_zone, None);
    assert_eq!(trimmed.calling_code, None);
}

#[test]
fn geoip_decodes_ipv6() {
    let data = decode::<GeoIpResponse>(&body("geoip-240e-390-a1-3cd0-be24-11ff-fe46-aca3")).data;
    assert_eq!(data.ip, "240e:390:a1:3cd0:be24:11ff:fe46:aca3");
    assert_eq!(data.netmask, "240e:390::/32");
    assert_eq!(data.time_zone.flatten().unwrap().abbr, "GMT+8");
}

#[test]
fn intelligence_decodes_the_rich_fixture() {
    let data = decode::<IntelligenceResponse>(&body("intelligence-rich")).data;
    let live = &data.intelligence;

    let PtrIntelligence::Located(located) = &live.ptr[0] else {
        panic!("first PTR is not located");
    };
    assert_eq!(located.hostname, "ae-1.r01.tokyjp05.jp.bb.gin.ntt.net");
    assert_eq!(located.operator.asn, "AS2914");
    assert_eq!(located.hint.kind, PtrHintKind::Clli);
    assert_eq!(located.location.city, "Tokyo");
    assert_eq!(located.location.region_code.as_deref(), Some("13"));
    assert_eq!(located.location.latitude, 35.6895);
    assert_eq!(located.location.precision, "metro");
    assert_eq!(located.confidence.level, PtrConfidenceLevel::High);
    assert_eq!(located.confidence.score, 0.92);
    assert_eq!(
        located.evidence.sources[0].dataset,
        PtrDatasetName::Geonames
    );

    let PtrIntelligence::AsnMismatch(mismatch) = &live.ptr[1] else {
        panic!("second PTR is not an ASN mismatch");
    };
    assert_eq!(mismatch.observed_asn, "AS64501");
    assert!(mismatch.evidence.sources.is_empty());

    let PtrIntelligence::Unmatched(unmatched) = &live.ptr[2] else {
        panic!("third PTR is not unmatched");
    };
    assert_eq!(unmatched.hostname, "dns.google");

    let transfer = &live.transfers.as_ref().unwrap()[0];
    assert_eq!(transfer.source_holder, "Level 3");
    assert_eq!(transfer.recipient_holder, "Google LLC");

    let probe = live.segment_probe.clone().flatten().unwrap();
    assert_eq!(probe.ip, "8.8.8.1");
    assert_eq!(probe.ports, [53, 443]);
    assert_eq!(probe.tags, [LiveIntelligenceTag::PublicDnsResolver]);

    let abuse = live.abuse.clone().flatten().unwrap();
    assert_eq!(abuse.confidence_score, 12);
    assert_eq!(abuse.is_whitelisted, Some(false));
    assert_eq!(abuse.categories[0].id, 18);

    assert_eq!(live.cpes[0].part, CpePart::Application);
    assert_eq!(live.cpes[0].version.as_deref(), Some("1.25.3"));
    assert_eq!(
        live.tags,
        [
            LiveIntelligenceTag::PublicDnsResolver,
            LiveIntelligenceTag::WebServer
        ]
    );
    assert_eq!(live.revoked_tags, [RevokedNetworkTag::HomeIsp]);
    assert_eq!(live.usage_type.as_deref(), Some("DCH"));

    let connectivity = data.autonomous_system.connectivity.as_ref().unwrap();
    assert_eq!(connectivity.exchanges.len(), 30);
    assert_eq!(connectivity.exchanges_total, None);
    assert_eq!(connectivity.estimated_capacity.lower_gbps, 3000.0);
    assert_eq!(connectivity.estimated_capacity.upper_gbps, None);
    assert_eq!(
        connectivity.exchanges[1].country_code.as_deref(),
        Some("DE")
    );
    assert_eq!(connectivity.exchanges[0].kind, AsnExchangeType::Physical);
    assert_eq!(connectivity.exchanges[0].ports[0].rs_peer, Some(true));

    let rpki = data.rpki.as_ref().unwrap();
    assert_eq!(rpki.status, RpkiStatus::Valid);
    assert_eq!(rpki.source, Some(RpkiSource::RtrLive));
    assert_eq!(rpki.route_source.as_deref(), Some("Local DB"));
    assert_eq!(rpki.max_length, Some(24));
    let rtr = rpki.rtr.as_ref().unwrap();
    assert_eq!(rtr.source, RpkiRtrSource::Cloudflare);
    assert_eq!(rtr.last_updated, 1_790_000_000);

    let class = &data.network_class;
    assert_eq!(class.primary, NetworkClass::Hosting);
    assert_eq!(class.confidence, NetworkClassConfidence::Medium);
    assert_eq!(class.source, NetworkClassSource::NetworkTag);
    assert_eq!(class.evidence[0].scope, NetworkClassScope::Prefix);

    let threat = data.threat.as_ref().unwrap();
    assert_eq!(threat.is_known_abuser, Some(true));
    assert_eq!(threat.blocklists[0].kind, "known_abuser");
    assert_eq!(threat.scores.threat_score, Some(35.0));
    assert_eq!(threat.anonymizer_exemptions, None);
    assert_eq!(data.is_hosting, Some(true));
}

#[test]
fn intelligence_decodes_a_reserved_address() {
    let data = decode::<IntelligenceResponse>(&body("intelligence-10-1-2-3")).data;
    assert!(data.is_bogon);
    assert_eq!(data.network, None);
    assert_eq!(data.rpki, None);
    let threat = data.threat.unwrap();
    assert_eq!(threat.is_tor, None);
    assert_eq!(threat.scores.trust_score, None);
    assert_eq!(data.intelligence.usage_type, None);
    assert_eq!(data.intelligence.abuse, None);
    assert_eq!(data.intelligence.segment_probe, None);
    assert_eq!(data.network_class.source, NetworkClassSource::AsnTag);
}

#[test]
fn error_code_constants_match_the_wire() {
    let not_found: ErrorResponse = decode(&body("error-not-found"));
    assert_eq!(not_found.error.code, Some(ErrorCode::IP_NOT_FOUND));
    let unauthorized: ErrorResponse = decode(&body("error-unauthorized"));
    assert_eq!(unauthorized.error.code, Some(ErrorCode::INVALID_API_KEY));
    let invalid: ErrorResponse = decode(&body("error-invalid-request"));
    assert_eq!(invalid.error.code, Some(ErrorCode::INVALID_REQUEST));
}
