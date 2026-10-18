use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::{BucketStore, LimitsConfig, TokenBucket};

pub struct RateLimiter {
    pub store: Arc<dyn BucketStore>,
    pub config: LimitsConfig,
}

/// Spends one token per request from the bucket of the caller's API key
/// and answers 429 once the bucket is empty.
pub async fn rate_limit(State(limiter): State<Arc<RateLimiter>>, request: Request, next: Next) -> Response {
    let Some(api_key) = request.headers().get("x-api-key").and_then(|v| v.to_str().ok()) else {
        return (StatusCode::UNAUTHORIZED, "missing API key").into_response();
    };
    let api_key = api_key.to_owned();

    let limits = limiter.config.for_key(&api_key);
    let now = unix_now();
    let mut bucket = match limiter.store.get(&api_key).await {
        Ok(Some(bucket)) => bucket,
        Ok(None) => TokenBucket::for_plan(&limits, now),
        Err(err) => {
            tracing::error!(%err, "load bucket");
            return (StatusCode::SERVICE_UNAVAILABLE, "rate limiter unavailable").into_response();
        }
    };
    let allowed = bucket.take(now);
    if let Err(err) = limiter.store.set(&api_key, &bucket, limits.idle_ttl()).await {
        tracing::error!(%err, "save bucket");
    }

    if !allowed {
        let wait = bucket.retry_after(now);
        let headers = [(header::RETRY_AFTER, wait.ceil().to_string())];
        return (StatusCode::TOO_MANY_REQUESTS, headers, "rate limit exceeded").into_response();
    }
    next.run(request).await
}

fn unix_now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}
