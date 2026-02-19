use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
use crate::config::Env;

pub async fn fetch_data(url: &str, env: &Env) -> Result<serde_json::Value> {
    let retries = env.network_retry_limit;
    let timeout = Duration::from_millis(env.request_timeout_ms);
    let retry_delay = Duration::from_secs(1);

    let client = Client::builder()
        .timeout(timeout)
        .build()?;

    for attempt in 1..=retries {
        match client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(response.json().await?);
                } else if attempt < retries {
                    let delay = retry_delay * (1 << (attempt - 1)); // Exponential backoff
                    eprintln!(
                        "⚠️  HTTP error {} (attempt {}/{}), retrying in {:?}...",
                        response.status(),
                        attempt,
                        retries,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    anyhow::bail!("HTTP error: {}", response.status());
                }
            }
            Err(e) => {
                let is_network_error = e.is_timeout()
                    || e.is_connect()
                    || e.is_request()
                    || e.to_string().contains("network");

                if is_network_error && attempt < retries {
                    let delay = retry_delay * (1 << (attempt - 1)); // Exponential backoff
                    eprintln!(
                        "⚠️  Network error (attempt {}/{}), retrying in {:?}...",
                        attempt,
                        retries,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                } else if attempt == retries && is_network_error {
                    eprintln!("❌ Network timeout after {} attempts - {}", retries, e);
                }
                return Err(e.into());
            }
        }
    }

    unreachable!()
}

