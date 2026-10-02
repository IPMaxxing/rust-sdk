use serde::{Deserialize, Deserializer, Serialize};

use crate::ApiError;

fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Envelope<T> {
    pub status: bool,
    pub data: T,
}

pub type CatalogResponse = Envelope<Catalog>;
pub type AccountResponse = Envelope<Account>;
pub type GeoIpResponse = Envelope<GeoIpData>;
pub type IntelligenceResponse = Envelope<IntelligenceData>;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ErrorResponse {
    pub status: bool,
    pub error: ApiError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LookupRequest {
    pub ip: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Product {
    #[serde(rename = "geoip")]
    GeoIp,
    #[serde(rename = "intelligence")]
    Intelligence,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub currency: String,
    pub prices: Vec<Price>,
    pub purchase_email: String,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub product: Product,
    pub name: String,
    pub unit_micros: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub wallets: Vec<Wallet>,
    pub entries: Vec<LedgerEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub product: Product,
    pub balance_micros: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub id: String,
    pub product: Product,
    pub kind: LedgerEntryKind,
    pub amount_micros: f64,
    pub balance_micros: f64,
    pub reference: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryKind {
    Credit,
    Charge,
    Adjustment,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GeoIpData {
    #[serde(rename = "as")]
    pub autonomous_system: GeoIpAs,
    pub ip: String,
    pub geo: Geolocation,
    pub netmask: String,
    pub prefix_length: Option<u8>,
    pub is_bogon: bool,
    #[serde(default, deserialize_with = "nullable")]
    pub currency: Option<Option<Currency>>,
    #[serde(default, deserialize_with = "nullable")]
    pub time_zone: Option<Option<TimeZone>>,
    #[serde(default, deserialize_with = "nullable")]
    pub calling_code: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GeoIpAs {
    pub asn: String,
    pub name: String,
    pub name_en: Option<String>,
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
    pub domain: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Geolocation {
    pub city: String,
    pub region: String,
    pub district: String,
    pub region_code: String,
    pub adcode: String,
    pub country: String,
    pub country_code: String,
    pub continent: String,
    pub continent_code: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: String,
    pub postal_code: String,
    pub radius: Option<f64>,
    pub area_code: Option<String>,
    pub plus_code: Option<String>,
    pub country_en: Option<String>,
    pub country_zh: Option<String>,
    pub region_en: Option<String>,
    pub region_zh: Option<String>,
    pub city_en: Option<String>,
    pub city_zh: Option<String>,
    pub district_en: Option<String>,
    pub district_zh: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Currency {
    pub name: String,
    pub code: String,
    pub symbol: String,
    pub native: String,
    pub plural: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TimeZone {
    pub name: String,
    pub abbr: String,
    pub offset: String,
    pub is_dst: bool,
    pub current_time: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct IntelligenceData {
    #[serde(rename = "as")]
    pub autonomous_system: IntelligenceAs,
    pub intelligence: LiveIntelligence,
    pub ip: String,
    pub network: Option<NetworkInfo>,
    pub network_class: NetworkClassification,
    pub mobile: Option<MobileCarrier>,
    pub threat: Option<ThreatInfo>,
    pub rpki: Option<RpkiInfo>,
    pub company: Option<Company>,
    pub abuse: Option<AbuseContact>,
    pub dns: Option<DnsRecords>,
    pub is_bogon: bool,
    pub is_anycast: Option<bool>,
    pub is_mobile: Option<bool>,
    pub is_hosting: Option<bool>,
    pub is_satellite: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct IntelligenceAs {
    pub asn: String,
    pub name: String,
    pub name_en: Option<String>,
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
    pub domain: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub class: Option<String>,
    pub tags: Option<Vec<String>>,
    pub connectivity: Option<AsnConnectivity>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AsnConnectivity {
    pub declared_traffic: String,
    pub exchanges_total: Option<u32>,
    pub estimated_capacity: AsnEstimatedCapacity,
    pub exchanges: Vec<AsnExchange>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AsnEstimatedCapacity {
    pub lower_gbps: f64,
    pub upper_gbps: Option<f64>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AsnExchange {
    pub name: String,
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
    #[serde(rename = "type")]
    pub kind: AsnExchangeType,
    pub ports: Vec<AsnExchangePort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsnExchangeType {
    Physical,
    Virtual,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AsnExchangePort {
    pub capacity_gbps: f64,
    pub operational: Option<bool>,
    pub rs_peer: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NetworkInfo {
    pub connection_type: String,
    pub usage_type: String,
    pub address_type: Option<String>,
    pub net_speed: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NetworkClassification {
    pub primary: NetworkClass,
    pub confidence: NetworkClassConfidence,
    pub source: NetworkClassSource,
    pub conflicts: Vec<NetworkClass>,
    pub evidence: Vec<NetworkClassEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NetworkClassEvidence {
    pub class: NetworkClass,
    pub source: NetworkClassSource,
    pub scope: NetworkClassScope,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkClass {
    Mobile,
    Hosting,
    Residential,
    Business,
    Organization,
    Government,
    Military,
    Education,
    Library,
    Dedicated,
    Satellite,
    Ixp,
    SearchEngine,
    AiCrawler,
    Unknown,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkClassConfidence {
    High,
    Medium,
    Low,
    Unknown,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkClassSource {
    DomesticIsp,
    UsageType,
    MobileCarrier,
    ConnectionType,
    NetworkTag,
    ThreatIntelligence,
    AsnTag,
    Ixp,
    None,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkClassScope {
    Ip,
    Prefix,
    Asn,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MobileCarrier {
    pub mcc: String,
    pub mnc: String,
    pub brand: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ThreatInfo {
    pub is_tor: Option<bool>,
    pub is_vpn: Option<bool>,
    pub is_icloud_relay: Option<bool>,
    pub is_cloudflare_warp: Option<bool>,
    pub is_proxy: Option<bool>,
    pub is_datacenter: Option<bool>,
    pub is_anonymous: Option<bool>,
    pub is_known_attacker: Option<bool>,
    pub is_known_abuser: Option<bool>,
    pub is_threat: Option<bool>,
    pub is_bogon: bool,
    pub blocklists: Vec<Blocklist>,
    pub blocklist_count: u32,
    pub scores: ThreatScores,
    pub anonymizer_exemptions: Option<Vec<AnonymizerExemptionReason>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Blocklist {
    pub name: String,
    pub site: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ThreatScores {
    pub vpn_score: Option<f64>,
    pub proxy_score: Option<f64>,
    pub threat_score: Option<f64>,
    pub trust_score: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnonymizerExemptionReason {
    CloudflareCdnOrigin,
    SearchEngineCrawler,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RpkiInfo {
    pub status: RpkiStatus,
    pub source: Option<RpkiSource>,
    pub route_source: Option<String>,
    pub origin_asn: Option<String>,
    pub route_prefix: Option<String>,
    pub roa_prefix: Option<String>,
    pub roa_asn: Option<String>,
    pub max_length: Option<u8>,
    pub ta: Option<String>,
    pub rtr: Option<RpkiRtr>,
    pub validator: Option<RpkiValidator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpkiStatus {
    Valid,
    InvalidAsn,
    InvalidLength,
    Unknown,
    NotRouted,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum RpkiSource {
    #[serde(rename = "RTR Live")]
    RtrLive,
    #[serde(rename = "Local Cache")]
    LocalCache,
    #[serde(rename = "Local DB")]
    LocalDb,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpkiRtr {
    pub source: RpkiRtrSource,
    pub server: Option<String>,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum RpkiRtrSource {
    Cloudflare,
    #[serde(rename = "MFEED")]
    Mfeed,
    #[serde(rename = "RTR")]
    Rtr,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpkiValidator {
    pub source: String,
    pub engine: String,
    pub last_updated: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Company {
    pub name: Option<String>,
    pub address: Option<String>,
    pub domain: Option<String>,
    pub phone: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub network: Option<String>,
    pub country: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AbuseContact {
    pub name: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub email: Option<String>,
    pub network: Option<String>,
    pub phone: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DnsRecords {
    pub ptr: Vec<String>,
    pub ptr_evidence: Vec<DnsPtrEvidence>,
    pub hostnames: Vec<DnsHostnameEvidence>,
    pub sources: Vec<String>,
    pub snapshot_id: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DnsPtrEvidence {
    pub hostname: String,
    pub sources: Vec<String>,
    pub forward_confirmed: bool,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DnsHostnameEvidence {
    pub hostname: String,
    pub kinds: Vec<DnsHostnameKind>,
    pub sources: Vec<String>,
    pub forward_confirmed: bool,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsHostnameKind {
    Fdns,
    Ct,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LiveIntelligence {
    pub ptr: Vec<PtrIntelligence>,
    pub nameservers: Vec<String>,
    pub cpes: Vec<CpeObservation>,
    pub tags: Vec<LiveIntelligenceTag>,
    pub revoked_tags: Vec<RevokedNetworkTag>,
    pub usage_type: Option<String>,
    pub rpki: Option<RpkiInfo>,
    #[serde(default, deserialize_with = "nullable")]
    pub abuse: Option<Option<AbuseReports>>,
    pub transfers: Option<Vec<ResourceTransfer>>,
    #[serde(default, deserialize_with = "nullable")]
    pub segment_probe: Option<Option<SegmentProbe>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveIntelligenceTag {
    Hosting,
    PublicDnsResolver,
    WebServer,
    Vpn,
    ResidentialProxy,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevokedNetworkTag {
    BusinessBroadband,
    HomeIsp,
    MobileIsp,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CpeObservation {
    pub cpe: String,
    pub part: CpePart,
    pub vendor: String,
    pub product: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpePart {
    Application,
    OperatingSystem,
    Hardware,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AbuseReports {
    pub confidence_score: u32,
    pub total_reports: u32,
    pub distinct_reporters: u32,
    pub last_reported_at: Option<String>,
    pub is_whitelisted: Option<bool>,
    pub categories: Vec<AbuseCategoryCount>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct AbuseCategoryCount {
    pub id: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ResourceTransfer {
    pub transfer_time: String,
    pub transfer_type: String,
    pub source_resource: String,
    pub recipient_resource: String,
    pub source_registry: String,
    pub recipient_registry: String,
    pub source_holder: String,
    pub recipient_holder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SegmentProbe {
    pub ip: String,
    pub ports: Vec<u16>,
    pub tags: Vec<LiveIntelligenceTag>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PtrIntelligence {
    Located(PtrLocatedIntelligence),
    AsnMismatch(PtrAsnMismatchIntelligence),
    Unmatched(PtrUnmatchedIntelligence),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PtrLocatedIntelligence {
    pub hostname: String,
    pub operator: PtrOperator,
    pub hint: PtrHint,
    pub location: PtrMetroLocation,
    pub confidence: PtrConfidence,
    pub evidence: PtrEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtrAsnMismatchIntelligence {
    pub hostname: String,
    pub operator: PtrOperator,
    pub observed_asn: String,
    pub evidence: PtrEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtrUnmatchedIntelligence {
    pub hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtrOperator {
    pub name: String,
    pub asn: String,
    pub suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtrHint {
    pub value: String,
    pub kind: PtrHintKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtrHintKind {
    Iata,
    Unlocode,
    Clli,
    CityName,
    OperatorCode,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PtrMetroLocation {
    pub city: String,
    pub region: Option<String>,
    pub region_code: Option<String>,
    pub country: String,
    pub country_code: String,
    pub latitude: f64,
    pub longitude: f64,
    pub precision: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PtrConfidence {
    pub level: PtrConfidenceLevel,
    pub score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtrConfidenceLevel {
    High,
    Medium,
    Low,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtrEvidence {
    pub rule_id: String,
    pub ruleset_version: String,
    pub sources: Vec<PtrEvidenceSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtrEvidenceSource {
    pub dataset: PtrDatasetName,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PtrDatasetName {
    Geonames,
    Ourairports,
    OperatorOverrides,
    Unlocode,
    #[serde(other)]
    Other,
}
