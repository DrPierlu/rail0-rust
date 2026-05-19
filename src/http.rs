use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{ApiErrorBody, Rail0Error};

/// One log record emitted per request attempt.
#[derive(Debug)]
pub struct LogEntry {
    /// HTTP method (GET, POST, …).
    pub method: String,
    /// Full URL including query string.
    pub url: String,
    /// Wall-clock time from sending to receiving, in milliseconds.
    pub duration_ms: u64,
    /// Serialised request body, if any (POST requests).
    pub request_body: Option<serde_json::Value>,
    /// HTTP status code. `None` on network-level errors.
    pub status: Option<u16>,
    /// Parsed JSON response body, if any.
    pub response_body: Option<serde_json::Value>,
    /// Network error or API error for non-2xx responses.
    pub error: Option<String>,
    /// 1-based attempt number. `None` when `max_retries` is 0.
    pub attempt: Option<u32>,
    /// `true` when a retry is scheduled after this failed attempt.
    pub will_retry: bool,
}

/// Pluggable logging callback. Receives one [`LogEntry`] per request attempt.
pub type Logger = Arc<dyn Fn(LogEntry) + Send + Sync>;

/// Built-in logger that writes a one-line summary to stderr.
///
/// ```no_run
/// use rail0::{Rail0Client, ClientOptions, debug_logger};
/// let client = Rail0Client::new(ClientOptions {
///     base_url: "https://api.rail0.xyz".into(),
///     logger: Some(debug_logger()),
///     ..Default::default()
/// });
/// ```
pub fn debug_logger() -> Logger {
    Arc::new(|e: LogEntry| {
        if let Some(err) = &e.error {
            let attempt = e.attempt.map(|a| format!(" [attempt {a}]")).unwrap_or_default();
            eprintln!("[rail0] ERROR{attempt} {} {} ! {err}", e.method, e.url);
        } else {
            let status = e.status.map(|s| format!(" {s}")).unwrap_or_default();
            let attempt = e.attempt.map(|a| format!(" [attempt {a}]")).unwrap_or_default();
            eprintln!("[rail0]{attempt} {}{status} {} {}ms", e.method, e.url, e.duration_ms);
        }
    })
}

/// Constructor options for [`Rail0Client`](crate::Rail0Client).
#[derive(Clone)]
pub struct ClientOptions {
    /// Base URL of the RAIL0 API, e.g. `"https://api.rail0.xyz"`. Trailing slash is stripped.
    pub base_url: String,
    /// Extra headers merged into every request. Useful for API keys or correlation IDs.
    pub headers: HashMap<String, String>,
    /// Per-request timeout. Default: 30 s.
    pub timeout: Duration,
    /// Number of extra attempts after the first network failure.
    /// Only network errors are retried — HTTP errors (4xx / 5xx) are not. Default: 0.
    pub max_retries: u32,
    /// Base delay between retries; doubles each attempt (exponential backoff). Default: 200 ms.
    pub retry_delay: Duration,
    /// Optional logging callback. Receives one entry per request attempt.
    pub logger: Option<Logger>,
    /// Custom `reqwest::Client`. When `None`, a default client is constructed from `timeout`.
    /// Inject a client pointed at a mock server URL for testing.
    pub client: Option<reqwest::Client>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            max_retries: 0,
            retry_delay: Duration::from_millis(200),
            logger: None,
            client: None,
        }
    }
}

pub(crate) struct HttpClient {
    base_url: String,
    headers: HashMap<String, String>,
    max_retries: u32,
    retry_delay: Duration,
    logger: Option<Logger>,
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new(opts: ClientOptions) -> Self {
        let base_url = opts.base_url.trim_end_matches('/').to_owned();

        let mut headers = HashMap::from([("content-type".into(), "application/json".into())]);
        headers.extend(opts.headers);

        let client = opts.client.unwrap_or_else(|| {
            reqwest::Client::builder()
                .timeout(opts.timeout)
                .build()
                .expect("reqwest::Client build failed")
        });

        Self {
            base_url,
            headers,
            max_retries: opts.max_retries,
            retry_delay: opts.retry_delay,
            logger: opts.logger,
            client,
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Rail0Error> {
        self.execute(reqwest::Method::GET, path, None::<&()>).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Rail0Error> {
        self.execute(reqwest::Method::POST, path, Some(body)).await
    }

    pub async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Rail0Error> {
        self.execute(reqwest::Method::PUT, path, Some(body)).await
    }

    async fn execute<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, Rail0Error> {
        let url = format!("{}{}", self.base_url, path);
        let max_attempts = self.max_retries + 1;
        let track_attempts = self.max_retries > 0;

        let body_json: Option<serde_json::Value> = match body {
            Some(b) => Some(serde_json::to_value(b)?),
            None => None,
        };

        let mut attempt = 0u32;
        loop {
            attempt += 1;

            if attempt > 1 {
                let exp = attempt.saturating_sub(2);
                let multiplier = 1u32.checked_shl(exp).unwrap_or(u32::MAX);
                let delay = self.retry_delay.saturating_mul(multiplier);
                tokio::time::sleep(delay).await;
            }

            let mut req = self.client.request(method.clone(), &url);
            for (k, v) in &self.headers {
                req = req.header(k, v);
            }
            if let Some(ref json_body) = body_json {
                req = req.json(json_body);
            }

            let start = Instant::now();
            match req.send().await {
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let will_retry = attempt < max_attempts;
                    if let Some(ref logger) = self.logger {
                        logger(LogEntry {
                            method: method.to_string(),
                            url: url.clone(),
                            duration_ms,
                            request_body: body_json.clone(),
                            status: None,
                            response_body: None,
                            error: Some(e.to_string()),
                            attempt: track_attempts.then_some(attempt),
                            will_retry,
                        });
                    }
                    if will_retry {
                        continue;
                    }
                    return Err(Rail0Error::Http(e));
                }
                Ok(resp) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let status = resp.status();

                    if status.is_success() {
                        let resp_body: T = resp.json().await?;
                        if let Some(ref logger) = self.logger {
                            logger(LogEntry {
                                method: method.to_string(),
                                url: url.clone(),
                                duration_ms,
                                request_body: body_json.clone(),
                                status: Some(status.as_u16()),
                                response_body: None, // T may not be Serialize; omit from log
                                error: None,
                                attempt: track_attempts.then_some(attempt),
                                will_retry: false,
                            });
                        }
                        return Ok(resp_body);
                    }

                    // Non-2xx: parse the error body — do not retry HTTP errors.
                    let api_err = match resp.json::<ApiErrorBody>().await {
                        Ok(body) => Rail0Error::Api {
                            status: status.as_u16(),
                            code: body.code,
                            message: body.message,
                        },
                        Err(_) => Rail0Error::Api {
                            status: status.as_u16(),
                            code: "UnknownError".into(),
                            message: format!("HTTP {}", status.as_u16()),
                        },
                    };
                    if let Some(ref logger) = self.logger {
                        logger(LogEntry {
                            method: method.to_string(),
                            url: url.clone(),
                            duration_ms,
                            request_body: body_json.clone(),
                            status: Some(status.as_u16()),
                            response_body: None,
                            error: Some(api_err.to_string()),
                            attempt: track_attempts.then_some(attempt),
                            will_retry: false,
                        });
                    }
                    return Err(api_err);
                }
            }
        }
    }
}
