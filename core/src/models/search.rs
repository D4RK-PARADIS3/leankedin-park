//! Domain models and builders for searching LinkedIn members.

use crate::errors::{LinkedInError, UrlError};
use crate::helpers::extract_id_from_people_search;
use crate::models::identifiers::ProfileId;
use crate::models::people::ConnectionDegree;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::{collections::HashSet, fmt};
use typed_builder::TypedBuilder;

use url::Url;

impl SearchPeopleBuilder {
    /// Parses a standard browser LinkedIn people search URL into a populated [`SearchPeopleBuilder`].
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the URL is malformed.
    pub fn from_url(url_str: &str) -> Result<Self, LinkedInError> {
        let url = Url::parse(url_str).map_err(UrlError::CantParse)?;
        let mut builder = SearchPeopleBuilder::default();
        let clean = |s: &str| s.trim_matches('"').to_string();

        // As in the original implementation, an unreadable JSON list produces an empty list
        // and a warning. URL parsing does not add strict validation.
        let parse_list = |v: &str| match serde_json::from_str::<Vec<String>>(v) {
            Ok(vec) => vec,
            Err(e) => {
                log::warn!("Failed to parse JSON array from URL parameter '{v}': {e}");
                Vec::new()
            }
        };

        for (key, val) in url.query_pairs() {
            match key.as_ref() {
                "keywords" => builder.keywords = Some(val.to_string()),
                "firstName" => builder.keyword_first_name = Some(clean(&val)),
                "lastName" => builder.keyword_last_name = Some(clean(&val)),
                "title" => builder.keyword_title = Some(clean(&val)),
                "company" => builder.keyword_company = Some(clean(&val)),
                "schoolFreetext" => builder.keyword_school = Some(clean(&val)),
                "geoUrn" => builder.regions = parse_list(&val),
                "industry" => builder.industries = parse_list(&val),
                "currentCompany" => builder.current_company = parse_list(&val),
                "pastCompany" => builder.past_companies = parse_list(&val),
                "schoolFilter" => builder.schools = parse_list(&val),
                "serviceCategory" => builder.service_categories = parse_list(&val),
                "profileLanguage" => builder.profile_languages = parse_list(&val),
                "followerOf" => builder.follower_of = parse_list(&val),
                "nonprofitInterest" => builder.nonprofit_interests = parse_list(&val),
                "contactInterests" => builder.contact_interests = parse_list(&val),
                "connectionOf" => {
                    let l = parse_list(&val);
                    if let Some(first) = l.first() {
                        builder.connection_of = Some(first.clone());
                    } else {
                        builder.connection_of = Some(clean(&val).replace(['[', ']', '"'], ""));
                    }
                }
                "facetConnectionOf" => builder.connection_of = Some(clean(&val)),
                "network" => {
                    let codes = parse_list(&val);
                    let depths: HashSet<ConnectionDegree> = codes
                        .iter()
                        .filter_map(|c| ConnectionDegree::from_search_code(c))
                        .collect();
                    if !depths.is_empty() {
                        builder.network_depths = Some(depths);
                    }
                }
                "facetNetwork" => {
                    if let Some(d) = ConnectionDegree::from_search_code(&clean(&val)) {
                        builder
                            .network_depths
                            .get_or_insert_with(HashSet::new)
                            .insert(d);
                    }
                }
                "openToVolunteer" if val.contains("true") => {
                    builder.open_to_volunteer = Some(true);
                }
                _ => {}
            }
        }
        Ok(builder)
    }

    /// Sets the maximum number of people results to return.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the initial pagination offset (0-based index).
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

impl TryFrom<&str> for SearchPeopleBuilder {
    type Error = LinkedInError;

    #[inline]
    fn try_from(url: &str) -> Result<Self, Self::Error> {
        Self::from_url(url)
    }
}

impl TryFrom<&Url> for SearchPeopleBuilder {
    type Error = LinkedInError;

    #[inline]
    fn try_from(url: &Url) -> Result<Self, Self::Error> {
        Self::from_url(url.as_str())
    }
}

/// Builder for configuring criteria when searching for LinkedIn members.
///
/// Can be constructed using [`SearchPeopleBuilder::builder()`] or created from
/// a browser URL using [`SearchPeopleBuilder::from_url`].
#[derive(Debug, Default, Clone, TypedBuilder)]
pub struct SearchPeopleBuilder {
    /// Global keyword query string.
    #[builder(default, setter(strip_option, into))]
    pub keywords: Option<String>,
    /// Maximum number of search results to return.
    #[builder(default, setter(strip_option))]
    pub limit: Option<usize>,
    /// Pagination offset.
    #[builder(default)]
    pub offset: usize,
    /// Filter by 1st-degree connections of a specific profile URN.
    #[builder(default, setter(strip_option, into))]
    pub connection_of: Option<String>,
    /// Filter by relationship distance (1st, 2nd, 3rd degree).
    #[builder(default, setter(strip_option, into))]
    pub network_depths: Option<HashSet<ConnectionDegree>>,

    /// Filter by current company URNs or names.
    #[builder(default)]
    pub current_company: Vec<String>,
    /// Filter by past company URNs or names.
    #[builder(default)]
    pub past_companies: Vec<String>,
    /// Filter by nonprofit interests.
    #[builder(default)]
    pub nonprofit_interests: Vec<String>,
    /// Filter by profile languages (e.g., `["en", "fr"]`).
    #[builder(default)]
    pub profile_languages: Vec<String>,
    /// Filter by geographical region URNs.
    #[builder(default)]
    pub regions: Vec<String>,
    /// Filter by industry codes.
    #[builder(default)]
    pub industries: Vec<String>,
    /// Filter by school / university URNs.
    #[builder(default)]
    pub schools: Vec<String>,
    /// Filter by contact interests.
    #[builder(default)]
    pub contact_interests: Vec<String>,
    /// Filter by service category codes.
    #[builder(default)]
    pub service_categories: Vec<String>,
    /// Filter by followers of specific organization URNs.
    #[builder(default)]
    pub follower_of: Vec<String>,

    /// Whether to include private profiles in search results.
    #[builder(default)]
    pub include_private_profiles: bool,
    /// Targeted first name search keyword.
    #[builder(default, setter(strip_option, into))]
    pub keyword_first_name: Option<String>,
    /// Targeted last name search keyword.
    #[builder(default, setter(strip_option, into))]
    pub keyword_last_name: Option<String>,
    /// Targeted job title keyword.
    #[builder(default, setter(strip_option, into))]
    pub keyword_title: Option<String>,
    /// Targeted company name keyword.
    #[builder(default, setter(strip_option, into))]
    pub keyword_company: Option<String>,
    /// Targeted school name keyword.
    #[builder(default, setter(strip_option, into))]
    pub keyword_school: Option<String>,
    /// Filter for members open to volunteering.
    #[builder(default, setter(strip_option))]
    pub open_to_volunteer: Option<bool>,
}

/// An individual profile result returned by a LinkedIn people search.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct People {
    /// Full display name of the member.
    #[serde(rename(serialize = "name"))]
    pub full_name: String,

    /// Network connection distance relative to the authenticated user.
    pub distance: ConnectionDegree,

    /// Short bio or summary snippet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Primary subtitle (usually job title / headline).
    #[serde(rename(serialize = "job"), skip_serializing_if = "Option::is_none")]
    pub primary_subtitle: Option<String>,

    /// Secondary subtitle (usually location or shared connections).
    #[serde(
        rename(serialize = "location"),
        skip_serializing_if = "Option::is_none"
    )]
    pub secondary_subtitle: Option<String>,

    /// Public LinkedIn navigation URL if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub navigation_url: Option<String>,

    /// Extracted profile identifier / URN if available in search cluster payload.
    /// Independent of distance: an out-of-network profile may have an identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<ProfileId>,
}

impl<'a> TryFrom<IndividualPeopleData<'a>> for People {
    type Error = LinkedInError;

    fn try_from(item: IndividualPeopleData<'a>) -> Result<Self, Self::Error> {
        let distance = item.entity_custom_tracking_info.member_distance;

        // Missing actions are allowed; present but malformed actions are errors.
        let profile_id = item
            .lazy_loaded_actions
            .map(|actions| extract_id_from_people_search(&actions.entity_urn))
            .transpose()?;

        Ok(People {
            full_name: item.title.into_owned(),
            distance,
            summary: item.summary.map(|s| s.into_owned()),
            primary_subtitle: item.primary_subtitle.map(|s| s.into_owned()),
            secondary_subtitle: item.secondary_subtitle.map(|s| s.into_owned()),
            navigation_url: item.navigation_url.map(|s| s.into_owned()),
            profile_id,
        })
    }
}

impl fmt::Display for People {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.full_name, self.distance)?;
        if let Some(job) = &self.primary_subtitle {
            write!(f, "\nJob: {job}")?;
        }
        if let Some(loc) = &self.secondary_subtitle {
            write!(f, "\nLocation: {loc}")?;
        }
        if let Some(summary) = &self.summary {
            write!(f, "\nSummary: {summary}")?;
        }
        if let Some(profile_id) = &self.profile_id {
            write!(f, "\nID (URN) : {profile_id}")?;
        }
        Ok(())
    }
}

pub(crate) mod dto {
    use super::IndividualPeopleData;
    use serde::Deserialize;
    use std::borrow::Cow;

    #[derive(Deserialize)]
    pub struct ResponseRoot<'a> {
        #[serde(borrow)]
        pub data: Option<ResponseData<'a>>,
    }

    #[derive(Deserialize)]
    pub struct ResponseData<'a> {
        #[serde(borrow, rename = "searchDashClustersByAll")]
        pub clusters: Option<Clusters<'a>>,
    }

    #[derive(Deserialize)]
    pub struct Clusters<'a> {
        #[serde(borrow, rename = "_type")]
        pub type_name: Cow<'a, str>,
        #[serde(borrow)]
        pub elements: Vec<ClusterElement<'a>>,
        pub paging: Option<Paging>,
    }

    #[derive(Deserialize, Debug, Clone, Copy)]
    pub struct Paging {
        pub total: usize,
        pub start: usize,
        pub count: usize,
    }

    #[derive(Deserialize)]
    pub struct ClusterElement<'a> {
        #[serde(borrow, rename = "_type")]
        pub type_name: Cow<'a, str>,
        // Unrelated modules may omit items; search clusters must provide them.
        #[serde(borrow)]
        pub items: Option<Vec<ClusterItem<'a>>>,
    }

    #[derive(Deserialize)]
    pub struct ClusterItem<'a> {
        #[serde(borrow, rename = "_type")]
        pub type_name: Cow<'a, str>,
        #[serde(borrow)]
        pub item: ItemDetail<'a>,
    }

    #[derive(Deserialize)]
    pub struct ItemDetail<'a> {
        // Cards that are not profiles may omit this value.
        #[serde(borrow, rename = "entityResult")]
        pub entity_result: Option<IndividualPeopleData<'a>>,
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndividualPeopleData<'a> {
    #[serde(borrow, deserialize_with = "crate::helpers::deserialize_text")]
    pub title: Cow<'a, str>,
    pub entity_custom_tracking_info: EntityCustomTrackingInfo,
    #[serde(
        borrow,
        default,
        deserialize_with = "crate::helpers::deserialize_optional_text"
    )]
    pub summary: Option<Cow<'a, str>>,
    #[serde(
        borrow,
        default,
        deserialize_with = "crate::helpers::deserialize_optional_text"
    )]
    pub primary_subtitle: Option<Cow<'a, str>>,
    #[serde(
        borrow,
        default,
        deserialize_with = "crate::helpers::deserialize_optional_text"
    )]
    pub secondary_subtitle: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub lazy_loaded_actions: Option<LazyLoadedActions<'a>>,
    #[serde(borrow)]
    pub navigation_url: Option<Cow<'a, str>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EntityCustomTrackingInfo {
    pub member_distance: ConnectionDegree,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LazyLoadedActions<'a> {
    #[serde(borrow)]
    pub entity_urn: Cow<'a, str>,
}
