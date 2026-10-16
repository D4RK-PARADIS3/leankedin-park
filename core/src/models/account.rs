//! Domain models for LinkedIn account and profile view metrics.

use crate::models::identifiers::{ProfileId, VanityName};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Profile data for the currently authenticated LinkedIn user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedInProfile {
    /// First / given name of the authenticated member.
    pub first_name: String,
    /// Last / family name of the authenticated member.
    pub last_name: String,
    /// Current headline or occupation text.
    pub occupation: Option<String>,
    /// Public vanity name identifier (e.g., `"williamhgates"`).
    pub vanity_name: VanityName,
    /// Custom pronoun string if defined in account settings.
    pub custom_pronoun: Option<String>,
    /// Unique profile identifier / URN.
    pub profile_urn: ProfileId,
    /// Indicates if the user has an active LinkedIn Premium subscription.
    pub premium_subscriber: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawProfileResponse<'a> {
    pub premium_subscriber: bool,
    #[serde(borrow)]
    pub mini_profile: RawMiniProfile<'a>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawMiniProfile<'a> {
    #[serde(borrow)]
    pub custom_pronoun: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub first_name: Cow<'a, str>,
    #[serde(borrow)]
    pub last_name: Cow<'a, str>,
    #[serde(borrow)]
    pub occupation: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub entity_urn: Cow<'a, str>,
    #[serde(borrow)]
    pub public_identifier: Cow<'a, str>,
}

impl<'a> TryFrom<RawProfileResponse<'a>> for LinkedInProfile {
    type Error = crate::errors::LinkedInError;

    fn try_from(raw: RawProfileResponse<'a>) -> Result<Self, Self::Error> {
        let mini = raw.mini_profile;
        Ok(Self {
            first_name: mini.first_name.into_owned(),
            last_name: mini.last_name.into_owned(),
            occupation: mini.occupation.map(|c| c.into_owned()),
            vanity_name: mini.public_identifier.parse()?,
            custom_pronoun: mini.custom_pronoun.map(|c| c.into_owned()),
            profile_urn: mini.entity_urn.parse()?,
            premium_subscriber: raw.premium_subscriber,
        })
    }
}

/// Statistics regarding views of your LinkedIn profile over the evaluated timeframe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileViews {
    /// Total view count recorded over the period.
    pub num_views: i64,
    /// Evaluated timeframe description (e.g., `"past 90 days"`).
    pub time_frame: String,
}

pub(crate) mod dto {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct RawResponse {
        pub elements: Vec<RawElement>,
    }

    #[derive(Deserialize)]
    pub struct RawElement {
        pub value: RawElementValue,
    }

    #[derive(Deserialize)]
    pub struct RawElementValue {
        #[serde(rename = "com.linkedin.voyager.identity.me.wvmpOverview.WvmpViewersCard")]
        pub viewers_card: RawViewersCard,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RawViewersCard {
        pub insight_cards: Vec<RawInsightCard>,
    }

    #[derive(Deserialize)]
    pub struct RawInsightCard {
        pub value: RawInsightValue,
    }

    #[derive(Deserialize)]
    pub struct RawInsightValue {
        #[serde(rename = "com.linkedin.voyager.identity.me.wvmpOverview.WvmpSummaryInsightCard")]
        pub summary: RawSummary,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RawSummary {
        pub num_views: i64,
        pub time_frame: String,
    }
}
