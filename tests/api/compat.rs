use ipmax::{
    AnonymizerExemptionReason, AsnExchangeType, CpePart, Error, ErrorCode, LedgerEntryKind,
    LiveIntelligenceTag, NetworkClass, NetworkClassConfidence, NetworkClassScope,
    NetworkClassSource, Product, PtrDatasetName, PtrHintKind, PtrIntelligence, RevokedNetworkTag,
    RpkiRtrSource, RpkiSource, RpkiStatus,
};
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

use crate::support::{body, builder, client, lookup};

#[tokio::test]
async fn unknown_enum_values_and_fields_decode() {
    let mut body = body("intelligence-rich");
    let data = &mut body["data"];
    data["introduced_later"] = json!({ "nested": [1, 2, 3] });
    data["network_class"]["primary"] = json!("quantum_mesh");
    data["network_class"]["confidence"] = json!("certain");
    data["network_class"]["source"] = json!("oracle");
    data["network_class"]["conflicts"] = json!(["submarine"]);
    data["network_class"]["evidence"][0]["scope"] = json!("planet");
    data["as"]["connectivity"]["exchanges"][0]["type"] = json!("hybrid");
    data["as"]["connectivity"]["exchanges"][0]["ports"][0]["speed_class"] = json!("fast");
    data["rpki"]["status"] = json!("pending");
    data["rpki"]["source"] = json!("Remote Mirror");
    data["rpki"]["rtr"]["source"] = json!("NEWRTR");
    data["threat"]["anonymizer_exemptions"] = json!(["search_engine_crawler", "partner_vpn"]);
    let live = &mut data["intelligence"];
    live["tags"] = json!(["vpn", "ai_agent"]);
    live["revoked_tags"] = json!(["satellite_isp"]);
    live["cpes"][0]["part"] = json!("firmware");
    live["ptr"][0]["hint"]["kind"] = json!("geohash");
    live["ptr"][0]["evidence"]["sources"][0]["dataset"] = json!("openstreetmap");
    live["ptr"][0]["location"]["population"] = json!(37_000_000);
    live["ptr"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "hostname": "edge.example", "status": "teleported", "via": "wormhole" }));
    live["segment_probe"]["tags"] = json!(["residential_proxy", "honeypot"]);

    let server = MockServer::start().await;
    lookup("intelligence", "8.8.8.8")
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let data = client(&server).intelligence("8.8.8.8").await.unwrap();

    assert_eq!(data.network_class.primary, NetworkClass::Other);
    assert_eq!(data.network_class.confidence, NetworkClassConfidence::Other);
    assert_eq!(data.network_class.source, NetworkClassSource::Other);
    assert_eq!(data.network_class.conflicts, [NetworkClass::Other]);
    assert_eq!(
        data.network_class.evidence[0].scope,
        NetworkClassScope::Other
    );
    let connectivity = data.autonomous_system.connectivity.unwrap();
    assert_eq!(connectivity.exchanges[0].kind, AsnExchangeType::Other);
    let rpki = data.rpki.unwrap();
    assert_eq!(rpki.status, RpkiStatus::Other);
    assert_eq!(rpki.source, Some(RpkiSource::Other));
    assert_eq!(rpki.rtr.unwrap().source, RpkiRtrSource::Other);
    assert_eq!(
        data.threat.unwrap().anonymizer_exemptions.unwrap(),
        [
            AnonymizerExemptionReason::SearchEngineCrawler,
            AnonymizerExemptionReason::Other
        ]
    );

    let live = data.intelligence;
    assert_eq!(
        live.tags,
        [LiveIntelligenceTag::Vpn, LiveIntelligenceTag::Other]
    );
    assert_eq!(live.revoked_tags, [RevokedNetworkTag::Other]);
    assert_eq!(live.cpes[0].part, CpePart::Other);
    let PtrIntelligence::Located(located) = &live.ptr[0] else {
        panic!("first PTR is not located");
    };
    assert_eq!(located.hint.kind, PtrHintKind::Other);
    assert_eq!(located.evidence.sources[0].dataset, PtrDatasetName::Other);
    assert_eq!(live.ptr[3], PtrIntelligence::Other);
    assert_eq!(
        live.segment_probe.flatten().unwrap().tags,
        [
            LiveIntelligenceTag::ResidentialProxy,
            LiveIntelligenceTag::Other
        ]
    );
}

#[tokio::test]
async fn unknown_products_and_ledger_kinds_decode() {
    let mut body = body("account");
    body["data"]["wallets"][1]["product"] = json!("bundle");
    body["data"]["entries"] = json!([{
        "id": "entry-9",
        "product": "bundle",
        "kind": "refund",
        "amountMicros": -437.5,
        "balanceMicros": 1_000_437.5,
        "reference": "refund-9",
        "createdAt": "2026-09-30T00:00:00.000Z",
        "operator": "support"
    }]);
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let account = client(&server).account().await.unwrap();
    assert_eq!(account.wallets[0].product, Product::GeoIp);
    assert_eq!(account.wallets[1].product, Product::Other);
    assert_eq!(account.entries[0].product, Product::Other);
    assert_eq!(account.entries[0].kind, LedgerEntryKind::Other);
    assert_eq!(account.entries[0].amount_micros, -437.5);
}

#[tokio::test]
async fn unknown_error_codes_stay_opaque() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "status": false,
            "data": null,
            "error": {
                "code": 4242,
                "message": "Something new",
                "request_id": "ENT-new",
                "retryable": false,
                "hint": "added later"
            }
        })))
        .mount(&server)
        .await;
    let client = builder(&server).max_retries(0).build().unwrap();
    let Error::Api(error) = client.geoip("8.8.8.8").await.unwrap_err() else {
        panic!("expected an API error");
    };
    assert_eq!(error.code, Some(ErrorCode(4242)));
    assert_eq!(error.message, "Something new");
}
