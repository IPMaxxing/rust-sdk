# ipmax

Async Rust client for the [IP-Max](https://ipm.ax) GeoIP and IP intelligence API.

## Install

```sh
cargo add ipmax
```

The client runs on Tokio and uses `reqwest` with rustls. Rust 1.91 or newer.

## Quickstart

```rust
#[tokio::main]
async fn main() -> Result<(), ipmax::Error> {
    let client = ipmax::Client::new("sg_live_...")?;

    let geo = client.geoip("8.8.8.8").await?;
    println!("{} {}", geo.geo.country_code, geo.geo.city);

    let intel = client.intelligence("8.8.8.8").await?;
    println!("{:?}", intel.network_class.primary);

    let catalog = client.catalog().await?;
    let account = client.account().await?;
    println!("{} prices, {} wallets", catalog.prices.len(), account.wallets.len());
    Ok(())
}
```

Lookups send a fresh UUID v4 `Idempotency-Key` and reuse it on every retry, so a retried request is never billed twice. To choose the key yourself, call `geoip_with_key(ip, key)` or `intelligence_with_key(ip, key)`. These skip the cache read but still store the result.

## Configuration

```rust
use std::time::Duration;

let client = ipmax::Client::builder()
    .api_key("sg_live_...")
    .timeout(Duration::from_secs(5))
    .max_retries(3)
    .cache_capacity(4096)
    .build()?;
```

| Option | Default | Meaning |
| --- | --- | --- |
| `api_key` | `IPMAX_API_KEY` env var | Bearer key. `catalog()` works without one. |
| `base_url` | `https://api.ipm.ax` | API origin. |
| `timeout` | 10 s | Per attempt. |
| `max_retries` | 2 | Retries network errors, 408, 429 and 5xx with exponential backoff and full jitter (0.5 s base, 8 s cap). Honours `Retry-After`. |
| `cache_capacity` | 1024 | Maximum cached lookups. `0` disables the cache. |
| `cache_ttl` | 5 min | Lifetime of a cached lookup. |
| `http_client` | `reqwest::Client` with defaults | Bring your own client for proxies, custom TLS or tests. |

Only successful `geoip` and `intelligence` results are cached, keyed by product and trimmed IP. A cache hit makes no request and costs nothing. Clones of a `Client` share the cache.

## Error handling

Every call returns `Result<T, ipmax::Error>`:

- `Error::Api(ApiError)`: the API answered with a non-2xx status. `ApiError` carries `status`, `code`, `message`, `request_id`, `retryable` and `retry_after`.
- `Error::Http(reqwest::Error)`: connection failure, timeout or other transport problem.
- `Error::Decode(serde_json::Error)`: a successful response that did not match the expected shape.

```rust
use ipmax::{Error, ErrorCode};

match client.geoip("203.0.113.9").await {
    Ok(data) => println!("{}", data.geo.country),
    Err(Error::Api(error)) if error.code == Some(ErrorCode::IP_NOT_FOUND) => {
        println!("not in the database, nothing was charged");
    }
    Err(Error::Api(error)) if error.code == Some(ErrorCode::INSUFFICIENT_BALANCE) => {
        println!("top up: {}", error.message);
    }
    Err(error) => eprintln!("lookup failed: {error}"),
}
```

`code` is `None` when the error body is not the API's JSON envelope, for example a proxy's HTML page. Unknown error codes and unknown enum values (decoded as `Other`) never fail decoding.

## License

[Apache-2.0](LICENSE)
