use std::{fmt, sync::Arc, time::Duration};

use reqwest::{
    Method, RequestBuilder,
    header::{ACCEPT, USER_AGENT},
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
    Account, ApiError, Catalog, Envelope, Error, GeoIpData, IntelligenceData, LookupRequest,
    cache::Cache,
};

const DEFAULT_BASE_URL: &str = "https://api.ipm.ax";
const API_KEY_VAR: &str = "IPMAX_API_KEY";
const AGENT: &str = concat!("ipmax-rust/", env!("CARGO_PKG_VERSION"));
const BACKOFF_BASE: Duration = Duration::from_millis(500);
const BACKOFF_CAP: Duration = Duration::from_secs(8);

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    timeout: Duration,
    max_retries: u32,
    cache: Option<Arc<Cache>>,
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Result<Self, Error> {
        Self::builder().api_key(api_key).build()
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub async fn catalog(&self) -> Result<Catalog, Error> {
        self.execute(|| self.request(Method::GET, "catalog")).await
    }

    pub async fn account(&self) -> Result<Account, Error> {
        self.execute(|| self.request(Method::GET, "account")).await
    }

    pub async fn geoip(&self, ip: &str) -> Result<GeoIpData, Error> {
        self.lookup("geoip", ip, None).await
    }

    pub async fn geoip_with_key(
        &self,
        ip: &str,
        idempotency_key: &str,
    ) -> Result<GeoIpData, Error> {
        self.lookup("geoip", ip, Some(idempotency_key)).await
    }

    pub async fn intelligence(&self, ip: &str) -> Result<IntelligenceData, Error> {
        self.lookup("intelligence", ip, None).await
    }

    pub async fn intelligence_with_key(
        &self,
        ip: &str,
        idempotency_key: &str,
    ) -> Result<IntelligenceData, Error> {
        self.lookup("intelligence", ip, Some(idempotency_key)).await
    }

    async fn lookup<T>(
        &self,
        product: &'static str,
        ip: &str,
        idempotency_key: Option<&str>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned + Clone + Send + 'static,
    {
        let ip = ip.trim();
        if idempotency_key.is_none()
            && let Some(hit) = self.cache.as_ref().and_then(|cache| cache.get(product, ip))
        {
            return Ok(hit);
        }
        let idempotency_key =
            idempotency_key.map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
        let body = LookupRequest { ip: ip.to_owned() };
        let data: T = self
            .execute(|| {
                self.request(Method::POST, product)
                    .header("Idempotency-Key", &idempotency_key)
                    .json(&body)
            })
            .await?;
        if let Some(cache) = &self.cache {
            cache.put(product, ip, data.clone());
        }
        Ok(data)
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let request = self
            .http
            .request(method, format!("{}/api/v1/{path}", self.base_url))
            .timeout(self.timeout)
            .header(USER_AGENT, AGENT)
            .header(ACCEPT, "application/json");
        match &self.api_key {
            Some(api_key) => request.bearer_auth(api_key),
            None => request,
        }
    }

    async fn execute<T: DeserializeOwned>(
        &self,
        request: impl Fn() -> RequestBuilder,
    ) -> Result<T, Error> {
        let mut attempt = 0;
        loop {
            let error = match send(request()).await {
                Ok(data) => return Ok(data),
                Err(error) => error,
            };
            if attempt >= self.max_retries || !error.is_retryable() {
                return Err(error);
            }
            tokio::time::sleep(error.retry_after().unwrap_or_else(|| backoff(attempt))).await;
            attempt += 1;
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
    cache_capacity: usize,
    cache_ttl: Duration,
    http: Option<reqwest::Client>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_owned(),
            timeout: Duration::from_secs(10),
            max_retries: 2,
            cache_capacity: 1024,
            cache_ttl: Duration::from_secs(300),
            http: None,
        }
    }
}

impl ClientBuilder {
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity;
        self
    }

    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        let http = match self.http {
            Some(http) => http,
            None => reqwest::Client::builder().build()?,
        };
        Ok(Client {
            http,
            base_url: self.base_url.trim_end_matches('/').to_owned(),
            api_key: self
                .api_key
                .or_else(|| std::env::var(API_KEY_VAR).ok())
                .filter(|api_key| !api_key.is_empty()),
            timeout: self.timeout,
            max_retries: self.max_retries,
            cache: Cache::new(self.cache_capacity, self.cache_ttl).map(Arc::new),
        })
    }
}

async fn send<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, Error> {
    let response = request.send().await?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await?;
    if status.is_success() {
        Ok(serde_json::from_slice::<Envelope<T>>(&body)?.data)
    } else {
        Err(ApiError::from_response(status, &headers, &body).into())
    }
}

fn backoff(attempt: u32) -> Duration {
    BACKOFF_BASE
        .saturating_mul(2u32.saturating_pow(attempt))
        .min(BACKOFF_CAP)
        .mul_f64(rand::random())
}
