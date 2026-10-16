//! Core LinkedIn API client and configuration definitions.
//!
//! Provides [`Linkedin`] as the root entry point, [`ClientConfig`] to configure
//! browsers and evasions, and sub-clients in dedicated namespace modules.

use crate::errors::{ClientConfigError, HttpApiError, LinkedInError, UrlError};
use crate::models::identifiers::ProfileId;
use log::debug;
use rand::Rng;
use serde::Serialize;
use std::fmt;
use std::ops::{Bound, RangeBounds, RangeInclusive};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio::time::{Duration, Instant, sleep_until};
use url::Url;
use wreq::Client;

pub mod account;
pub mod connections;
pub mod messaging;
pub mod people;
pub mod posts;
pub mod search;

const DEFAULT_EVASION_RANGE_IN_SECONDS: RangeInclusive<f64> = 1.0..=3.0;

/// Invariant: The time calculation logic (next_allowed_request) assumes
/// that only one task computes and advances the time cursor at a time.
/// Capacity MUST remain 1.
const MAX_CONCURRENT_REQUESTS: usize = 1;

/// Handles request spacing and concurrency to avoid LinkedIn anti-bot detection.
///
/// Guarantees that requests execute with a randomized delay within `delay_range`
/// and that at most one request is in-flight on the network at any time.
pub struct EvasionThrottler {
    /// Ensures that only one HTTP request travels on the wire at a time
    transport_lock: Semaphore,
    /// Shared clock to reserve the next slot (ultra-short critical section)
    timeline_cursor: Mutex<Instant>,
    delay_range: Option<RangeInclusive<f64>>,
}

impl EvasionThrottler {
    /// Creates a new throttler with the optional evasion delay range in seconds.
    pub fn new(delay_range: Option<RangeInclusive<f64>>) -> Self {
        Self {
            transport_lock: Semaphore::new(MAX_CONCURRENT_REQUESTS),
            timeline_cursor: Mutex::new(Instant::now()),
            delay_range,
        }
    }

    /// Waits for the next start slot. Call this only while holding the transport permit.
    ///
    /// The clock is advanced when the request can actually start, not when it joins
    /// the queue. Otherwise a slow response consumes the queued requests' delays.
    pub async fn wait_evasion_delay(&self) -> Result<(), LinkedInError> {
        if let Some(range) = &self.delay_range {
            let scheduled_time = {
                // Safe poisoning handling: retrieve data even in case of poison
                let mut cursor = self
                    .timeline_cursor
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

                let now = Instant::now();
                // If inactive, restart from "now", otherwise continue from cursor
                let base_time = (*cursor).max(now);

                // EvasionThrottler is public, so callers may bypass ClientConfig's
                // setter. Validate before sampling or converting to avoid a panic.
                validate_delay_range(range)?;
                let random_secs = rand::rng().random_range(range.clone());
                let delay =
                    Duration::try_from_secs_f64(random_secs).map_err(|_| invalid_delay(range))?;

                // checked_add also protects platforms whose Instant range is narrower
                // than Duration's. Invalid input must remain a recoverable error.
                *cursor = base_time
                    .checked_add(delay)
                    .ok_or_else(|| invalid_delay(range))?;
                base_time
            }; // Release the clock mutex before sleeping; the transport permit
            // still prevents another queued request from reserving an early slot.

            sleep_until(scheduled_time).await;
        }

        Ok(())
    }

    /// Acquires FIFO mutual exclusion on HTTP transport, before waiting for a start slot.
    pub async fn acquire_transport(&self) -> Result<SemaphorePermit<'_>, LinkedInError> {
        self.transport_lock
            .acquire()
            .await
            .map_err(|_| LinkedInError::Api("Throttler transport semaphore was closed".into()))
    }

    /// Acquires and spaces one request, returning a permit to keep until its body is read.
    pub async fn acquire_request(&self) -> Result<SemaphorePermit<'_>, LinkedInError> {
        // Keeping the permit across the wait couples scheduling to real transport
        // availability. Cancellation drops the permit, allowing the queue to progress.
        let permit = self.acquire_transport().await?;
        self.wait_evasion_delay().await?;
        Ok(permit)
    }
}

fn invalid_delay(range: &RangeInclusive<f64>) -> LinkedInError {
    ClientConfigError::InvalidEvasionDelay {
        min: *range.start(),
        max: *range.end(),
    }
    .into()
}

fn validate_delay_range(range: &RangeInclusive<f64>) -> Result<(), LinkedInError> {
    let (min, max) = (*range.start(), *range.end());
    if !min.is_finite() || !max.is_finite() || min < 0.0 || min > max {
        return Err(invalid_delay(range));
    }
    let duration = Duration::try_from_secs_f64(max).map_err(|_| invalid_delay(range))?;
    if Instant::now().checked_add(duration).is_none() {
        return Err(invalid_delay(range));
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) enum RequestType {
    ApiRequest,
}

/// Origin of authentication cookies for the LinkedIn session.
#[derive(Clone)]
pub enum CookieSource {
    /// Automatically extracts cookies from the local desktop browser.
    Auto,
    /// Manually specifies the `li_at` and `JSESSIONID` cookie strings.
    Manual { li_at: String, jsessionid: String },
    /// Loads cookies from a text file formatted in the Netscape standard format.
    NetscapeFile(PathBuf),
}

impl fmt::Debug for CookieSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CookieSource::Auto => {
                write!(f, "Auto")
            }
            CookieSource::Manual { .. } => f
                .debug_struct("Manual")
                .field("li_at", &"<REDACTED>")
                .field("jsessionid", &"<REDACTED>")
                .finish(),
            CookieSource::NetscapeFile(path) => f.debug_tuple("NetscapeFile").field(path).finish(),
        }
    }
}

/// Supported desktop web browsers for local session cookie extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    /// Google Chrome
    Chrome,
    /// Mozilla Firefox
    Firefox,
    /// Microsoft Edge
    Edge,
    /// Apple Safari (macOS only)
    Safari,
    /// Brave Browser
    Brave,
    /// Chromium
    Chromium,
}

impl std::fmt::Display for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Browser::Chrome => "chrome",
            Browser::Firefox => "firefox",
            Browser::Edge => "edge",
            Browser::Safari => "safari",
            Browser::Brave => "brave",
            Browser::Chromium => "chromium",
        };
        write!(f, "{}", s)
    }
}

/// Configuration options for initializing a [`Linkedin`] client instance.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Target browser for automatic cookie extraction.
    pub browser: Browser,
    /// Origin of authentication cookies.
    pub cookie_source: CookieSource,
    /// Optional randomized delay range (in seconds) between outgoing HTTP requests.
    pub evasion_delay: Option<RangeInclusive<f64>>,
}

impl ClientConfig {
    /// Creates a new configuration with default settings for the specified browser.
    ///
    /// Defaults to [`CookieSource::Auto`] and a randomized evasion delay between 1.0 and 3.0 seconds.
    pub fn new(browser: Browser) -> Self {
        Self {
            browser,
            cookie_source: CookieSource::Auto,
            evasion_delay: Some(DEFAULT_EVASION_RANGE_IN_SECONDS),
        }
    }

    /// Configures a randomized delay range in seconds applied before each outgoing request.
    ///
    /// Bounds must be finite, non-negative, and satisfy `start <= end`.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError::Config`] if bounds are negative, non-finite, or reversed.
    pub fn with_evasion_delay_in_seconds<T>(
        mut self,
        delay: impl RangeBounds<T>,
    ) -> Result<Self, LinkedInError>
    where
        T: Into<f64> + Copy,
    {
        let start = match delay.start_bound() {
            Bound::Included(&value) | Bound::Excluded(&value) => value.into(),
            Bound::Unbounded => 0.0,
        };

        let end = match delay.end_bound() {
            Bound::Included(&value) | Bound::Excluded(&value) => value.into(),
            Bound::Unbounded => 60.0,
        };

        if !start.is_finite() || !end.is_finite() {
            return Err(LinkedInError::Config(
                ClientConfigError::InvalidEvasionDelay {
                    min: start,
                    max: end,
                },
            ));
        }

        if start < 0.0 || end < 0.0 {
            return Err(LinkedInError::Config(
                ClientConfigError::InvalidEvasionDelay {
                    min: start,
                    max: end,
                },
            ));
        }

        if start > end {
            return Err(LinkedInError::Config(
                ClientConfigError::InvalidEvasionDelay {
                    min: start,
                    max: end,
                },
            ));
        }

        // A finite f64 is not necessarily representable as a Duration or Instant.
        validate_delay_range(&(start..=end))?;
        self.evasion_delay = Some(start..=end);

        Ok(self)
    }

    /// Disables the evasion delay between requests.
    pub fn without_evasion_delay(mut self) -> Self {
        self.evasion_delay = None;
        self
    }

    /// Manually provides the `li_at` and `JSESSIONID` cookies instead of extracting from a browser.
    pub fn with_manual_cookies(
        mut self,
        li_at: impl Into<String>,
        jsessionid: impl Into<String>,
    ) -> Self {
        self.cookie_source = CookieSource::Manual {
            li_at: li_at.into(),
            jsessionid: jsessionid.into(),
        };
        self
    }

    /// Configures loading cookies from an exported text file in Netscape standard format.
    pub fn with_cookies_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.cookie_source = CookieSource::NetscapeFile(path.into());
        self
    }
}

/// Primary client for interacting with the LinkedIn Voyager API.
///
/// Provides a modular namespace architecture with sub-clients for
/// account details, connections, messaging, people, posts, and search.
pub struct Linkedin {
    pub(crate) client: Client,
    pub(crate) config: ClientConfig,
    // Tokio's OnceCell is used for async instead of standard OnceLock
    pub(crate) profile_urn: tokio::sync::OnceCell<ProfileId>,
    pub(crate) throttler: EvasionThrottler,
}

impl Linkedin {
    pub(crate) const MAX_REPEATED_REQUESTS: usize = 200;
    pub(crate) const LINKEDIN_API_URL: &str = "https://www.linkedin.com/voyager/api";

    /// Creates a new LinkedIn client using default configuration for the specified browser.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if cookie extraction or HTTP client initialization fails.
    pub fn new(browser: Browser) -> Result<Self, LinkedInError> {
        Self::with_config(ClientConfig::new(browser))
    }

    /// Creates a new LinkedIn client with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if cookie extraction or HTTP client initialization fails.
    pub fn with_config(config: ClientConfig) -> Result<Self, LinkedInError> {
        // Public configuration fields can be assigned directly. Validate again at
        // this boundary, before loading cookies or building any network resources.
        if let Some(range) = &config.evasion_delay {
            validate_delay_range(range)?;
        }
        Ok(Self {
            client: crate::client::new_client(&config)?,
            throttler: EvasionThrottler::new(config.evasion_delay.clone()),
            config,
            profile_urn: tokio::sync::OnceCell::new(),
        })
    }

    /// Returns a reference to the client configuration.
    #[inline]
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Retrieves the authenticated user's profile URN, fetching it lazily from the API if not already cached.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the profile request fails or cannot be parsed.
    pub async fn get_or_fetch_urn(&self) -> Result<&ProfileId, LinkedInError> {
        self.profile_urn
            .get_or_try_init(|| async {
                debug!("Your own URN is not present. Fetching it.");
                let profile = self.account().get_my_profile().await?;
                Ok(profile.profile_urn)
            })
            .await
    }

    /// Returns a sub-client for account-related operations (profile info, view stats).
    pub fn account(&self) -> account::AccountClient<'_> {
        account::AccountClient(self)
    }

    /// Returns a sub-client for connection operations (invitations, requests, removals).
    pub fn connections(&self) -> connections::ConnectionsClient<'_> {
        connections::ConnectionsClient(self)
    }

    /// Returns a sub-client for messaging operations (conversations, direct & group messages).
    pub fn messaging(&self) -> messaging::MessagingClient<'_> {
        messaging::MessagingClient(self)
    }

    /// Returns a sub-client for people operations (public profiles, network info, follow).
    pub fn people(&self) -> people::PeopleClient<'_> {
        people::PeopleClient(self)
    }

    /// Returns a sub-client for post operations (publishing, reactions).
    pub fn posts(&self) -> posts::PostsClient<'_> {
        posts::PostsClient(self)
    }

    /// Returns a sub-client for search operations (people search by criteria or URL).
    pub fn search(&self) -> search::SearchClient<'_> {
        search::SearchClient(self)
    }

    pub(crate) fn build_url(uri: &str, request_type: RequestType) -> String {
        match request_type {
            RequestType::ApiRequest => format!("{}{}", Self::LINKEDIN_API_URL, uri),
        }
    }

    async fn execute_request(
        &self,
        request: wreq::RequestBuilder,
    ) -> Result<ApiResponse, LinkedInError> {
        // The permit covers both spacing and response streaming. Reserving a slot
        // before taking the permit would allow queued requests to depart in a burst.
        let _transport_permit = self.throttler.acquire_request().await?;
        let started = Instant::now();

        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        // Status, duration and byte counts are useful diagnostics without exposing
        // cookies, query filters, invitation secrets, or message contents.
        debug!(
            "HTTP response: status={}, elapsed={:?}, bytes={}",
            status,
            started.elapsed(),
            bytes.len()
        );
        Ok(ApiResponse { status, bytes })
    }

    /// Merges serializable parameters into a URL without overwriting existing query strings.
    fn append_query_params<P: Serialize + ?Sized>(
        url: &mut Url,
        params: Option<&P>,
    ) -> Result<(), LinkedInError> {
        let Some(p) = params else {
            return Ok(());
        };

        let query_string = serde_urlencoded::to_string(p).map_err(UrlError::from)?;
        if query_string.is_empty() {
            return Ok(());
        }

        match url.query() {
            Some(existing) if !existing.is_empty() => {
                let mut combined = String::with_capacity(existing.len() + 1 + query_string.len());
                combined.push_str(existing);
                combined.push('&');
                combined.push_str(&query_string);
                url.set_query(Some(&combined));
            }
            _ => {
                url.set_query(Some(&query_string));
            }
        }

        Ok(())
    }

    pub(crate) async fn _fetch<T: Serialize + ?Sized>(
        &self,
        uri: &str,
        params: Option<&T>,
        request_type: RequestType,
    ) -> Result<ApiResponse, LinkedInError> {
        let mut parsed_url =
            Url::parse(&Self::build_url(uri, request_type)).map_err(UrlError::from)?;

        Self::append_query_params(&mut parsed_url, params)?;
        let url_str = parsed_url.as_str();
        log_request("GET", 0);

        let request = self.client.get(url_str);
        self.execute_request(request).await
    }

    pub(crate) async fn _post<B: Serialize + ?Sized, P: Serialize + ?Sized>(
        &self,
        uri: &str,
        body: Option<&B>,
        params: Option<&P>,
        request_type: RequestType,
    ) -> Result<ApiResponse, LinkedInError> {
        let mut parsed_url =
            Url::parse(&Self::build_url(uri, request_type)).map_err(UrlError::from)?;

        Self::append_query_params(&mut parsed_url, params)?;

        let url_str = parsed_url.as_str();
        let mut request = self.client.post(url_str);

        if let Some(b) = body {
            let json_bytes = serde_json::to_vec(b)?;

            // Payloads may contain invitation secrets and private text. Logging
            // only their size makes the default safe even when DEBUG is enabled.
            log_request("POST", json_bytes.len());

            request = request
                .header(wreq::header::CONTENT_TYPE, "application/json")
                .body(json_bytes);
        } else {
            log_request("POST", 0);
        }

        self.execute_request(request).await
    }

    pub(crate) async fn _post_raw<P: Serialize + ?Sized>(
        &self,
        uri: &str,
        raw_body: String,
        params: Option<&P>,
        request_type: RequestType,
    ) -> Result<ApiResponse, LinkedInError> {
        let mut parsed_url =
            Url::parse(&Self::build_url(uri, request_type)).map_err(UrlError::from)?;

        Self::append_query_params(&mut parsed_url, params)?;
        let url_str = parsed_url.as_str();
        // Raw payloads follow the same policy as serialized JSON. Do not log the
        // URL either: its query may contain personal search criteria or identifiers.
        log_request("POST", raw_body.len());

        let request = self
            .client
            .post(url_str)
            .header(wreq::header::CONTENT_TYPE, "application/json")
            .body(raw_body);

        self.execute_request(request).await
    }
}

// This deliberately accepts no URL or body text: future callers cannot accidentally
// leak sensitive values while adding transport diagnostics.
fn log_request(method: &'static str, body_bytes: usize) {
    debug!("HTTP {} request: body_bytes={}", method, body_bytes);
}

pub(crate) struct ApiResponse {
    pub status: wreq::StatusCode,
    pub bytes: bytes::Bytes,
}

impl ApiResponse {
    pub fn json<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, LinkedInError> {
        serde_json::from_slice(&self.bytes).map_err(LinkedInError::from)
    }
}

pub(crate) trait ResponseExt {
    fn verify_success<D>(self, error_context: D) -> Result<Self, LinkedInError>
    where
        Self: Sized,
        D: std::fmt::Display;
}

impl ResponseExt for ApiResponse {
    fn verify_success<D: std::fmt::Display>(self, error_context: D) -> Result<Self, LinkedInError> {
        if self.status.is_success() {
            Ok(self)
        } else {
            Err(HttpApiError {
                status: self.status,
                context: error_context.to_string(),
            }
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_query_params_preserves_existing_query() {
        let mut url = Url::parse(
            "https://www.linkedin.com/voyager/api/graphql?queryId=test&variables=(foo:bar)",
        )
        .unwrap();
        let extra_params = [("count", "10"), ("start", "0")];

        Linkedin::append_query_params(&mut url, Some(&extra_params)).unwrap();

        assert_eq!(
            url.as_str(),
            "https://www.linkedin.com/voyager/api/graphql?queryId=test&variables=(foo:bar)&count=10&start=0"
        );
    }

    #[test]
    fn test_append_query_params_on_clean_url() {
        let mut url = Url::parse("https://www.linkedin.com/voyager/api/me").unwrap();
        let params = [("action", "test")];

        Linkedin::append_query_params(&mut url, Some(&params)).unwrap();

        assert_eq!(
            url.as_str(),
            "https://www.linkedin.com/voyager/api/me?action=test"
        );
    }

    #[test]
    fn test_append_query_params_none_does_not_mutate() {
        let original = "https://www.linkedin.com/voyager/api/graphql?queryId=test";
        let mut url = Url::parse(original).unwrap();

        Linkedin::append_query_params::<()>(&mut url, None).unwrap();

        assert_eq!(url.as_str(), original);
    }

    #[test]
    fn evasion_delay_rejects_negative_values() {
        let result = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(-1.0..=2.0);
        assert!(matches!(
            result,
            Err(LinkedInError::Config(
                ClientConfigError::InvalidEvasionDelay { .. }
            ))
        ));
    }

    #[test]
    fn evasion_delay_rejects_reversed_range() {
        let result = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(5.0..=2.0);
        assert!(matches!(
            result,
            Err(LinkedInError::Config(
                ClientConfigError::InvalidEvasionDelay { .. }
            ))
        ));
    }

    #[test]
    fn evasion_delay_rejects_non_finite_values() {
        for (min, max) in [(f64::NAN, 2.0), (1.0, f64::NAN), (f64::INFINITY, 2.0)] {
            let result =
                ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(min..=max);
            assert!(matches!(
                result,
                Err(LinkedInError::Config(
                    ClientConfigError::InvalidEvasionDelay { .. }
                ))
            ));
        }
    }

    #[test]
    fn evasion_delay_accepts_valid_range() {
        let config = ClientConfig::new(Browser::Firefox)
            .with_evasion_delay_in_seconds(1.5..=3.0)
            .unwrap();
        assert_eq!(config.evasion_delay, Some(1.5..=3.0));
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test(start_paused = true)]
    async fn slow_transport_does_not_consume_queued_request_spacing() {
        let throttler = Arc::new(EvasionThrottler::new(Some(1.0..=1.0)));
        let first = throttler.acquire_request().await.unwrap();
        let mut queued = Vec::new();
        for _ in 0..2 {
            let throttler = throttler.clone();
            queued.push(tokio::spawn(async move {
                let _permit = throttler.acquire_request().await.unwrap();
                Instant::now()
            }));
        }
        // Simulate a response taking longer than both previously reserved slots.
        // Paused Tokio time makes the regression deterministic and instantaneous.
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        drop(first);
        let second = queued.remove(0).await.unwrap();
        let third = queued.remove(0).await.unwrap();
        assert_eq!(third.duration_since(second), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn cancelling_a_waiting_request_preserves_fifo_and_releases_the_queue() {
        let throttler = Arc::new(EvasionThrottler::new(Some(1.0..=1.0)));
        let first = throttler.acquire_request().await.unwrap();
        let cancelled = {
            let t = throttler.clone();
            tokio::spawn(async move {
                let _permit = t.acquire_request().await.unwrap();
            })
        };
        tokio::task::yield_now().await;
        let next = {
            let t = throttler.clone();
            tokio::spawn(async move {
                let _permit = t.acquire_request().await.unwrap();
                Instant::now()
            })
        };
        cancelled.abort();
        assert!(cancelled.await.unwrap_err().is_cancelled());
        drop(first);
        next.await.unwrap();
        assert_eq!(throttler.transport_lock.available_permits(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn cancelling_during_spacing_releases_the_transport_permit() {
        let throttler = Arc::new(EvasionThrottler::new(Some(1.0..=1.0)));
        drop(throttler.acquire_request().await.unwrap());
        let waiting = {
            let t = throttler.clone();
            tokio::spawn(async move {
                let _permit = t.acquire_request().await.unwrap();
            })
        };
        tokio::task::yield_now().await;
        assert_eq!(throttler.transport_lock.available_permits(), 0);
        waiting.abort();
        assert!(waiting.await.unwrap_err().is_cancelled());
        assert_eq!(throttler.transport_lock.available_permits(), 1);
        drop(throttler.acquire_request().await.unwrap());
    }

    #[tokio::test]
    async fn directly_constructed_invalid_throttler_returns_error_instead_of_panicking() {
        for range in [f64::NAN..=1.0, -1.0..=2.0, 2.0..=1.0, f64::MAX..=f64::MAX] {
            let throttler = EvasionThrottler::new(Some(range));
            assert!(matches!(
                throttler.acquire_request().await,
                Err(LinkedInError::Config(_))
            ));
            // Returning an error must also return the transport permit to the pool.
            assert_eq!(throttler.transport_lock.available_permits(), 1);
        }
    }

    #[test]
    fn unrepresentable_delay_is_rejected_at_both_configuration_boundaries() {
        assert!(
            ClientConfig::new(Browser::Firefox)
                .with_evasion_delay_in_seconds(f64::MAX..=f64::MAX)
                .is_err()
        );
        let mut config = ClientConfig::new(Browser::Firefox);
        config.evasion_delay = Some(-1.0..=1.0);
        // Cookie extraction must not even run for an invalid direct assignment.
        assert!(matches!(
            Linkedin::with_config(config),
            Err(LinkedInError::Config(_))
        ));
    }

    struct CapturedLogs(Mutex<Vec<String>>);
    static LOGS: CapturedLogs = CapturedLogs(Mutex::new(Vec::new()));
    impl log::Log for CapturedLogs {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.target().starts_with("leankedin_park::api")
        }
        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                self.0.lock().unwrap().push(record.args().to_string());
            }
        }
        fn flush(&self) {}
    }

    #[tokio::test]
    async fn request_logs_never_include_queries_or_serialized_payloads() {
        log::set_logger(&LOGS).unwrap();
        log::set_max_level(log::LevelFilter::Debug);
        let client = Linkedin {
            client: Client::builder().build().unwrap(),
            config: ClientConfig::new(Browser::Firefox),
            profile_urn: tokio::sync::OnceCell::new(),
            throttler: EvasionThrottler::new(None),
        };
        // Close the transport before executing: real request-building and logging
        // run, but no request can ever reach the network or use browser cookies.
        client.throttler.transport_lock.close();
        let payload = serde_json::json!({"message": "PRIVATE_MESSAGE_SENTINEL", "invitationSharedSecret": "SECRET_SENTINEL"});
        assert!(
            client
                ._post::<_, ()>(
                    "/test?filter=QUERY_SENTINEL",
                    Some(&payload),
                    None,
                    RequestType::ApiRequest
                )
                .await
                .is_err()
        );
        assert!(
            client
                ._post_raw::<()>(
                    "/test?filter=QUERY_SENTINEL",
                    payload.to_string(),
                    None,
                    RequestType::ApiRequest
                )
                .await
                .is_err()
        );
        assert!(
            client
                ._fetch::<()>("/test?filter=QUERY_SENTINEL", None, RequestType::ApiRequest)
                .await
                .is_err()
        );
        let output = LOGS.0.lock().unwrap().join("\n");
        assert!(output.contains("HTTP POST request: body_bytes="));
        assert!(output.contains("HTTP GET request:"));
        for sensitive in [
            "PRIVATE_MESSAGE_SENTINEL",
            "SECRET_SENTINEL",
            "QUERY_SENTINEL",
        ] {
            assert!(
                !output.contains(sensitive),
                "sensitive input leaked into logs"
            );
        }
    }
}
