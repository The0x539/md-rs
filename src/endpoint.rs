use bytes::Bytes;
use once_cell::sync::Lazy;
use ratelimit::RateLimiter;
use reqwest::{Client, IntoUrl};
use serde::de::DeserializeOwned;
use snafu::ResultExt;
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

use crate::error::{HttpErr, JsonErr, Result};
use crate::schema;

static RATE_LIMIT: Lazy<RateLimiter> = Lazy::new(|| RateLimiter::new(5, Duration::from_secs(1)));
static CLIENT: Lazy<Client> = Lazy::new(Client::new);

pub async fn get_json<U: IntoUrl, T: DeserializeOwned>(url: U) -> Result<T> {
    let permit = RATE_LIMIT.request().await;
    let resp = CLIENT
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .context(HttpErr)?;
    let text = resp.text().await.context(HttpErr)?;
    drop(permit);
    let val = serde_json::from_str(&text).context(JsonErr {
        type_name: pretty_type_name::pretty_type_name::<T>(),
    })?;
    Ok(val)
}

pub async fn report(url: &str, success: bool, cached: bool, bytes: usize, duration: u128) {
    let report = schema::HealthReport {
        url,
        success,
        cached,
        bytes,
        duration,
    };
    let _ = CLIENT
        .post("https://api.mangadex.network/report")
        .json(&report)
        .send()
        .await;
}

pub async fn get_image(
    base_url: &Url,
    quality_mode: &str,
    hash: &schema::ChapterHash,
    filename: &schema::Filename,
) -> Result<Bytes> {
    let url = format!("{}/{}/{}/{}", base_url, quality_mode, hash, filename);

    let permit = RATE_LIMIT.request().await;
    let before = Instant::now();

    async fn err(e: reqwest::Error, before: Instant, url: &str) -> Result<Bytes> {
        if !e.is_builder() {
            let duration = (Instant::now() - before).as_millis();
            report(&url, false, false, 0, duration).await;
        }
        Err(e).context(HttpErr)
    }

    let resp = match CLIENT
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => r,
        Err(e) => return err(e, before, &url).await,
    };

    let cached = resp.headers().get("X-Cache").map(|v| v.as_ref()) == Some(b"HIT");

    let bytes = match resp.bytes().await {
        Ok(r) => r,
        Err(e) => return err(e, before, &url).await,
    };

    drop(permit);

    let duration = (Instant::now() - before).as_millis();
    report(&url, true, cached, bytes.len(), duration).await;

    Ok(bytes)
}
