use reqwest::Response;
use std::future::Future;
use std::time::Duration;
use tracing::warn;

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Execute an HTTP request with exponential backoff retry.
/// Retries on 429 (rate limited) and 5xx (server error) responses.
pub async fn with_retry<F, Fut>(make_request: F) -> Result<Response, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<Response, reqwest::Error>>,
{
    let mut attempt = 0;
    loop {
        let response = make_request().await?;
        let status = response.status();

        if status.is_success()
            || status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            return Ok(response);
        }

        attempt += 1;
        if attempt >= MAX_RETRIES {
            return Ok(response);
        }

        let backoff = Duration::from_millis(INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1));
        warn!(
            status = status.as_u16(),
            attempt,
            backoff_ms = backoff.as_millis() as u64,
            "Retrying request"
        );
        tokio::time::sleep(backoff).await;
    }
}
