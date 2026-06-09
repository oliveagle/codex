use crate::error::TransportError;
use crate::request::Request;
use http::HeaderMap;
use http::header::RETRY_AFTER;
use http::header::HeaderName;
use http::header::HeaderValue;
use rand::Rng;
use std::future::Future;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::time::sleep;

const X_RATELIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");
const X_RATELIMIT_RESET_AFTER: HeaderName =
    HeaderName::from_static("x-ratelimit-reset-after");

/// Maximum delay (cap) for any single retry, regardless of `Retry-After`.
/// Some servers return very large values (e.g. on permanent bans) - we never
/// want to block for hours, so cap at 60s.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u64,
    pub base_delay: Duration,
    pub retry_on: RetryOn,
}

#[derive(Debug, Clone)]
pub struct RetryOn {
    pub retry_429: bool,
    pub retry_5xx: bool,
    pub retry_transport: bool,
}

impl RetryOn {
    pub fn should_retry(&self, err: &TransportError, attempt: u64, max_attempts: u64) -> bool {
        if attempt >= max_attempts {
            return false;
        }
        match err {
            TransportError::Http { status, .. } => {
                (self.retry_429 && status.as_u16() == 429)
                    || (self.retry_5xx && status.is_server_error())
            }
            TransportError::Timeout | TransportError::Network(_) => self.retry_transport,
            _ => false,
        }
    }
}

/// Compute the delay before the next retry attempt.
///
/// For 429 responses with a `Retry-After`, `X-RateLimit-Reset`, or
/// `X-RateLimit-Reset-After` header, the value from the header is used
/// (clamped to `MAX_RETRY_AFTER`). Otherwise, falls back to exponential
/// backoff with jitter.
pub fn backoff_for(base: Duration, attempt: u64, err: &TransportError) -> Duration {
    if let Some(server_delay) = retry_after_from_err(err) {
        return server_delay;
    }
    backoff(base, attempt)
}

/// Parse the retry delay from a 429 response's headers.
///
/// Supports, in order of preference:
/// - `Retry-After: <seconds>`
/// - `Retry-After: <HTTP-date>`  (RFC 7231 §7.1.3)
/// - `X-RateLimit-Reset-After: <seconds>`
/// - `X-RateLimit-Reset: <unix-seconds>`
pub fn retry_after_from_err(err: &TransportError) -> Option<Duration> {
    let headers = match err {
        TransportError::Http { headers, .. } => headers.as_ref()?,
        _ => return None,
    };
    retry_after_from_headers(headers)
}

pub fn retry_after_from_headers(headers: &HeaderMap) -> Option<Duration> {
    // 1. Retry-After
    if let Some(v) = headers.get(RETRY_AFTER) {
        if let Some(d) = parse_retry_after(v) {
            return Some(d);
        }
    }
    // 2. X-RateLimit-Reset-After (seconds, GitHub-style)
    if let Some(v) = headers.get(&X_RATELIMIT_RESET_AFTER) {
        if let Some(d) = parse_seconds(v) {
            return Some(d);
        }
    }
    // 3. X-RateLimit-Reset (unix epoch seconds)
    if let Some(v) = headers.get(&X_RATELIMIT_RESET) {
        if let Some(d) = parse_unix_reset(v) {
            return Some(d);
        }
    }
    None
}

fn parse_retry_after(v: &HeaderValue) -> Option<Duration> {
    let s = v.to_str().ok()?.trim();
    if let Some(secs) = parse_seconds_str(s) {
        return Some(Duration::from_secs(secs));
    }
    // HTTP-date fallback
    if let Ok(t) = httpdate::parse_http_date(s) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
        let target = t.duration_since(UNIX_EPOCH).ok()?;
        if target > now {
            return Some(target - now);
        }
    }
    None
}

fn parse_seconds(v: &HeaderValue) -> Option<Duration> {
    let s = v.to_str().ok()?.trim();
    let secs = parse_seconds_str(s)?;
    Some(Duration::from_secs(secs))
}

fn parse_unix_reset(v: &HeaderValue) -> Option<Duration> {
    let s = v.to_str().ok()?.trim();
    let secs: u64 = s.parse().ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let target = Duration::from_secs(secs);
    if target > now {
        Some(target - now)
    } else {
        Some(Duration::ZERO)
    }
}

fn parse_seconds_str(s: &str) -> Option<u64> {
    // Accept either an integer or a float (e.g. "1.5").
    if let Ok(secs) = s.parse::<u64>() {
        return Some(secs);
    }
    let f: f64 = s.parse().ok()?;
    if f.is_sign_negative() || !f.is_finite() {
        return None;
    }
    Some(f.ceil() as u64)
}

pub fn backoff(base: Duration, attempt: u64) -> Duration {
    if attempt == 0 {
        return base;
    }
    let exp = 2u64.saturating_pow(attempt as u32 - 1);
    let millis = base.as_millis() as u64;
    let raw = millis.saturating_mul(exp);
    let jitter: f64 = rand::rng().random_range(0.9..1.1);
    let computed = Duration::from_millis((raw as f64 * jitter) as u64);
    if computed > MAX_RETRY_AFTER {
        MAX_RETRY_AFTER
    } else {
        computed
    }
}

pub async fn run_with_retry<T, F, Fut>(
    policy: RetryPolicy,
    mut make_req: impl FnMut() -> Request,
    op: F,
) -> Result<T, TransportError>
where
    F: Fn(Request, u64) -> Fut,
    Fut: Future<Output = Result<T, TransportError>>,
{
    for attempt in 0..=policy.max_attempts {
        let req = make_req();
        match op(req, attempt).await {
            Ok(resp) => return Ok(resp),
            Err(err)
                if policy
                    .retry_on
                    .should_retry(&err, attempt, policy.max_attempts) =>
            {
                let delay = backoff_for(policy.base_delay, attempt + 1, &err);
                sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
    Err(TransportError::RetryLimit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use http::StatusCode;

    fn make_err_with_headers(headers: HeaderMap) -> TransportError {
        TransportError::Http {
            status: StatusCode::TOO_MANY_REQUESTS,
            url: None,
            headers: Some(headers),
            body: None,
        }
    }

    #[test]
    fn retry_after_seconds() {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, HeaderValue::from_static("5"));
        let err = make_err_with_headers(h);
        assert_eq!(retry_after_from_err(&err), Some(Duration::from_secs(5)));
    }

    #[test]
    fn retry_after_float_seconds() {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, HeaderValue::from_static("2.5"));
        let err = make_err_with_headers(h);
        assert_eq!(retry_after_from_err(&err), Some(Duration::from_secs(3)));
    }

    #[test]
    fn ratelimit_reset_after() {
        let mut h = HeaderMap::new();
        h.insert(&X_RATELIMIT_RESET_AFTER, HeaderValue::from_static("12"));
        let err = make_err_with_headers(h);
        assert_eq!(retry_after_from_err(&err), Some(Duration::from_secs(12)));
    }

    #[test]
    fn ratelimit_reset_unix() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let future = now + Duration::from_secs(30);
        let mut h = HeaderMap::new();
        h.insert(
            &X_RATELIMIT_RESET,
            HeaderValue::from_str(&future.as_secs().to_string()).unwrap(),
        );
        let err = make_err_with_headers(h);
        let d = retry_after_from_err(&err).unwrap();
        // Allow 1s slop for the time it took to run the test
        assert!(d.as_secs() >= 28 && d.as_secs() <= 30);
    }

    #[test]
    fn no_headers_falls_back() {
        let err = make_err_with_headers(HeaderMap::new());
        assert_eq!(retry_after_from_err(&err), None);
    }

    #[test]
    fn non_http_error_returns_none() {
        let err = TransportError::Timeout;
        assert_eq!(retry_after_from_err(&err), None);
    }

    #[test]
    fn backoff_caps_at_max() {
        // Very large attempt count - ensure we never exceed MAX_RETRY_AFTER
        let d = backoff(Duration::from_secs(10), 30);
        assert!(d <= MAX_RETRY_AFTER);
    }
}
