// PyO3 generates FromPyObject implementations outside each enum, so the
// lint allowance must cover this module containing only Copy pyclass enums.
#[allow(
    clippy::clone_on_copy,
    reason = "PyO3 from_py_object generates clone calls on Copy enums"
)]
mod enums {
    use pyo3::prelude::*;

    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, Default, PartialEq, Eq)]
    pub enum Browser {
        Chrome,
        // Also used for HTTP emulation when cookies are supplied explicitly.
        #[default]
        Firefox,
        Edge,
        Safari,
        Brave,
        Chromium,
    }

    /// Possible actions when responding to a LinkedIn connection invitation.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum InvitationAction {
        Accept,
        Reject,
    }

    /// Visibility levels when publishing a new post.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum VisibilityType {
        Anyone,
        ConnectionsOnly,
    }

    /// Determines who is allowed to comment on your published posts.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum AllowedCommentersScope {
        Anyone,
        ConnectionsOnly,
        NoOne,
    }

    /// Types of reactions applicable to a LinkedIn post.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum ReactionType {
        Like,
        Praise,
        Appreciation,
        Empathy,
        Interest,
        Entertainment,
    }

    /// Network depths to target when executing searches (e.g., 1st degree connections).
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum NetworkDepth {
        FirstDegree,
        SecondDegree,
        ThirdDegree,
    }

    /// Relationship distance level relative to your current profile.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum ProfileDistance {
        Myself,
        FirstDegree,
        SecondDegree,
        ThirdDegree,
        OutOfNetwork,
        Unknown,
    }

    /// Inbox categories for LinkedIn messaging.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum ConversationCategory {
        Inbox,
        PrimaryInbox,
        Inmail,
        Archive,
        Unknown,
    }
}
pub use enums::{
    AllowedCommentersScope, Browser, ConversationCategory, InvitationAction, NetworkDepth,
    ProfileDistance, ReactionType, VisibilityType,
};

use pyo3::PyRefMut;
use pyo3::exceptions::{PyException, PyRuntimeError, PyTypeError, PyValueError};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Runtime};

use ::leankedin_park::errors as core_errors;

use ::leankedin_park::models::account::{
    LinkedInProfile as CoreLinkedInProfile, ProfileViews as CoreProfileViews,
};
use ::leankedin_park::models::connections::{
    ConnectionRequest, InvitationAction as CoreInvitationAction,
    ReceivedInvitation as CoreReceivedInvitation,
};
use ::leankedin_park::models::identifiers::{AsVanityStr, InvitationId, ProfileId, VanityName};
use ::leankedin_park::models::messaging::{
    Category as CoreConversationCategory, ConversationData as CoreConversationData,
    ConversationParticipant as CoreConversationParticipant,
};
use ::leankedin_park::models::people::{
    BaseProfile as CoreBaseProfile, ConnectionDegree as CoreConnectionDegree, Date as CoreDate,
    FullProfile as CoreFullProfile, NetworkInfo as CoreNetworkInfo,
};
use ::leankedin_park::models::posts::{
    AllowedCommentersScope as CoreAllowedCommentersScope, NewPost,
    ReactionType as CoreReactionType, VisibilityType as CoreVisibilityType,
};
use ::leankedin_park::models::search::{
    People as CorePeople, SearchPeopleBuilder as CoreSearchBuilder,
};
use ::leankedin_park::{
    Browser as CoreBrowser, ClientConfig as CoreClientConfig, Linkedin as CoreLinkedin,
};

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize global Tokio runtime")
});

pyo3::create_exception!(leankedin_park, LinkedInAPIError, PyException);

pyo3::create_exception!(leankedin_park, CookieError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, ClientConfigError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, MessagingError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, UrnError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, VanityNameError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, UrlError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, PublishPostError, LinkedInAPIError);
pyo3::create_exception!(leankedin_park, HttpApiError, LinkedInAPIError);
// Search schema failures are distinct from an ordinary successful empty result.
pyo3::create_exception!(leankedin_park, SearchError, LinkedInAPIError);

pub struct PyLinkedInError(core_errors::LinkedInError);

impl From<core_errors::LinkedInError> for PyLinkedInError {
    #[inline]
    fn from(err: core_errors::LinkedInError) -> Self {
        Self(err)
    }
}

impl From<core_errors::UrnError> for PyLinkedInError {
    #[inline]
    fn from(err: core_errors::UrnError) -> Self {
        Self(core_errors::LinkedInError::Urn(err))
    }
}

impl From<core_errors::VanityNameError> for PyLinkedInError {
    #[inline]
    fn from(err: core_errors::VanityNameError) -> Self {
        Self(core_errors::LinkedInError::VanityName(err))
    }
}

impl From<PyLinkedInError> for PyErr {
    fn from(err: PyLinkedInError) -> PyErr {
        let msg = err.0.to_string();
        match err.0 {
            core_errors::LinkedInError::Cookie(_) => CookieError::new_err(msg),
            core_errors::LinkedInError::Config(_) => ClientConfigError::new_err(msg),
            core_errors::LinkedInError::Messaging(_) => MessagingError::new_err(msg),
            core_errors::LinkedInError::Urn(_) => UrnError::new_err(msg),
            core_errors::LinkedInError::VanityName(_) => VanityNameError::new_err(msg),
            core_errors::LinkedInError::Url(_) => UrlError::new_err(msg),
            core_errors::LinkedInError::PublishPost(_) => PublishPostError::new_err(msg),
            core_errors::LinkedInError::Search(_) => SearchError::new_err(msg),
            core_errors::LinkedInError::Http(http_err) => Python::attach(|py| {
                let status = http_err.status.as_u16();
                let context = http_err.context.clone();
                let py_err = HttpApiError::new_err(http_err.to_string());
                let value = py_err.value(py);
                let _ = value.setattr("status", status);
                let _ = value.setattr("context", context);
                py_err
            }),
            _ => LinkedInAPIError::new_err(msg),
        }
    }
}

pub type PyLnResult<T> = Result<T, PyLinkedInError>;

impl From<Browser> for CoreBrowser {
    #[inline]
    fn from(val: Browser) -> Self {
        match val {
            Browser::Chrome => CoreBrowser::Chrome,
            Browser::Firefox => CoreBrowser::Firefox,
            Browser::Edge => CoreBrowser::Edge,
            Browser::Safari => CoreBrowser::Safari,
            Browser::Brave => CoreBrowser::Brave,
            Browser::Chromium => CoreBrowser::Chromium,
        }
    }
}

/// Configuration for the LinkedIn client.
///
/// Allows defining the target browser for cookie extraction, evasion delays (anti-bot),
/// and providing manual cookie sources if necessary.
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct ClientConfig {
    pub(crate) inner: CoreClientConfig,
}

#[pymethods]
impl ClientConfig {
    /// Initializes a new configuration.
    ///
    /// Args:
    ///     browser (Browser, optional): Browser used for automatic cookie
    ///     extraction and HTTP emulation. Defaults to Firefox.
    ///
    /// Returns:
    ///     ClientConfig: A new instance of the client configuration.
    #[new]
    #[pyo3(signature = (browser=None))]
    pub fn new(browser: Option<Browser>) -> Self {
        Self {
            inner: CoreClientConfig::new(browser.unwrap_or_default().into()),
        }
    }

    /// Creates a configuration that obtains cookies from the specified browser.
    #[staticmethod]
    #[pyo3(signature = (browser))]
    pub fn from_browser(browser: Browser) -> Self {
        Self::new(Some(browser))
    }

    /// Creates a configuration from LinkedIn's mandatory authentication cookies.
    /// The browser argument is intentionally omitted because no browser profile
    /// is read. Firefox is used internally for HTTP emulation.
    #[staticmethod]
    #[pyo3(signature = (li_at, jsessionid))]
    pub fn from_cookies(li_at: &str, jsessionid: &str) -> Self {
        Self {
            inner: CoreClientConfig::new(CoreBrowser::Firefox)
                .with_manual_cookies(li_at, jsessionid),
        }
    }

    /// Creates a configuration from a Netscape-format cookies file.
    ///
    /// No browser profile is read. Firefox is used internally for HTTP emulation.
    #[staticmethod]
    #[pyo3(signature = (path))]
    pub fn from_cookies_file(path: PathBuf) -> Self {
        Self {
            inner: CoreClientConfig::new(CoreBrowser::Firefox).with_cookies_file(path),
        }
    }

    /// Applies a random delay in seconds between each request to avoid detection.
    ///
    /// Args:
    ///     min (float): The minimum delay in seconds.
    ///     max (float): The maximum delay in seconds.
    ///
    /// Returns:
    ///     ClientConfig: The updated configuration instance (mutated in-place).
    #[pyo3(signature = (min, max))]
    pub fn with_evasion_delay_in_seconds<'py>(
        mut slf: PyRefMut<'py, Self>,
        min: f64,
        max: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Validate here as well as in the core so Python users get a native ValueError
        // at the configuration call itself, instead of a deferred client-construction error.
        if !min.is_finite() || !max.is_finite() || min < 0.0 || max < 0.0 || min > max {
            return Err(PyValueError::new_err(format!(
                "Invalid evasion delay range: min and max must be finite and non-negative, and min <= max (got {min}..={max})"
            )));
        }

        let validated = CoreClientConfig::new(slf.inner.browser)
            .with_evasion_delay_in_seconds(min..=max)
            .map_err(PyLinkedInError::from)?;
        slf.inner.evasion_delay = validated.evasion_delay;
        Ok(slf)
    }

    /// Completely disables evasion delays.
    ///
    /// Warning: This is not recommended for production as it greatly increases the
    /// chances of your account being restricted by LinkedIn's anti-bot systems.
    ///
    /// Returns:
    ///     ClientConfig: The updated configuration instance (mutated in-place).
    #[pyo3(signature = ())]
    pub fn without_evasion_delay<'py>(mut slf: PyRefMut<'py, Self>) -> PyRefMut<'py, Self> {
        slf.inner.evasion_delay = None;
        slf
    }

    /// Manually specifies the required cookies instead of extracting them from a browser.
    ///
    /// Args:
    ///     li_at (str): The value of the `li_at` cookie.
    ///     jsessionid (str): The value of the `JSESSIONID` cookie (including quotes if present).
    ///
    /// Returns:
    ///     ClientConfig: The updated configuration instance (mutated in-place).
    #[pyo3(signature = (li_at, jsessionid))]
    pub fn with_manual_cookies<'py>(
        mut slf: PyRefMut<'py, Self>,
        li_at: &str,
        jsessionid: &str,
    ) -> PyRefMut<'py, Self> {
        slf.inner.cookie_source = ::leankedin_park::api::CookieSource::Manual {
            li_at: li_at.to_owned(),
            jsessionid: jsessionid.to_owned(),
        };
        slf
    }

    /// Loads cookies from a text file formatted in the Netscape standard format.
    ///
    /// Args:
    ///     path (str): The absolute or relative path to the cookies file.
    ///
    /// Returns:
    ///     ClientConfig: The updated configuration instance (mutated in-place).
    #[pyo3(signature = (path))]
    pub fn with_cookies_file<'py>(
        mut slf: PyRefMut<'py, Self>,
        path: PathBuf,
    ) -> PyRefMut<'py, Self> {
        slf.inner.cookie_source = ::leankedin_park::api::CookieSource::NetscapeFile(path);
        slf
    }

    fn __repr__(&self) -> String {
        format!(
            "<ClientConfig browser={:?} has_custom_delay={}>",
            self.inner.browser,
            self.inner.evasion_delay.is_some()
        )
    }
}

#[pymethods]
impl InvitationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Reject => "reject",
        }
    }
}

impl From<InvitationAction> for CoreInvitationAction {
    #[inline]
    fn from(val: InvitationAction) -> Self {
        match val {
            InvitationAction::Accept => CoreInvitationAction::Accept,
            InvitationAction::Reject => CoreInvitationAction::Reject,
        }
    }
}

impl From<VisibilityType> for CoreVisibilityType {
    #[inline]
    fn from(val: VisibilityType) -> Self {
        match val {
            VisibilityType::Anyone => CoreVisibilityType::Anyone,
            VisibilityType::ConnectionsOnly => CoreVisibilityType::ConnectionsOnly,
        }
    }
}

impl From<AllowedCommentersScope> for CoreAllowedCommentersScope {
    #[inline]
    fn from(val: AllowedCommentersScope) -> Self {
        match val {
            AllowedCommentersScope::Anyone => CoreAllowedCommentersScope::Anyone,
            AllowedCommentersScope::ConnectionsOnly => CoreAllowedCommentersScope::ConnectionsOnly,
            AllowedCommentersScope::NoOne => CoreAllowedCommentersScope::NoOne,
        }
    }
}

impl From<ReactionType> for CoreReactionType {
    #[inline]
    fn from(val: ReactionType) -> Self {
        match val {
            ReactionType::Like => CoreReactionType::Like,
            ReactionType::Praise => CoreReactionType::Praise,
            ReactionType::Appreciation => CoreReactionType::Appreciation,
            ReactionType::Empathy => CoreReactionType::Empathy,
            ReactionType::Interest => CoreReactionType::Interest,
            ReactionType::Entertainment => CoreReactionType::Entertainment,
        }
    }
}

impl From<NetworkDepth> for CoreConnectionDegree {
    #[inline]
    fn from(val: NetworkDepth) -> Self {
        match val {
            NetworkDepth::FirstDegree => CoreConnectionDegree::First,
            NetworkDepth::SecondDegree => CoreConnectionDegree::Second,
            NetworkDepth::ThirdDegree => CoreConnectionDegree::Third,
        }
    }
}

#[pymethods]
impl ProfileDistance {
    pub fn search_code(&self) -> Option<&'static str> {
        match self {
            Self::FirstDegree => Some("F"),
            Self::SecondDegree => Some("S"),
            Self::ThirdDegree => Some("O"),
            _ => None,
        }
    }

    #[staticmethod]
    pub fn from_search_code(code: &str) -> Option<Self> {
        match code {
            "F" => Some(Self::FirstDegree),
            "S" => Some(Self::SecondDegree),
            "O" => Some(Self::ThirdDegree),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Myself => "Myself",
            Self::FirstDegree => "FirstDegree",
            Self::SecondDegree => "SecondDegree",
            Self::ThirdDegree => "ThirdDegree",
            Self::OutOfNetwork => "OutOfNetwork",
            Self::Unknown => "Unknown",
        }
    }
}

impl From<CoreConnectionDegree> for ProfileDistance {
    #[inline]
    fn from(val: CoreConnectionDegree) -> Self {
        match val {
            CoreConnectionDegree::Myself => Self::Myself,
            CoreConnectionDegree::First => Self::FirstDegree,
            CoreConnectionDegree::Second => Self::SecondDegree,
            CoreConnectionDegree::Third => Self::ThirdDegree,
            CoreConnectionDegree::OutOfNetwork => Self::OutOfNetwork,
            CoreConnectionDegree::Unknown => Self::Unknown,
        }
    }
}

impl From<CoreConversationCategory> for ConversationCategory {
    #[inline]
    fn from(val: CoreConversationCategory) -> Self {
        match val {
            CoreConversationCategory::Inbox => Self::Inbox,
            CoreConversationCategory::PrimaryInbox => Self::PrimaryInbox,
            CoreConversationCategory::Inmail => Self::Inmail,
            CoreConversationCategory::Archive => Self::Archive,
            CoreConversationCategory::Unknown => Self::Unknown,
        }
    }
}

/// Represents the viewing statistics of your profile.
#[pyclass(dict)]
pub struct ProfileViews {
    #[pyo3(get)]
    pub num_views: i64,
    #[pyo3(get)]
    pub time_frame: String,
}

#[pymethods]
impl ProfileViews {
    fn __repr__(&self) -> String {
        format!(
            "<ProfileViews num_views={} time_frame=\"{}\">",
            self.num_views, self.time_frame
        )
    }
}

impl From<CoreProfileViews> for ProfileViews {
    #[inline]
    fn from(v: CoreProfileViews) -> Self {
        Self {
            num_views: v.num_views,
            time_frame: v.time_frame,
        }
    }
}

/// Information regarding a participant within a LinkedIn conversation.
#[pyclass(from_py_object, dict)]
#[derive(Clone)]
pub struct ConversationParticipantData {
    #[pyo3(get)]
    pub profile_url: Option<String>,
    #[pyo3(get)]
    pub first_name: Option<String>,
    #[pyo3(get)]
    pub last_name: Option<String>,
    #[pyo3(get)]
    pub distance: Option<ProfileDistance>,
    #[pyo3(get)]
    pub urn: String,
}

impl From<CoreConversationParticipant> for ConversationParticipantData {
    #[inline]
    fn from(p: CoreConversationParticipant) -> Self {
        Self {
            profile_url: p.profile_url,
            first_name: p.first_name,
            last_name: p.last_name,
            distance: p.distance.map(Into::into),
            urn: p.urn.to_string(),
        }
    }
}

#[pymethods]
impl ConversationParticipantData {
    fn __repr__(&self) -> String {
        let first = self.first_name.as_deref().unwrap_or("");
        let last = self.last_name.as_deref().unwrap_or("");
        format!(
            "<ConversationParticipantData urn=\"{}\" name=\"{} {}\">",
            self.urn, first, last
        )
    }
}

/// Core information about a messaging conversation.
#[pyclass(dict)]
pub struct ConversationData {
    #[pyo3(get)]
    pub urn: String,
    #[pyo3(get)]
    pub unread_count: u32,
    #[pyo3(get)]
    pub is_read: bool,
    #[pyo3(get)]
    pub is_group: bool,
    #[pyo3(get)]
    pub categories: Vec<ConversationCategory>,
    #[pyo3(get)]
    pub created_at: u64,
    #[pyo3(get)]
    pub last_activity_at: u64,
    #[pyo3(get)]
    pub last_read_at: Option<u64>,
    #[pyo3(get)]
    pub participants: Vec<ConversationParticipantData>,
}

impl From<CoreConversationData> for ConversationData {
    fn from(c: CoreConversationData) -> Self {
        Self {
            urn: c.urn.to_string(),
            unread_count: c.unread_count,
            is_read: c.is_read,
            is_group: c.is_group,
            categories: c.categories.into_iter().map(Into::into).collect(),
            created_at: c.created_at,
            last_activity_at: c.last_activity_at,
            last_read_at: c.last_read_at,
            participants: c.participants.into_iter().map(Into::into).collect(),
        }
    }
}

#[pymethods]
impl ConversationData {
    pub fn is_human_conversation(&self) -> bool {
        self.participants.len() >= 2
    }

    fn __repr__(&self) -> String {
        format!(
            "<ConversationData urn=\"{}\" unread={} is_group={} participants_number={}>",
            self.urn,
            self.unread_count,
            self.is_group,
            self.participants.len()
        )
    }
}

/// Data encapsulating a received network connection invitation.
#[pyclass(dict)]
pub struct InvitationData {
    #[pyo3(get)]
    pub first_name: String,
    #[pyo3(get)]
    pub last_name: String,
    #[pyo3(get)]
    pub occupation: Option<String>,
    #[pyo3(get)]
    pub unseen: bool,
    #[pyo3(get)]
    pub entity_urn: String,
    #[pyo3(get)]
    pub invitation_type: String,
    #[pyo3(get)]
    pub custom_message: bool,
    #[pyo3(get)]
    pub shared_secret: String,
    #[pyo3(get)]
    pub sent_time: u64,
}

impl From<CoreReceivedInvitation> for InvitationData {
    #[inline]
    fn from(i: CoreReceivedInvitation) -> Self {
        Self {
            first_name: i.first_name,
            last_name: i.last_name,
            occupation: i.occupation,
            unseen: i.unseen,
            entity_urn: i.entity_urn.to_string(),
            invitation_type: i.invitation_type,
            custom_message: i.custom_message,
            shared_secret: i.shared_secret,
            sent_time: i.sent_time,
        }
    }
}

#[pymethods]
impl InvitationData {
    fn __repr__(&self) -> String {
        format!(
            "<InvitationData entity_urn=\"{}\" name=\"{} {}\">",
            self.entity_urn, self.first_name, self.last_name
        )
    }
}

#[pyclass(skip_from_py_object)]
#[derive(Clone, Copy)]
pub struct DateData {
    #[pyo3(get)]
    pub month: Option<u8>,
    #[pyo3(get)]
    pub day: Option<u8>,
    #[pyo3(get)]
    pub year: Option<i32>,
}

impl From<CoreDate> for DateData {
    #[inline]
    fn from(d: CoreDate) -> Self {
        Self {
            month: d.month,
            day: d.day,
            year: d.year,
        }
    }
}

#[pymethods]
impl DateData {
    /// Returns a JSON-compatible dictionary, preserving missing date components as None.
    pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("year", self.year)?;
        dict.set_item("month", self.month)?;
        dict.set_item("day", self.day)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<DateData year={:?} month={:?} day={:?}>",
            self.year, self.month, self.day
        )
    }
}

#[pyclass(dict)]
pub struct LinkedInProfileData {
    #[pyo3(get)]
    pub first_name: String,
    #[pyo3(get)]
    pub last_name: String,
    #[pyo3(get)]
    pub occupation: Option<String>,
    #[pyo3(get)]
    pub vanity_name: String,
    #[pyo3(get)]
    pub custom_pronoun: Option<String>,
    #[pyo3(get)]
    pub profile_urn: String,
    #[pyo3(get)]
    pub premium_subscriber: bool,
}

#[pymethods]
impl LinkedInProfileData {
    fn __repr__(&self) -> String {
        let name = format!("{} {}", self.first_name, self.last_name);
        format!(
            "<LinkedInProfileData profile_urn={:?} name={name:?}>",
            self.profile_urn
        )
    }
}

#[pyclass(dict)]
pub struct BaseProfileData {
    #[pyo3(get)]
    pub first_name: String,
    #[pyo3(get)]
    pub last_name: String,
    #[pyo3(get)]
    pub public_identifier: String,
    #[pyo3(get)]
    pub profile_urn: String,
    #[pyo3(get)]
    pub headline: Option<String>,
    #[pyo3(get)]
    pub pronoun: Option<String>,
    #[pyo3(get)]
    pub full_location: Option<String>,
    #[pyo3(get)]
    pub country: Option<String>,
    #[pyo3(get)]
    pub country_iso_code: Option<String>,
    #[pyo3(get)]
    pub postal_code: Option<String>,
    #[pyo3(get)]
    pub creator: Option<bool>,
    #[pyo3(get)]
    pub premium: Option<bool>,
    #[pyo3(get)]
    pub influencer: Option<bool>,
    #[pyo3(get)]
    pub birth_date_on: Option<DateData>,
    #[pyo3(get)]
    pub member_distance: ProfileDistance,
    #[pyo3(get)]
    pub connected_since: Option<i64>,
}

#[pymethods]
impl BaseProfileData {
    fn __repr__(&self) -> String {
        let name = format!("{} {}", self.first_name, self.last_name);
        format!(
            "<BaseProfileData profile_urn={:?} name={name:?}>",
            self.profile_urn
        )
    }
}

#[pyclass(dict)]
pub struct FullProfileData {
    #[pyo3(get)]
    pub first_name: String,
    #[pyo3(get)]
    pub last_name: String,
    #[pyo3(get)]
    pub public_identifier: String,
    #[pyo3(get)]
    pub headline: Option<String>,
    #[pyo3(get)]
    pub pronoun: Option<String>,
    #[pyo3(get)]
    pub is_creator: Option<bool>,
    #[pyo3(get)]
    pub is_premium: Option<bool>,
    #[pyo3(get)]
    pub is_influencer: Option<bool>,
    #[pyo3(get)]
    pub birth_date_on: Option<DateData>,
    #[pyo3(get)]
    pub full_location: Option<String>,
    #[pyo3(get)]
    pub country: Option<String>,
    #[pyo3(get)]
    pub country_iso_code: Option<String>,
    #[pyo3(get)]
    pub postal_code: Option<String>,
    #[pyo3(get)]
    pub member_distance: ProfileDistance,
    #[pyo3(get)]
    pub connected_since: Option<i64>,
    #[pyo3(get)]
    pub follower_count: Option<u32>,
    #[pyo3(get)]
    pub total_connections: Option<u32>,
    #[pyo3(get)]
    pub is_following: Option<bool>,
    #[pyo3(get)]
    pub profile_urn: String,
}

impl From<CoreLinkedInProfile> for LinkedInProfileData {
    #[inline]
    fn from(p: CoreLinkedInProfile) -> Self {
        Self {
            first_name: p.first_name,
            last_name: p.last_name,
            occupation: p.occupation,
            vanity_name: p.vanity_name.to_string(),
            custom_pronoun: p.custom_pronoun,
            profile_urn: p.profile_urn.to_string(),
            premium_subscriber: p.premium_subscriber,
        }
    }
}

impl From<CoreBaseProfile> for BaseProfileData {
    #[inline]
    fn from(p: CoreBaseProfile) -> Self {
        Self {
            first_name: p.first_name,
            last_name: p.last_name,
            public_identifier: p.public_identifier.to_string(),
            profile_urn: p.profile_urn.to_string(),
            headline: p.headline,
            pronoun: p.pronoun,
            full_location: p.full_location,
            country: p.country,
            country_iso_code: p.country_iso_code,
            postal_code: p.postal_code,
            creator: p.creator,
            premium: p.premium,
            influencer: p.influencer,
            birth_date_on: p.birth_date_on.map(Into::into),
            member_distance: p.member_distance.into(),
            connected_since: p.connected_since,
        }
    }
}

impl From<CoreFullProfile> for FullProfileData {
    #[inline]
    fn from(p: CoreFullProfile) -> Self {
        Self {
            first_name: p.first_name,
            last_name: p.last_name,
            public_identifier: p.public_identifier.to_string(),
            headline: p.headline,
            pronoun: p.pronoun,
            is_creator: p.is_creator,
            is_premium: p.is_premium,
            is_influencer: p.is_influencer,
            birth_date_on: p.birth_date_on.map(Into::into),
            full_location: p.full_location,
            country: p.country,
            country_iso_code: p.country_iso_code,
            postal_code: p.postal_code,
            member_distance: p.member_distance.into(),
            connected_since: p.connected_since,
            follower_count: p.follower_count,
            total_connections: p.total_connections,
            is_following: p.is_following,
            profile_urn: p.profile_urn.to_string(),
        }
    }
}

#[pymethods]
impl FullProfileData {
    /// Backward-compatible alias for the historical `ProfileData.occupation` field.
    #[getter]
    pub fn occupation(&self) -> Option<&str> {
        self.headline.as_deref()
    }

    /// Backward-compatible alias for the historical `ProfileData.distance` field.
    #[getter]
    pub fn distance(&self) -> ProfileDistance {
        self.member_distance
    }

    /// Returns JSON-compatible values, including a nested birth date dictionary or None.
    pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("first_name", &self.first_name)?;
        dict.set_item("last_name", &self.last_name)?;
        dict.set_item("public_identifier", &self.public_identifier)?;
        dict.set_item("headline", &self.headline)?;
        dict.set_item("pronoun", &self.pronoun)?;
        dict.set_item("is_creator", self.is_creator)?;
        dict.set_item("is_premium", self.is_premium)?;
        dict.set_item("is_influencer", self.is_influencer)?;
        dict.set_item(
            "birth_date_on",
            self.birth_date_on
                .as_ref()
                .map(|d| d.to_dict(py))
                .transpose()?,
        )?;
        dict.set_item("full_location", &self.full_location)?;
        dict.set_item("country", &self.country)?;
        dict.set_item("country_iso_code", &self.country_iso_code)?;
        dict.set_item("postal_code", &self.postal_code)?;
        dict.set_item("member_distance", self.member_distance.as_str())?;
        dict.set_item("connected_since", self.connected_since)?;
        dict.set_item("follower_count", self.follower_count)?;
        dict.set_item("total_connections", self.total_connections)?;
        dict.set_item("is_following", self.is_following)?;
        dict.set_item("profile_urn", &self.profile_urn)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<FullProfile profile_urn=\"{}\" name=\"{} {}\">",
            self.profile_urn, self.first_name, self.last_name
        )
    }

    fn __str__(&self) -> String {
        let job = self.headline.as_deref().unwrap_or("No headline");
        format!(
            "{} {} - {} ({})",
            self.first_name, self.last_name, job, self.public_identifier
        )
    }
}

// Backwards-compatible alias retained at module registration time as `ProfileData`.

/// Network-specific metrics associated with a profile.
#[pyclass(dict)]
pub struct NetworkInfoData {
    #[pyo3(get)]
    pub following: Option<bool>,
    #[pyo3(get)]
    pub follower_count: Option<u32>,
    #[pyo3(get)]
    pub total_connections: Option<u32>,
}

impl From<CoreNetworkInfo> for NetworkInfoData {
    #[inline]
    fn from(n: CoreNetworkInfo) -> Self {
        Self {
            following: n.following,
            follower_count: n.follower_count,
            total_connections: n.total_connections,
        }
    }
}

#[pymethods]
impl NetworkInfoData {
    pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("following", self.following)?;
        dict.set_item("follower_count", self.follower_count)?;
        dict.set_item("total_connections", self.total_connections)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<NetworkInfoData following={:?} follower_count={:?} total_connections={:?}>",
            self.following, self.follower_count, self.total_connections
        )
    }
}

/// A single person's result returned during a search operation.
#[pyclass(dict)]
pub struct PeopleResult {
    #[pyo3(get)]
    pub full_name: String,
    #[pyo3(get)]
    pub distance: ProfileDistance,
    #[pyo3(get)]
    pub summary: Option<String>,
    #[pyo3(get)]
    pub primary_subtitle: Option<String>,
    #[pyo3(get)]
    pub secondary_subtitle: Option<String>,
    #[pyo3(get)]
    pub navigation_url: Option<String>,
    #[pyo3(get)]
    pub profile_id: Option<String>,
}

impl From<CorePeople> for PeopleResult {
    #[inline]
    fn from(p: CorePeople) -> Self {
        Self {
            full_name: p.full_name,
            distance: p.distance.into(),
            summary: p.summary,
            primary_subtitle: p.primary_subtitle,
            secondary_subtitle: p.secondary_subtitle,
            navigation_url: p.navigation_url,
            profile_id: p.profile_id.map(|id| id.to_string()),
        }
    }
}

#[pymethods]
impl PeopleResult {
    /// Backward-compatible alias for the historical `PeopleResult.profile_urn` field.
    #[getter]
    pub fn profile_urn(&self) -> Option<&str> {
        self.profile_id.as_deref()
    }

    pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("full_name", &self.full_name)?;
        dict.set_item("distance", self.distance.as_str())?;
        dict.set_item("summary", &self.summary)?;
        dict.set_item("primary_subtitle", &self.primary_subtitle)?;
        dict.set_item("secondary_subtitle", &self.secondary_subtitle)?;
        dict.set_item("navigation_url", &self.navigation_url)?;
        dict.set_item("profile_id", &self.profile_id)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<PeopleResult profile_id={:?} full_name=\"{}\">",
            self.profile_id, self.full_name
        )
    }

    fn __str__(&self) -> String {
        let job = self.primary_subtitle.as_deref().unwrap_or("No title");
        format!("{} ({})", self.full_name, job)
    }
}

/// Powerful builder to construct and execute LinkedIn search queries.
#[pyclass(get_all, set_all, from_py_object)]
#[derive(Clone, Default)]
pub struct SearchPeopleBuilder {
    pub keywords: Option<String>,
    pub limit: Option<usize>,
    pub offset: usize,
    pub connection_of: Option<String>,
    pub network_depths: Option<Vec<NetworkDepth>>,
    pub current_company: Option<Vec<String>>,
    pub past_companies: Option<Vec<String>>,
    pub nonprofit_interests: Option<Vec<String>>,
    pub profile_languages: Option<Vec<String>>,
    pub regions: Option<Vec<String>>,
    pub industries: Option<Vec<String>>,
    pub schools: Option<Vec<String>>,
    pub contact_interests: Option<Vec<String>>,
    pub service_categories: Option<Vec<String>>,
    pub include_private_profiles: bool,
    pub keyword_first_name: Option<String>,
    pub keyword_last_name: Option<String>,
    pub keyword_title: Option<String>,
    pub keyword_company: Option<String>,
    pub keyword_school: Option<String>,
    pub follower_of: Option<Vec<String>>,
    pub open_to_volunteer: Option<bool>,
}

#[pymethods]
impl SearchPeopleBuilder {
    /// Constructs a new search query by specifying desired filters.
    ///
    /// Args:
    ///     keywords (str, optional): The main search keyword. Defaults to None.
    ///     limit (int, optional): The maximum number of results to retrieve. Defaults to None (fetches all).
    ///     offset (int, optional): The starting offset for pagination. Defaults to 0.
    ///     connection_of (str, optional): Filters results to connections of this specific profile URN. Defaults to None.
    ///     current_company: LinkedIn internal company IDs as strings, copied from
    ///         currentCompany in a people-search URL, not company names.
    ///     network_depths (List[NetworkDepth], optional): Specifies the target network circles. Defaults to None.
    ///     include_private_profiles (bool, optional): Whether to keep "LinkedIn Member" private results. Defaults to False.
    ///     ... (other advanced demographic and professional filters).
    ///
    /// Returns:
    ///     SearchPeopleBuilder: The initialized builder instance.
    #[new]
    #[pyo3(signature = (
        keywords=None, limit=None, offset=0, connection_of=None, network_depths=None,
        current_company=None, past_companies=None, nonprofit_interests=None,
        profile_languages=None, regions=None, industries=None, schools=None,
        contact_interests=None, service_categories=None, include_private_profiles=false,
        keyword_first_name=None, keyword_last_name=None, keyword_title=None,
        keyword_company=None, keyword_school=None, follower_of=None, open_to_volunteer=None
    ))]
    #[allow(clippy::too_many_arguments)] // It's a builder, meant to have many parameters
    pub fn new(
        keywords: Option<String>,
        limit: Option<usize>,
        offset: usize,
        connection_of: Option<String>,
        network_depths: Option<Vec<NetworkDepth>>,
        current_company: Option<Vec<String>>,
        past_companies: Option<Vec<String>>,
        nonprofit_interests: Option<Vec<String>>,
        profile_languages: Option<Vec<String>>,
        regions: Option<Vec<String>>,
        industries: Option<Vec<String>>,
        schools: Option<Vec<String>>,
        contact_interests: Option<Vec<String>>,
        service_categories: Option<Vec<String>>,
        include_private_profiles: bool,
        keyword_first_name: Option<String>,
        keyword_last_name: Option<String>,
        keyword_title: Option<String>,
        keyword_company: Option<String>,
        keyword_school: Option<String>,
        follower_of: Option<Vec<String>>,
        open_to_volunteer: Option<bool>,
    ) -> Self {
        Self {
            keywords,
            limit,
            offset,
            connection_of,
            network_depths,
            current_company,
            past_companies,
            nonprofit_interests,
            profile_languages,
            regions,
            industries,
            schools,
            contact_interests,
            service_categories,
            include_private_profiles,
            keyword_first_name,
            keyword_last_name,
            keyword_title,
            keyword_company,
            keyword_school,
            follower_of,
            open_to_volunteer,
        }
    }

    /// Generates a SearchPeopleBuilder directly from a standard LinkedIn search URL.
    ///
    /// Args:
    ///     url (str): The copied URL from your LinkedIn web browser search.
    ///
    /// Returns:
    ///     SearchPeopleBuilder: A builder instance pre-filled with the URL's parameters.
    ///
    /// Raises:
    ///     UrlError: If the provided URL is malformed or unparseable.
    #[staticmethod]
    #[pyo3(signature = (url))]
    pub fn from_url(url: &str) -> PyLnResult<Self> {
        let core_builder = CoreSearchBuilder::from_url(url)?;
        let py_network_depths = core_builder.network_depths.map(|depths| {
            depths
                .into_iter()
                .filter_map(|d| match d {
                    CoreConnectionDegree::First => Some(NetworkDepth::FirstDegree),
                    CoreConnectionDegree::Second => Some(NetworkDepth::SecondDegree),
                    CoreConnectionDegree::Third => Some(NetworkDepth::ThirdDegree),
                    _ => None,
                })
                .collect::<Vec<NetworkDepth>>()
        });

        let to_py_opt = |v: Vec<String>| (!v.is_empty()).then_some(v);

        Ok(Self {
            keywords: core_builder.keywords,
            limit: core_builder.limit,
            offset: core_builder.offset,
            connection_of: core_builder.connection_of,
            network_depths: py_network_depths,
            current_company: to_py_opt(core_builder.current_company),
            past_companies: to_py_opt(core_builder.past_companies),
            nonprofit_interests: to_py_opt(core_builder.nonprofit_interests),
            profile_languages: to_py_opt(core_builder.profile_languages),
            regions: to_py_opt(core_builder.regions),
            industries: to_py_opt(core_builder.industries),
            schools: to_py_opt(core_builder.schools),
            contact_interests: to_py_opt(core_builder.contact_interests),
            service_categories: to_py_opt(core_builder.service_categories),
            include_private_profiles: core_builder.include_private_profiles,
            keyword_first_name: core_builder.keyword_first_name,
            keyword_last_name: core_builder.keyword_last_name,
            keyword_title: core_builder.keyword_title,
            keyword_company: core_builder.keyword_company,
            keyword_school: core_builder.keyword_school,
            follower_of: to_py_opt(core_builder.follower_of),
            open_to_volunteer: core_builder.open_to_volunteer,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "<SearchPeopleBuilder keywords={:?} limit={:?} offset={}>",
            self.keywords, self.limit, self.offset
        )
    }
}

impl SearchPeopleBuilder {
    /// Consumes `self` by value to build `CoreSearchBuilder` with zero heap clones.
    fn into_core(self) -> CoreSearchBuilder {
        let core_network_depths = self.network_depths.map(|depths| {
            depths
                .into_iter()
                .map(Into::into)
                .collect::<HashSet<CoreConnectionDegree>>()
        });

        CoreSearchBuilder {
            keywords: self.keywords,
            limit: self.limit,
            offset: self.offset,
            connection_of: self.connection_of,
            network_depths: core_network_depths,
            current_company: self.current_company.unwrap_or_default(),
            past_companies: self.past_companies.unwrap_or_default(),
            nonprofit_interests: self.nonprofit_interests.unwrap_or_default(),
            profile_languages: self.profile_languages.unwrap_or_default(),
            regions: self.regions.unwrap_or_default(),
            industries: self.industries.unwrap_or_default(),
            schools: self.schools.unwrap_or_default(),
            contact_interests: self.contact_interests.unwrap_or_default(),
            service_categories: self.service_categories.unwrap_or_default(),
            include_private_profiles: self.include_private_profiles,
            keyword_first_name: self.keyword_first_name,
            keyword_last_name: self.keyword_last_name,
            keyword_title: self.keyword_title,
            keyword_company: self.keyword_company,
            keyword_school: self.keyword_school,
            follower_of: self.follower_of.unwrap_or_default(),
            open_to_volunteer: self.open_to_volunteer,
        }
    }

    /// Converts a borrowed `SearchPeopleBuilder` into `CoreSearchBuilder` (1 clone instead of 2).
    fn to_core(&self) -> CoreSearchBuilder {
        self.clone().into_core()
    }
}

/// Python-friendly facade for the most common people-search filters.
/// `SearchPeopleBuilder` remains the advanced, 1:1 Rust-oriented API.
/// This facade is the recommended Python entry point for simple searches.
#[pyclass(from_py_object)]
#[derive(Clone, Default)]
pub struct SearchPeople {
    builder: SearchPeopleBuilder,
}

#[pymethods]
impl SearchPeople {
    #[new]
    #[pyo3(signature = (
        keywords=None, limit=None, offset=0, connection_of=None, network_depths=None,
        current_company=None, past_companies=None, nonprofit_interests=None,
        profile_languages=None, regions=None, industries=None, schools=None,
        contact_interests=None, service_categories=None, include_private_profiles=false,
        keyword_first_name=None, keyword_last_name=None, keyword_title=None,
        keyword_company=None, keyword_school=None, follower_of=None, open_to_volunteer=None
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        keywords: Option<String>,
        limit: Option<usize>,
        offset: usize,
        connection_of: Option<String>,
        network_depths: Option<Vec<NetworkDepth>>,
        current_company: Option<Vec<String>>,
        past_companies: Option<Vec<String>>,
        nonprofit_interests: Option<Vec<String>>,
        profile_languages: Option<Vec<String>>,
        regions: Option<Vec<String>>,
        industries: Option<Vec<String>>,
        schools: Option<Vec<String>>,
        contact_interests: Option<Vec<String>>,
        service_categories: Option<Vec<String>>,
        include_private_profiles: bool,
        keyword_first_name: Option<String>,
        keyword_last_name: Option<String>,
        keyword_title: Option<String>,
        keyword_company: Option<String>,
        keyword_school: Option<String>,
        follower_of: Option<Vec<String>>,
        open_to_volunteer: Option<bool>,
    ) -> Self {
        Self {
            builder: SearchPeopleBuilder::new(
                keywords,
                limit,
                offset,
                connection_of,
                network_depths,
                current_company,
                past_companies,
                nonprofit_interests,
                profile_languages,
                regions,
                industries,
                schools,
                contact_interests,
                service_categories,
                include_private_profiles,
                keyword_first_name,
                keyword_last_name,
                keyword_title,
                keyword_company,
                keyword_school,
                follower_of,
                open_to_volunteer,
            ),
        }
    }

    /// Imports a search directly from the browser URL.
    ///
    /// Raises:
    ///     UrlError: If the provided URL is malformed or unparseable.
    #[staticmethod]
    #[pyo3(signature = (url))]
    pub fn from_url(url: &str) -> PyLnResult<Self> {
        let builder = SearchPeopleBuilder::from_url(url)?;
        Ok(Self { builder })
    }

    pub fn keywords<'py>(mut slf: PyRefMut<'py, Self>, keywords: &str) -> PyRefMut<'py, Self> {
        slf.builder.keywords = Some(keywords.to_owned());
        slf
    }

    #[pyo3(signature = (limit=None))]
    pub fn limit<'py>(mut slf: PyRefMut<'py, Self>, limit: Option<usize>) -> PyRefMut<'py, Self> {
        slf.builder.limit = limit;
        slf
    }

    pub fn offset<'py>(mut slf: PyRefMut<'py, Self>, offset: usize) -> PyRefMut<'py, Self> {
        slf.builder.offset = offset;
        slf
    }

    pub fn connection_of<'py>(
        mut slf: PyRefMut<'py, Self>,
        connection_of: &str,
    ) -> PyRefMut<'py, Self> {
        slf.builder.connection_of = Some(connection_of.to_owned());
        slf
    }

    pub fn network<'py>(
        mut slf: PyRefMut<'py, Self>,
        network: Vec<NetworkDepth>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.network_depths = Some(network);
        slf
    }

    pub fn network_depths<'py>(
        mut slf: PyRefMut<'py, Self>,
        network_depths: Vec<NetworkDepth>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.network_depths = Some(network_depths);
        slf
    }

    /// Filters by LinkedIn internal company IDs (strings), not company names.
    /// Copy IDs from currentCompany in a people-search URL, or use from_url.
    pub fn current_company<'py>(
        mut slf: PyRefMut<'py, Self>,
        current_company: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.current_company = Some(current_company);
        slf
    }

    /// Alias of current_company; accepts LinkedIn internal company IDs.
    pub fn companies<'py>(
        mut slf: PyRefMut<'py, Self>,
        companies: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.current_company = Some(companies);
        slf
    }

    pub fn past_companies<'py>(
        mut slf: PyRefMut<'py, Self>,
        past_companies: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.past_companies = Some(past_companies);
        slf
    }

    pub fn schools<'py>(mut slf: PyRefMut<'py, Self>, schools: Vec<String>) -> PyRefMut<'py, Self> {
        slf.builder.schools = Some(schools);
        slf
    }

    pub fn regions<'py>(mut slf: PyRefMut<'py, Self>, regions: Vec<String>) -> PyRefMut<'py, Self> {
        slf.builder.regions = Some(regions);
        slf
    }

    pub fn industries<'py>(
        mut slf: PyRefMut<'py, Self>,
        industries: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.industries = Some(industries);
        slf
    }

    pub fn profile_languages<'py>(
        mut slf: PyRefMut<'py, Self>,
        profile_languages: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.profile_languages = Some(profile_languages);
        slf
    }

    pub fn service_categories<'py>(
        mut slf: PyRefMut<'py, Self>,
        service_categories: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.service_categories = Some(service_categories);
        slf
    }

    pub fn contact_interests<'py>(
        mut slf: PyRefMut<'py, Self>,
        contact_interests: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.contact_interests = Some(contact_interests);
        slf
    }

    pub fn nonprofit_interests<'py>(
        mut slf: PyRefMut<'py, Self>,
        nonprofit_interests: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.nonprofit_interests = Some(nonprofit_interests);
        slf
    }

    pub fn follower_of<'py>(
        mut slf: PyRefMut<'py, Self>,
        follower_of: Vec<String>,
    ) -> PyRefMut<'py, Self> {
        slf.builder.follower_of = Some(follower_of);
        slf
    }

    pub fn keyword_first_name<'py>(
        mut slf: PyRefMut<'py, Self>,
        first_name: &str,
    ) -> PyRefMut<'py, Self> {
        slf.builder.keyword_first_name = Some(first_name.to_owned());
        slf
    }

    pub fn first_name<'py>(mut slf: PyRefMut<'py, Self>, first_name: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_first_name = Some(first_name.to_owned());
        slf
    }

    pub fn keyword_last_name<'py>(
        mut slf: PyRefMut<'py, Self>,
        last_name: &str,
    ) -> PyRefMut<'py, Self> {
        slf.builder.keyword_last_name = Some(last_name.to_owned());
        slf
    }

    pub fn last_name<'py>(mut slf: PyRefMut<'py, Self>, last_name: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_last_name = Some(last_name.to_owned());
        slf
    }

    pub fn keyword_title<'py>(mut slf: PyRefMut<'py, Self>, title: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_title = Some(title.to_owned());
        slf
    }

    pub fn title<'py>(mut slf: PyRefMut<'py, Self>, title: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_title = Some(title.to_owned());
        slf
    }

    pub fn keyword_company<'py>(
        mut slf: PyRefMut<'py, Self>,
        company: &str,
    ) -> PyRefMut<'py, Self> {
        slf.builder.keyword_company = Some(company.to_owned());
        slf
    }

    pub fn company<'py>(mut slf: PyRefMut<'py, Self>, company: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_company = Some(company.to_owned());
        slf
    }

    pub fn keyword_school<'py>(mut slf: PyRefMut<'py, Self>, school: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_school = Some(school.to_owned());
        slf
    }

    pub fn school<'py>(mut slf: PyRefMut<'py, Self>, school: &str) -> PyRefMut<'py, Self> {
        slf.builder.keyword_school = Some(school.to_owned());
        slf
    }

    #[pyo3(signature = (include=true))]
    pub fn include_private_profiles<'py>(
        mut slf: PyRefMut<'py, Self>,
        include: bool,
    ) -> PyRefMut<'py, Self> {
        slf.builder.include_private_profiles = include;
        slf
    }

    #[pyo3(signature = (open=true))]
    pub fn open_to_volunteer<'py>(mut slf: PyRefMut<'py, Self>, open: bool) -> PyRefMut<'py, Self> {
        slf.builder.open_to_volunteer = Some(open);
        slf
    }

    #[getter]
    pub fn builder(&self) -> SearchPeopleBuilder {
        self.builder.clone()
    }

    pub fn build(&self) -> SearchPeopleBuilder {
        self.builder.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "<SearchPeople keywords={:?} limit={:?} offset={}>",
            self.builder.keywords, self.builder.limit, self.builder.offset,
        )
    }
}

#[inline]
fn extract_kw_opt<'py, T>(kw: &Bound<'py, PyDict>, key: &str) -> PyResult<Option<T>>
where
    T: for<'a> FromPyObject<'a, 'py>,
{
    kw.get_item(key)?
        .map(|v| v.extract::<Option<T>>().map_err(Into::into))
        .transpose()
        .map(Option::flatten)
}

#[inline]
fn extract_kw_or<'py, T>(kw: &Bound<'py, PyDict>, key: &str, default: T) -> PyResult<T>
where
    T: for<'a> FromPyObject<'a, 'py>,
{
    Ok(extract_kw_opt(kw, key)?.unwrap_or(default))
}

/// The main client to interact with the LinkedIn API.
///
/// This class maintains the active session, handles cookie extraction, bot evasion techniques,
/// and securely centralizes all network calls.
///
/// Example:
///     ```python
///     from leankedin_park import Linkedin, ClientConfig, Browser
///
///     config = ClientConfig(Browser.Firefox)
///     client = Linkedin.with_config(config)
///     ```
#[pyclass]
pub struct Linkedin {
    // `None` means that the context manager has deterministically closed the client.
    inner: Option<CoreLinkedin>,
}

#[pymethods]
impl Linkedin {
    /// Initializes a new LinkedIn client for the specified browser with default settings.
    ///
    /// Args:
    ///     browser (Browser): The browser to extract session cookies from.
    ///
    /// Returns:
    ///     Linkedin: A new client instance ready for network requests.
    ///
    /// Raises:
    ///     CookieError: If essential cookies are missing or expired.
    ///     ClientConfigError: If configuration is invalid.
    #[staticmethod]
    #[pyo3(signature = (browser))]
    pub fn new(browser: Browser) -> PyLnResult<Self> {
        LazyLock::force(&RUNTIME);
        Ok(Self {
            inner: Some(CoreLinkedin::new(browser.into())?),
        })
    }

    /// Retrieves the authenticated user's URN ID, fetching it lazily from the API if not already cached.
    ///
    /// Returns:
    ///     str: The raw profile ID (e.g. "ACoAAA...").
    ///
    /// Raises:
    ///     LinkedInAPIError: In case of a network or API issue.
    #[pyo3(signature = ())]
    pub fn get_or_fetch_urn(&self, py: Python<'_>) -> PyLnResult<String> {
        let client = self.client()?;
        Ok(py
            .detach(|| RUNTIME.block_on(client.get_or_fetch_urn()))?
            .as_str()
            .to_owned())
    }

    /// Initializes a new LinkedIn client with a customized [`ClientConfig`].
    ///
    /// Args:
    ///     config (ClientConfig): The custom client configuration.
    ///
    /// Returns:
    ///     Linkedin: A configured client instance.
    ///
    /// Raises:
    ///     LinkedInAPIError: In case of a configuration or session error.
    #[staticmethod]
    #[pyo3(signature = (config))]
    pub fn with_config(config: &ClientConfig) -> PyLnResult<Self> {
        LazyLock::force(&RUNTIME);

        let inner = CoreLinkedin::with_config(config.inner.clone())?;
        Ok(Self { inner: Some(inner) })
    }

    /// Retrieves identity information for the currently authenticated profile.
    ///
    /// Returns:
    ///     LinkedInProfileData: The profile object containing first name, last name, vanity name, and URN.
    ///
    /// Raises:
    ///     LinkedInAPIError: If the profile could not be fetched from the API.
    #[pyo3(signature = ())]
    pub fn get_my_profile(&self, py: Python<'_>) -> PyLnResult<LinkedInProfileData> {
        let client = self.client()?;
        let profile = py.detach(|| RUNTIME.block_on(client.account().get_my_profile()))?;
        Ok(profile.into())
    }

    /// Extracts statistics regarding your profile views (90-day history).
    ///
    /// Returns:
    ///     ProfileViews: View count and the exact time frame assessed.
    ///
    /// Raises:
    ///     LinkedInAPIError: If the profile view data cannot be retrieved.
    #[pyo3(signature = ())]
    pub fn get_current_profile_views_last_90_days(
        &self,
        py: Python<'_>,
    ) -> PyLnResult<ProfileViews> {
        let client = self.client()?;
        let views = py.detach(|| {
            RUNTIME.block_on(client.account().get_current_profile_views_last_90_days())
        })?;
        Ok(views.into())
    }

    /// Retrieves the list of pending network connection invitations.
    ///
    /// Args:
    ///     offset (int): Pagination offset (e.g., 0).
    ///     limit (int): Maximum number of invitations to fetch.
    ///
    /// Returns:
    ///     List[InvitationData]: The list of received invitations.
    ///
    /// Raises:
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (offset, limit))]
    pub fn get_invitations(
        &self,
        py: Python<'_>,
        offset: u32,
        limit: u32,
    ) -> PyLnResult<Vec<InvitationData>> {
        let client = self.client()?;
        let invs =
            py.detach(|| RUNTIME.block_on(client.connections().get_invitations(offset, limit)))?;
        Ok(invs.into_iter().map(Into::into).collect())
    }

    /// Accepts or rejects a connection invitation.
    ///
    /// Args:
    ///     urn (str): The entity URN of the invitation.
    ///     secret (str): The shared secret associated with the invitation.
    ///     action (InvitationAction): Action to perform (Accept or Reject).
    ///
    /// Raises:
    ///     UrnError: If the invitation URN is incorrectly formatted.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (urn, secret, action))]
    pub fn reply_invitation(
        &self,
        py: Python<'_>,
        urn: &str,
        secret: &str,
        action: InvitationAction,
    ) -> PyLnResult<()> {
        let client = self.client()?;
        let invitation_id = InvitationId::try_from(urn)?;
        py.detach(|| {
            RUNTIME.block_on(client.connections().reply_invitation(
                &invitation_id,
                secret,
                action.into(),
            ))
        })?;
        Ok(())
    }

    /// Sends a connection request to a third party with an optional custom message.
    ///
    /// Args:
    ///     vanity_name (str): The target user's public identifier.
    ///     profile_urn (str): The internal URN of the target user.
    ///     message (str): The personal message to attach (limit 300 characters for premium, 200 for free).
    ///
    /// Raises:
    ///     VanityNameError: If the vanity name format is invalid.
    ///     UrnError: If the profile URN format is invalid.
    ///     MessagingError: If the attached custom message exceeds the character limit.
    ///     LinkedInAPIError: In case of an API error.
    #[pyo3(signature = (vanity_name, profile_urn=None, message=None))]
    pub fn send_connection_request(
        &self,
        py: Python<'_>,
        vanity_name: &str,
        profile_urn: Option<&str>,
        message: Option<&str>,
    ) -> PyLnResult<()> {
        let client = self.client()?;
        let safe_v_name = vanity_name.parse::<VanityName>()?;
        let safe_urn = profile_urn.map(str::parse::<ProfileId>).transpose()?;

        let req = match (safe_urn, message.filter(|m| !m.is_empty())) {
            (Some(urn), Some(custom_message)) => ConnectionRequest::builder()
                .target_vanity_name(safe_v_name)
                .profile_urn(urn)
                .custom_message(custom_message.to_owned())
                .build(),
            (Some(urn), None) => ConnectionRequest::builder()
                .target_vanity_name(safe_v_name)
                .profile_urn(urn)
                .build(),
            (None, Some(custom_message)) => ConnectionRequest::builder()
                .target_vanity_name(safe_v_name)
                .custom_message(custom_message.to_owned())
                .build(),
            (None, None) => ConnectionRequest::builder()
                .target_vanity_name(safe_v_name)
                .build(),
        };

        py.detach(|| RUNTIME.block_on(client.connections().send_connection_request(&req)))?;
        Ok(())
    }

    /// Removes an existing person from your network connections using their Vanity Name.
    ///
    /// Args:
    ///     vanity_name (str): The public identifier of the person to disconnect from.
    ///
    /// Raises:
    ///     VanityNameError: If the vanity name format is invalid.
    ///     LinkedInAPIError: In case of an API or network error.
    #[pyo3(signature = (vanity_name))]
    pub fn remove_connection(&self, py: Python<'_>, vanity_name: &str) -> PyLnResult<()> {
        let client = self.client()?;
        let target = vanity_name.as_vanity_str()?;
        py.detach(|| RUNTIME.block_on(client.connections().remove_connection(target)))?;
        Ok(())
    }

    /// Retrieves the complete list of conversations from the inbox with participant details.
    ///
    /// Returns:
    ///     List[ConversationData]: A list detailing your messages and group chats.
    ///
    /// Raises:
    ///     LinkedInAPIError: In case of an API or network error.
    #[pyo3(signature = ())]
    pub fn get_conversations_list(&self, py: Python<'_>) -> PyLnResult<Vec<ConversationData>> {
        let client = self.client()?;
        let convs = py.detach(|| RUNTIME.block_on(client.messaging().get_conversations_list()))?;
        Ok(convs.into_iter().map(Into::into).collect())
    }

    /// Sends a direct message to a specific person.
    ///
    /// Args:
    ///     message (str): The body content of the message.
    ///     dest_urn (str): The internal URN of the destination profile.
    ///
    /// Raises:
    ///     UrnError: If a profile URN is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (message, dest_urn))]
    pub fn send_message(&self, py: Python<'_>, message: &str, dest_urn: &str) -> PyLnResult<()> {
        let client = self.client()?;
        let urn = dest_urn.parse::<ProfileId>()?;
        py.detach(|| RUNTIME.block_on(client.messaging().send_message(message, &urn)))?;
        Ok(())
    }

    /// Sends a group message to a defined list of recipients.
    ///
    /// Args:
    ///     message (str): The body content of the message.
    ///     dest_urns (List[str]): A list containing the target URNs.
    ///
    /// Raises:
    ///     UrnError: If a profile URN is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (message, dest_urns))]
    pub fn send_group_message<'py>(
        &self,
        py: Python<'py>,
        message: &str,
        dest_urns: Vec<Bound<'py, PyString>>,
    ) -> PyResult<()> {
        let client = self.client()?;
        let urns: Vec<ProfileId> = dest_urns
            .iter()
            .map(|py_str| {
                let s = py_str.to_str()?;
                s.parse::<ProfileId>()
                    .map_err(|e| PyErr::from(PyLinkedInError::from(e)))
            })
            .collect::<Result<Vec<_>, PyErr>>()?;

        py.detach(|| RUNTIME.block_on(client.messaging().send_group_message(message, &urns)))
            .map_err(PyLinkedInError::from)?;
        Ok(())
    }

    /// Retrieves basic public identity data of a profile.
    ///
    /// Args:
    ///     vanity_name (str): The public identifier of the target profile.
    ///
    /// Returns:
    ///     BaseProfileData: Base information regarding the target profile.
    ///
    /// Raises:
    ///     VanityNameError: If the vanity name format is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (vanity_name))]
    pub fn get_infos_of_a_profile(
        &self,
        py: Python<'_>,
        vanity_name: &str,
    ) -> PyLnResult<BaseProfileData> {
        let client = self.client()?;
        let target = vanity_name.as_vanity_str()?;
        let base_p =
            py.detach(|| RUNTIME.block_on(client.people().get_infos_of_a_profile(target)))?;
        Ok(base_p.into())
    }

    /// Retrieves network metric data associated with the profile (followers, network size).
    ///
    /// Args:
    ///     vanity_name (str): The public identifier of the target profile.
    ///
    /// Returns:
    ///     NetworkInfoData: Information regarding the profile's network statistics.
    ///
    /// Raises:
    ///     VanityNameError: If the vanity name format is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (vanity_name))]
    pub fn get_network_infos_of_a_profile(
        &self,
        py: Python<'_>,
        vanity_name: &str,
    ) -> PyLnResult<NetworkInfoData> {
        let client = self.client()?;
        let target = vanity_name.as_vanity_str()?;
        let net_info =
            py.detach(|| RUNTIME.block_on(client.people().get_network_infos_of_a_profile(target)))?;
        Ok(net_info.into())
    }

    /// All-in-one method to extract the entire profile and its network in a single call.
    ///
    /// Args:
    ///     vanity_name (str): The public identifier of the target profile.
    ///
    /// Returns:
    ///     FullProfileData: Complete information regarding the profile and its network.
    ///
    /// Raises:
    ///     VanityNameError: If the vanity name format is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (vanity_name))]
    pub fn get_full_infos_of_a_profile(
        &self,
        py: Python<'_>,
        vanity_name: &str,
    ) -> PyLnResult<FullProfileData> {
        let client = self.client()?;
        let target = vanity_name.as_vanity_str()?;
        let full_p =
            py.detach(|| RUNTIME.block_on(client.people().get_full_infos_of_a_profile(target)))?;
        Ok(full_p.into())
    }

    /// Subscribes to the publications and feed of a specific person.
    ///
    /// Args:
    ///     urn (str): The target profile's internal URN.
    ///
    /// Raises:
    ///     UrnError: If a profile URN is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (urn))]
    pub fn follow_people(&self, py: Python<'_>, urn: &str) -> PyLnResult<()> {
        let client = self.client()?;
        let parsed_urn = urn.parse::<ProfileId>()?;
        py.detach(|| RUNTIME.block_on(client.people().follow_people(&parsed_urn)))?;
        Ok(())
    }

    /// Unsubscribes from a person's feed (without necessarily removing them as a connection).
    ///
    /// Args:
    ///     urn (str): The target profile's internal URN.
    ///
    /// Raises:
    ///     UrnError: If a profile URN is invalid.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (urn))]
    pub fn unfollow_people(&self, py: Python<'_>, urn: &str) -> PyLnResult<()> {
        let client = self.client()?;
        let parsed_urn = urn.parse::<ProfileId>()?;
        py.detach(|| RUNTIME.block_on(client.people().unfollow_people(&parsed_urn)))?;
        Ok(())
    }

    /// Publishes a new LinkedIn post, configuring visibility and who is allowed to reply.
    ///
    /// Args:
    ///     text (str): The text content of the post.
    ///     visibility (VisibilityType): Who can view the post (e.g., Anyone).
    ///     comments_scope (AllowedCommentersScope): Who can comment on the post (e.g., ConnectionsOnly).
    ///
    /// Raises:
    ///     PublishPostError: If the content is empty or exceeds the character limit.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (text, visibility=None, comments_scope=None))]
    pub fn publish_post(
        &self,
        py: Python<'_>,
        text: &str,
        visibility: Option<VisibilityType>,
        comments_scope: Option<AllowedCommentersScope>,
    ) -> PyLnResult<()> {
        let client = self.client()?;
        let visibility = visibility.unwrap_or(VisibilityType::Anyone);
        let comments_scope = comments_scope.unwrap_or(AllowedCommentersScope::Anyone);
        let post = NewPost::builder()
            .text(text.to_owned())
            .visibility(visibility.into())
            .comments_scope(comments_scope.into())
            .build();
        py.detach(|| RUNTIME.block_on(client.posts().publish_post(&post)))?;
        Ok(())
    }

    /// Adds a reaction (Like, Praise, Support...) to an existing thread or post.
    ///
    /// Args:
    ///     reaction (ReactionType): The exact reaction type to apply.
    ///     post_urn (str): The activity URN of the targeted post.
    ///
    /// Raises:
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (reaction, post_urn))]
    pub fn react_to_post(
        &self,
        py: Python<'_>,
        reaction: ReactionType,
        post_urn: &str,
    ) -> PyLnResult<()> {
        let client = self.client()?;
        py.detach(|| RUNTIME.block_on(client.posts().react_to_post(reaction.into(), post_urn)))?;
        Ok(())
    }

    /// Executes a profile search with strictly validated filters and responses.
    ///
    /// This method accepts either a `SearchPeopleBuilder` object or directly
    /// keyword arguments (`kwargs`) for maximum flexibility.
    ///
    /// Args:
    ///     *args: Expects `builder` (SearchPeopleBuilder) in position 0.
    ///     current_company: LinkedIn internal company IDs as strings, copied from
    ///         currentCompany in a people-search URL, not company names.
    ///     **kwargs: Direct search keyword arguments (e.g., keywords="Rust", limit=10).
    ///
    /// Returns:
    ///     List[PeopleResult]: A list of profiles matching the search criteria.
    ///
    /// Raises:
    ///     TypeError: For unknown keywords, extra positional arguments, or mixed call styles.
    ///     SearchError: If the response is invalid or retrieval cannot complete.
    ///     LinkedInAPIError: In case of a network or API problem.
    #[pyo3(signature = (*args, **kwargs))]
    pub fn search_people(
        &self,
        py: Python<'_>,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Vec<PeopleResult>> {
        if args.len() > 1 {
            return Err(PyTypeError::new_err(
                "search_people() accepts at most one positional builder",
            ));
        }
        if !args.is_empty() && kwargs.is_some_and(|kw| !kw.is_empty()) {
            return Err(PyTypeError::new_err(
                "Pass a SearchPeople/SearchPeopleBuilder or keyword filters, not both.",
            ));
        }

        const KEYWORDS: &[&str] = &[
            "keywords",
            "limit",
            "offset",
            "connection_of",
            "network_depths",
            "current_company",
            "past_companies",
            "nonprofit_interests",
            "profile_languages",
            "regions",
            "industries",
            "schools",
            "contact_interests",
            "service_categories",
            "include_private_profiles",
            "keyword_first_name",
            "keyword_last_name",
            "keyword_title",
            "keyword_company",
            "keyword_school",
            "follower_of",
            "open_to_volunteer",
        ];

        if let Some(kw) = kwargs {
            for key in kw.keys().iter() {
                let py_str = key.cast::<PyString>().map_err(|_| {
                    PyTypeError::new_err("search_people() keywords must be strings")
                })?;
                let name = py_str.to_str()?;
                if !KEYWORDS.contains(&name) {
                    return Err(PyTypeError::new_err(format!(
                        "search_people() got an unexpected keyword argument '{name}'"
                    )));
                }
            }
        }

        let core_builder = if let Some(kw) = kwargs.filter(|kw| !kw.is_empty()) {
            SearchPeopleBuilder {
                keywords: extract_kw_opt(kw, "keywords")?,
                limit: extract_kw_opt(kw, "limit")?,
                offset: extract_kw_or(kw, "offset", 0)?,
                connection_of: extract_kw_opt(kw, "connection_of")?,
                network_depths: extract_kw_opt(kw, "network_depths")?,
                current_company: extract_kw_opt(kw, "current_company")?,
                past_companies: extract_kw_opt(kw, "past_companies")?,
                nonprofit_interests: extract_kw_opt(kw, "nonprofit_interests")?,
                profile_languages: extract_kw_opt(kw, "profile_languages")?,
                regions: extract_kw_opt(kw, "regions")?,
                industries: extract_kw_opt(kw, "industries")?,
                schools: extract_kw_opt(kw, "schools")?,
                contact_interests: extract_kw_opt(kw, "contact_interests")?,
                service_categories: extract_kw_opt(kw, "service_categories")?,
                include_private_profiles: extract_kw_or(kw, "include_private_profiles", false)?,
                keyword_first_name: extract_kw_opt(kw, "keyword_first_name")?,
                keyword_last_name: extract_kw_opt(kw, "keyword_last_name")?,
                keyword_title: extract_kw_opt(kw, "keyword_title")?,
                keyword_company: extract_kw_opt(kw, "keyword_company")?,
                keyword_school: extract_kw_opt(kw, "keyword_school")?,
                follower_of: extract_kw_opt(kw, "follower_of")?,
                open_to_volunteer: extract_kw_opt(kw, "open_to_volunteer")?,
            }
            .into_core()
        } else if !args.is_empty() {
            let item = args.get_item(0)?;
            if let Ok(search) = item.extract::<PyRef<'_, SearchPeople>>() {
                search.builder.to_core()
            } else if let Ok(builder) = item.extract::<PyRef<'_, SearchPeopleBuilder>>() {
                builder.to_core()
            } else {
                return Err(PyTypeError::new_err(
                    "The first argument must be a SearchPeople or SearchPeopleBuilder instance.",
                ));
            }
        } else {
            CoreSearchBuilder::default()
        };

        let client = self.client()?;
        let results = py
            .detach(|| RUNTIME.block_on(client.search().search_people(&core_builder)))
            .map_err(PyLinkedInError::from)?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Enters the context manager and returns the active client.
    ///
    /// Raises:
    ///     RuntimeError: If the client is already closed.
    fn __enter__(slf: PyRef<'_, Self>) -> PyResult<PyRef<'_, Self>> {
        if slf.inner.is_none() {
            return Err(PyRuntimeError::new_err(
                "The LinkedIn client is already closed.",
            ));
        }

        Ok(slf)
    }

    /// Exits the context manager and deterministically releases the Rust client.
    /// Returning False preserves any exception raised inside the `with` block.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        if self.inner.take().is_some() {
            log::debug!("LinkedIn client session closed by context manager.");
        }
        Ok(false)
    }

    fn __repr__(&self) -> String {
        if self.inner.is_some() {
            "<Linkedin (Active Session)>".to_string()
        } else {
            "<Linkedin (Closed Session)>".to_string()
        }
    }
}

impl Linkedin {
    fn client(&self) -> PyLnResult<&CoreLinkedin> {
        self.inner.as_ref().ok_or_else(|| {
            PyLinkedInError(core_errors::LinkedInError::Api(
                "The LinkedIn client is closed. Create a new client or keep it inside the `with` block."
                    .to_string(),
            ))
        })
    }
}

/// A high-performance Python client for the LinkedIn Voyager API powered by Rust.
#[pymodule]
fn leankedin_park(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();

    m.add("LinkedInAPIError", py.get_type::<LinkedInAPIError>())?;
    m.add("CookieError", py.get_type::<CookieError>())?;
    m.add("ClientConfigError", py.get_type::<ClientConfigError>())?;
    m.add("MessagingError", py.get_type::<MessagingError>())?;
    m.add("UrnError", py.get_type::<UrnError>())?;
    m.add("VanityNameError", py.get_type::<VanityNameError>())?;
    m.add("UrlError", py.get_type::<UrlError>())?;
    m.add("PublishPostError", py.get_type::<PublishPostError>())?;
    m.add("HttpApiError", py.get_type::<HttpApiError>())?;
    m.add("SearchError", py.get_type::<SearchError>())?;

    m.add_class::<Browser>()?;
    m.add_class::<ClientConfig>()?;

    m.add_class::<InvitationAction>()?;
    m.add_class::<VisibilityType>()?;
    m.add_class::<AllowedCommentersScope>()?;
    m.add_class::<ReactionType>()?;
    m.add_class::<NetworkDepth>()?;
    m.add_class::<ProfileDistance>()?;
    m.add_class::<ConversationCategory>()?;
    m.add("ConnectionDegree", py.get_type::<ProfileDistance>())?;

    m.add_class::<ProfileViews>()?;
    m.add_class::<ConversationParticipantData>()?;
    m.add_class::<ConversationData>()?;
    m.add_class::<InvitationData>()?;
    m.add_class::<DateData>()?;
    m.add_class::<LinkedInProfileData>()?;
    m.add_class::<BaseProfileData>()?;
    m.add_class::<FullProfileData>()?;
    m.add_class::<NetworkInfoData>()?;

    m.add_class::<SearchPeopleBuilder>()?;
    m.add_class::<SearchPeople>()?;
    m.add_class::<PeopleResult>()?;

    // Backwards-compatible aliases for the previous Python surface.
    m.add("ProfileData", py.get_type::<FullProfileData>())?;
    m.add("People", py.get_type::<PeopleResult>())?;
    m.add("ReceivedInvitation", py.get_type::<InvitationData>())?;
    m.add("NetworkInfo", py.get_type::<NetworkInfoData>())?;

    m.add_class::<Linkedin>()?;

    Ok(())
}

#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn full_profile_dict_round_trips_through_python_json() {
        Python::initialize();
        Python::attach(|py| {
            let json = py.import("json").unwrap();
            // Result objects have no Python constructor; build fixtures in Rust
            // and exercise the same conversion used by the public Python method.
            let mut profile = FullProfileData {
                first_name: "Example".into(),
                last_name: "Person".into(),
                public_identifier: "example-person".into(),
                headline: None,
                pronoun: None,
                is_creator: None,
                is_premium: None,
                is_influencer: None,
                birth_date_on: None,
                full_location: None,
                country: None,
                country_iso_code: None,
                postal_code: None,
                member_distance: ProfileDistance::Unknown,
                connected_since: None,
                follower_count: None,
                total_connections: None,
                is_following: None,
                profile_urn: "example".into(),
            };
            for (date, expected) in [
                (None, serde_json::Value::Null),
                (
                    Some(DateData {
                        year: Some(2000),
                        month: Some(1),
                        day: Some(2),
                    }),
                    serde_json::json!({"year": 2000, "month": 1, "day": 2}),
                ),
                (
                    Some(DateData {
                        year: None,
                        month: Some(1),
                        day: Some(2),
                    }),
                    serde_json::json!({"year": null, "month": 1, "day": 2}),
                ),
            ] {
                profile.birth_date_on = date;
                let dictionary = profile.to_dict(py).unwrap();
                let encoded: String = json
                    .call_method1("dumps", (dictionary,))
                    .unwrap()
                    .extract()
                    .unwrap();
                let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap();
                assert_eq!(decoded["birth_date_on"], expected);
                assert!(decoded["member_distance"].is_string());
                if let Some(date) = &profile.birth_date_on {
                    assert_eq!(date.month, Some(1));
                }
            }
        });
    }
}

#[cfg(test)]
mod search_error_tests {
    use super::*;

    #[test]
    fn every_core_search_failure_becomes_the_python_search_exception() {
        Python::initialize();
        Python::attach(|py| {
            let invalid_json = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
            for error in [
                core_errors::SearchError::InvalidResponse(invalid_json),
                core_errors::SearchError::UnexpectedResponse("missing collection"),
                core_errors::SearchError::InvalidResult {
                    index: 1,
                    source: Box::new(core_errors::UrnError::InvalidFormat("bad".into()).into()),
                },
                core_errors::SearchError::Incomplete {
                    collected: 10,
                    reason: "request cap",
                },
            ] {
                let expected = error.to_string();
                let error: PyErr =
                    PyLinkedInError::from(core_errors::LinkedInError::from(error)).into();
                assert!(error.is_instance_of::<SearchError>(py));
                assert!(error.is_instance_of::<LinkedInAPIError>(py));
                assert_eq!(error.value(py).str().unwrap().to_str().unwrap(), expected);
            }
        });
    }
}
