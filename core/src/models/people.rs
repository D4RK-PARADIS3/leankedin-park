//! Domain models for public LinkedIn profiles, network information, and connection degrees.

use crate::models::identifiers::{ProfileId, VanityName};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

/// Comprehensive profile data combining identity, location, and network metrics.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FullProfile {
    /// First / given name.
    pub first_name: String,
    /// Last / family name.
    pub last_name: String,
    /// Public vanity identifier (e.g., `"williamhgates"`).
    pub public_identifier: VanityName,
    /// Professional headline.
    pub headline: Option<String>,
    /// Pronouns if configured on profile.
    pub pronoun: Option<String>,
    /// Whether creator mode is enabled.
    pub is_creator: Option<bool>,
    /// Whether the user has a LinkedIn Premium subscription.
    pub is_premium: Option<bool>,
    /// Whether the user is recognized as a LinkedIn Influencer.
    pub is_influencer: Option<bool>,
    /// Birth date if publicly visible.
    pub birth_date_on: Option<Date>,
    /// Formatted full location string.
    pub full_location: Option<String>,
    /// Country name.
    pub country: Option<String>,
    /// Country ISO code (e.g., `"US"`).
    pub country_iso_code: Option<String>,
    /// Postal code if available.
    pub postal_code: Option<String>,
    /// Network connection distance relative to the authenticated user.
    pub member_distance: ConnectionDegree,
    /// Epoch timestamp when the connection was established (if 1st-degree).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_since: Option<i64>,
    /// Number of followers.
    pub follower_count: Option<u32>,
    /// Number of connections.
    pub total_connections: Option<u32>,
    /// Whether the authenticated user follows this member.
    pub is_following: Option<bool>,
    /// Unique profile identifier / URN.
    pub profile_urn: ProfileId,
}

impl FullProfile {
    /// Constructs a [`FullProfile`] by merging base identity data and network statistics.
    pub fn new(identity: BaseProfile, network: Option<NetworkInfo>) -> Self {
        let BaseProfile {
            first_name,
            last_name,
            public_identifier,
            profile_urn,
            headline,
            pronoun,
            creator,
            premium,
            influencer,
            birth_date_on,
            full_location,
            country,
            country_iso_code,
            postal_code,
            member_distance,
            connected_since,
        } = identity;

        let (follower_count, is_following, total_connections) = match network {
            Some(n) => (n.follower_count, n.following, n.total_connections),
            None => (None, None, None),
        };

        Self {
            first_name,
            last_name,
            public_identifier,
            profile_urn,
            headline,
            pronoun,
            is_creator: creator,
            is_premium: premium,
            is_influencer: influencer,
            birth_date_on,
            full_location,
            country,
            country_iso_code,
            postal_code,
            member_distance,
            connected_since,
            follower_count,
            total_connections,
            is_following,
        }
    }
}

/// Network and engagement metrics associated with a LinkedIn member.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    /// Whether the authenticated user is currently following this profile.
    pub following: Option<bool>,
    /// Total number of followers.
    pub follower_count: Option<u32>,
    /// Total number of 1st-degree connections.
    pub total_connections: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RawConnectionDegree {
    #[serde(alias = "SELF", rename(serialize = "Self"))]
    Myself,
    #[serde(alias = "DISTANCE_1", rename(serialize = "First-degree"))]
    First,
    #[serde(alias = "DISTANCE_2", rename(serialize = "Second-degree"))]
    Second,
    #[serde(alias = "DISTANCE_3", rename(serialize = "Third-degree"))]
    Third,
    #[serde(alias = "OUT_OF_NETWORK", rename(serialize = "Out of network"))]
    OutOfNetwork,
    // Absorbs any new distance that LinkedIn might invent without crashing
    #[serde(other, rename(serialize = "Unknown"))]
    Unknown,
}

/// Relationship distance level between two LinkedIn profiles.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RawConnectionDegree")]
pub enum ConnectionDegree {
    /// The authenticated user themselves.
    Myself,
    /// 1st-degree direct connection.
    First,
    /// 2nd-degree connection (connection of a connection).
    Second,
    /// 3rd-degree connection.
    Third,
    /// Member is outside the authenticated user's 3-degree network.
    OutOfNetwork,
    /// Unrecognized distance code.
    Unknown,
}

impl From<RawConnectionDegree> for ConnectionDegree {
    fn from(raw: RawConnectionDegree) -> Self {
        match raw {
            RawConnectionDegree::Myself => Self::Myself,
            RawConnectionDegree::First => Self::First,
            RawConnectionDegree::Second => Self::Second,
            RawConnectionDegree::Third => Self::Third,
            RawConnectionDegree::OutOfNetwork => Self::OutOfNetwork,
            RawConnectionDegree::Unknown => Self::Unknown,
        }
    }
}

impl ConnectionDegree {
    /// Converts the degree into the character code expected by the LinkedIn search engine (`"F"`, `"S"`, `"O"`).
    pub fn search_code(self) -> Option<&'static str> {
        match self {
            Self::First => Some("F"),
            Self::Second => Some("S"),
            Self::Third => Some("O"),
            _ => None,
        }
    }

    /// Builds the degree from a search URL parameter character code.
    pub fn from_search_code(code: &str) -> Option<Self> {
        match code {
            "F" => Some(Self::First),
            "S" => Some(Self::Second),
            "O" => Some(Self::Third),
            _ => None,
        }
    }
}

impl fmt::Display for ConnectionDegree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Myself => "Self",
            Self::First => "First-degree",
            Self::Second => "Second-degree",
            Self::Third => "Third-degree",
            Self::OutOfNetwork => "Out of network",
            Self::Unknown => "Unknown",
        };
        write!(f, "{s}")
    }
}

/// Representation of a calendar date (day, month, year) from a profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Date {
    /// Day of the month (1-31).
    pub day: Option<u8>,
    /// Month of the year (1-12).
    pub month: Option<u8>,
    /// Year (e.g., `1990`).
    pub year: Option<i32>,
}

/// Public identity and background information for a LinkedIn profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseProfile {
    /// First / given name.
    pub first_name: String,
    /// Last / family name.
    pub last_name: String,
    /// Public vanity identifier (e.g., `"williamhgates"`).
    pub public_identifier: VanityName,
    /// Unique profile identifier / URN.
    pub profile_urn: ProfileId,
    /// Professional headline.
    pub headline: Option<String>,
    /// Custom pronoun if configured.
    pub pronoun: Option<String>,
    /// Formatted full location string.
    pub full_location: Option<String>,
    /// Country name.
    pub country: Option<String>,
    /// Country ISO code.
    pub country_iso_code: Option<String>,
    /// Postal code.
    pub postal_code: Option<String>,
    /// Whether creator mode is enabled.
    pub creator: Option<bool>,
    /// Whether the user has a LinkedIn Premium subscription.
    pub premium: Option<bool>,
    /// Whether the user is recognized as an Influencer.
    pub influencer: Option<bool>,
    pub birth_date_on: Option<Date>,
    pub member_distance: ConnectionDegree,
    pub connected_since: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawBaseProfile<'a> {
    #[serde(borrow)]
    pub first_name: Cow<'a, str>,
    #[serde(borrow)]
    pub last_name: Cow<'a, str>,
    #[serde(borrow)]
    pub public_identifier: Cow<'a, str>,
    #[serde(borrow, rename = "entityUrn")]
    pub profile_urn: Cow<'a, str>,
    #[serde(borrow)]
    pub headline: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub pronoun: Option<RawPronounInfo<'a>>,
    pub creator: Option<bool>,
    pub premium: Option<bool>,
    pub influencer: Option<bool>,
    pub birth_date_on: Option<Date>,
    #[serde(borrow)]
    pub geo_location: Option<RawGeoLocation<'a>>,
    pub member_relationship: Option<MemberRelationshipWrapper>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPronounInfo<'a> {
    #[serde(borrow)]
    pub custom_pronoun: Option<Cow<'a, str>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawGeoLocation<'a> {
    #[serde(borrow)]
    pub geo_location_name: Option<Cow<'a, str>>,
    pub geo: Option<Geo>,
    pub postal_code: Option<String>,
}

impl<'a> TryFrom<RawBaseProfile<'a>> for BaseProfile {
    type Error = crate::errors::LinkedInError;

    fn try_from(raw: RawBaseProfile<'a>) -> Result<Self, Self::Error> {
        let mut full_location = None;
        let mut country = None;
        let mut country_iso_code = None;
        let mut postal_code = None;

        if let Some(gl) = raw.geo_location {
            postal_code = gl.postal_code;
            full_location = gl.geo_location_name.map(|c| c.into_owned());
            if let Some(geo) = gl.geo {
                if full_location.as_deref().unwrap_or("").trim().is_empty() {
                    full_location = geo.default_localized_name_country_region_and_city;
                }
                country_iso_code = geo.country_iso_code;
                if let Some(c) = geo.country {
                    country = c.default_localized_name_country_only;
                }
            }
        }

        let (connection, no_connection) = match raw.member_relationship {
            Some(w) => (
                w.member_relationship.connection,
                w.member_relationship.no_connection,
            ),
            None => (None, None),
        };

        let (member_distance, connected_since) = if let Some(c) = connection {
            (ConnectionDegree::First, Some(c.created_at))
        } else if let Some(nc) = no_connection {
            (nc.member_distance.into(), None)
        } else {
            (ConnectionDegree::Unknown, None)
        };

        Ok(Self {
            first_name: raw.first_name.into_owned(),
            last_name: raw.last_name.into_owned(),
            public_identifier: raw.public_identifier.parse()?,
            profile_urn: raw.profile_urn.parse()?,
            headline: raw.headline.map(|c| c.into_owned()),
            pronoun: raw
                .pronoun
                .and_then(|p| p.custom_pronoun)
                .map(|c| c.into_owned()),
            full_location,
            country,
            country_iso_code,
            postal_code,
            creator: raw.creator,
            premium: raw.premium,
            influencer: raw.influencer,
            birth_date_on: raw.birth_date_on,
            member_distance,
            connected_since,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Geo {
    #[serde(rename = "defaultLocalizedName")]
    pub default_localized_name_country_region_and_city: Option<String>,
    #[serde(rename = "countryISOCode")]
    pub country_iso_code: Option<String>,
    pub country: Option<Country>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    #[serde(rename = "defaultLocalizedName")]
    pub default_localized_name_country_only: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemberRelationshipWrapper {
    pub member_relationship: MemberRelationshipContent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemberRelationshipContent {
    pub connection: Option<Connection>,
    pub no_connection: Option<NoConnection>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Connection {
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoConnection {
    pub member_distance: RawConnectionDegree,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum DynamicRoot<T> {
    Nested(RootNested<T>),
    Flat(RootFlat<T>),
}

impl<T> DynamicRoot<T> {
    pub fn into_elements(self) -> Vec<T> {
        match self {
            DynamicRoot::Nested(root) => {
                root.data
                    .data
                    .identity_dash_profiles_by_member_identity
                    .elements
            }
            DynamicRoot::Flat(root) => root.data.identity_dash_profiles_by_member_identity.elements,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct RootNested<T> {
    data: ResponseWrapper<T>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ResponseWrapper<T> {
    data: Data<T>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct RootFlat<T> {
    data: Data<T>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Data<T> {
    identity_dash_profiles_by_member_identity: CollectionResponse<T>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CollectionResponse<T> {
    elements: Vec<T>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkElement {
    following_state: Option<FollowingState>,
    connections: Option<Connections>,
}

impl From<DynamicRoot<NetworkElement>> for NetworkInfo {
    fn from(root: DynamicRoot<NetworkElement>) -> Self {
        root.into_elements()
            .into_iter()
            .next()
            .map(|e| NetworkInfo {
                following: e.following_state.as_ref().and_then(|fs| fs.following),
                follower_count: e.following_state.as_ref().and_then(|fs| fs.follower_count),
                total_connections: e
                    .connections
                    .as_ref()
                    .and_then(|c| c.paging.as_ref())
                    .and_then(|p| p.total),
            })
            .unwrap_or(NetworkInfo {
                following: None,
                follower_count: None,
                total_connections: None,
            })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FollowingState {
    following: Option<bool>,
    follower_count: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Connections {
    paging: Option<Paging>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Paging {
    total: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct FollowSet {
    pub following: bool,
}

#[derive(Serialize)]
pub(crate) struct FollowPatch {
    #[serde(rename = "$set")]
    pub set: FollowSet,
}

#[derive(Serialize)]
pub(crate) struct FollowPayload {
    pub patch: FollowPatch,
}
