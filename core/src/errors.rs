//! Comprehensive error types for the `leankedin-park` library.
//!
//! Categorizes network, configuration, validation, serialization, and API domain errors.

use wreq::header::{InvalidHeaderName, InvalidHeaderValue};

/// Errors related to cookie and session management.
#[derive(Debug, thiserror::Error)]
pub enum CookieError {
    /// Some essential cookies were missing from the session.
    #[error("Missing {} essential cookie(s): {cookies:?}", cookies.len())]
    MissingSomeEssential {
        /// Names of the missing cookies.
        cookies: Vec<&'static str>,
    },

    /// None of the essential cookies were found.
    #[error("Missing all the essential cookies: {0:?}")]
    MissingAllEssential(&'static [&'static str]),

    /// No cookies whatsoever were extracted from the specified browser or file.
    #[error("Zero cookies were found")]
    ZeroCookieFound,

    /// An essential authentication cookie has expired.
    #[error("The essential cookie '{0}' has expired. Please update your cookies.")]
    Expired(String),

    /// The `JSESSIONID` cookie was present but could not be parsed to create the CSRF token.
    #[error("The JSESSIONID cookie exists but we can't create the csrf_token from it")]
    CantCreateCsrfToken,
}

/// Errors related to client and environment configuration.
#[derive(Debug, thiserror::Error)]
pub enum ClientConfigError {
    /// Safari was selected as the cookie extraction browser on a non-macOS platform.
    #[error("The browser Safari can only be used on MacOS")]
    SafariCanOnlyBeUsedOnMacOS,

    /// An error occurred in the underlying `rookie` cookie extraction engine.
    #[error("The following error appeared from the Rookie crate: {0}")]
    RookieCrate(String),

    /// Evasion delay bounds are invalid or cannot be represented by the timer.
    #[error(
        "Invalid evasion delay range: bounds must be finite, non-negative, ordered and representable by the timer (got {min}..={max})"
    )]
    InvalidEvasionDelay {
        /// Lower bound provided.
        min: f64,
        /// Upper bound provided.
        max: f64,
    },
}

/// Errors related to messaging operations.
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    /// The message length exceeded the 300 character limit for connection notes.
    #[error("Message too long. Max size is 300 characters")]
    MessageTooLong,
}

/// Errors related to URN validation and parsing.
#[derive(Debug, thiserror::Error)]
pub enum UrnError {
    /// No identifier segment was found in the provided string.
    #[error("Failed to find the ID in the following string: {0}")]
    MissingId(String),
    /// Failed to extract pure ID from compound URN.
    #[error("Failed to extract the URN: {0}")]
    FailToExtractUrn(String),
    /// The format of the URN or identifier is invalid.
    #[error("Invalid URN format: {0}")]
    InvalidFormat(String),
}

/// Errors related to public profile vanity names.
#[derive(Debug, thiserror::Error)]
pub enum VanityNameError {
    /// The vanity name was empty or whitespace only.
    #[error("A vanity name cannot be empty or consist solely of spaces")]
    Empty,
    /// The vanity name contained forbidden characters (only alphanumeric and hyphens allowed).
    #[error("A vanity name cannot contain any special characters other than hyphen: {0}")]
    InvalidCharacters(String),
}

/// Errors related to URL construction and parameter encoding.
#[derive(Debug, thiserror::Error)]
pub enum UrlError {
    /// Failed to serialize parameters to `x-www-form-urlencoded`.
    #[error("Failed to encode the URL parameters to the URL-encoded format")]
    CantEncode(#[from] serde_urlencoded::ser::Error),

    /// Failed to parse a URL string.
    #[error("Failed to parse the URL")]
    CantParse(#[from] url::ParseError),
}

/// Errors related to post publishing.
#[derive(Debug, thiserror::Error)]
pub enum PublishPostError {
    /// The post content text was empty or whitespace only.
    #[error("Post content cannot be empty or only whitespaces")]
    EmptyContent,

    /// Post content character count exceeded the 3,000 character maximum.
    #[error("Post content is too long: {0} characters. Maximum allowed is {1}.")]
    ContentTooLong(usize, u32),
}

/// HTTP API error indicating a non-2xx status code response.
#[derive(Debug, thiserror::Error)]
#[error("{context} (HTTP {status})")]
pub struct HttpApiError {
    /// The HTTP status code returned by LinkedIn.
    pub status: wreq::StatusCode,
    /// Contextual description of the failed action.
    pub context: String,
}

/// Search failures that must not be mistaken for an empty or complete result list.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// JSON syntax or required typed fields do not match the search schema.
    #[error("Invalid search response: {0}")]
    InvalidResponse(#[source] serde_json::Error),
    /// Required collection structure or pagination metadata changed.
    #[error("Unexpected search response: {0}")]
    UnexpectedResponse(&'static str),
    /// A result was present but could not be converted to the public model.
    #[error("Invalid search result at index {index}: {source}")]
    InvalidResult {
        index: usize,
        #[source]
        source: Box<LinkedInError>,
    },
    /// A safety guard stopped retrieval before reaching the requested end.
    #[error("Search incomplete after collecting {collected} results: {reason}")]
    Incomplete {
        collected: usize,
        reason: &'static str,
    },
}

/// Master error enum aggregating all system and domain errors in `leankedin-park`.
#[derive(Debug, thiserror::Error)]
pub enum LinkedInError {
    /// Underlying HTTP transport network error.
    #[error("Network request failed: {0}")]
    Network(#[from] wreq::Error),

    /// File or standard I/O error.
    #[error("IO operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON parsing/serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Invalid HTTP header name.
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),

    /// Invalid HTTP header value.
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    /// Cookie or session authentication error.
    #[error(transparent)]
    Cookie(#[from] CookieError),

    /// Client or environment configuration error.
    #[error(transparent)]
    Config(#[from] ClientConfigError),

    /// Messaging error.
    #[error(transparent)]
    Messaging(#[from] MessagingError),

    /// URN identifier validation or extraction error.
    #[error(transparent)]
    Urn(#[from] UrnError),

    /// Vanity name validation error.
    #[error(transparent)]
    VanityName(#[from] VanityNameError),

    /// URL formatting or parameter encoding error.
    #[error(transparent)]
    Url(#[from] UrlError),

    /// Post creation or validation error.
    #[error(transparent)]
    PublishPost(#[from] PublishPostError),

    /// Non-success HTTP API status error.
    #[error(transparent)]
    Http(#[from] HttpApiError),

    /// Incompatible search response or incomplete pagination.
    #[error(transparent)]
    Search(#[from] SearchError),

    /// General API error message returned by LinkedIn.
    #[error("LinkedIn API error: {0}")]
    Api(String),
}

impl From<std::convert::Infallible> for LinkedInError {
    #[inline]
    fn from(err: std::convert::Infallible) -> Self {
        match err {}
    }
}
