//! Account and authenticated user identity API operations.

use crate::api::{Linkedin, RequestType, ResponseExt};
use crate::errors::LinkedInError;
use crate::models::account::*;
use log::{info, warn};

/// Sub-client for inspecting the current authenticated LinkedIn account.
pub struct AccountClient<'a>(pub(crate) &'a Linkedin);

impl<'a> AccountClient<'a> {
    /// Retrieves profile view statistics over the last 90 days.
    ///
    /// # Returns
    ///
    /// A [`ProfileViews`] instance containing total view count and the time frame assessed.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the request fails or if the payload structure is unexpected.
    pub async fn get_current_profile_views_last_90_days(
        &self,
    ) -> Result<ProfileViews, LinkedInError> {
        info!("Fetching current profile views (last 90 days)...");

        let raw_data: dto::RawResponse = self
            .0
            ._fetch::<()>("/identity/wvmpCards", None, RequestType::ApiRequest)
            .await?
            .verify_success("Can't fetch last 90 days profile views.")?
            .json()?;

        let summary = raw_data
            .elements
            .into_iter()
            .next()
            .map(|el| el.value.viewers_card)
            .and_then(|card| card.insight_cards.into_iter().next())
            .map(|insight| insight.value.summary)
            .ok_or_else(|| {
                warn!("Structure mismatch: Could not find summary card in response");
                LinkedInError::Api("Profile views data not found in expected structure".to_string())
            })?;

        let result = ProfileViews {
            num_views: summary.num_views,
            time_frame: summary.time_frame,
        };

        info!(
            "Successfully retrieved profile views: {} ({})",
            result.num_views, result.time_frame
        );
        Ok(result)
    }

    /// Fetches the authenticated user's own profile information.
    ///
    /// # Returns
    ///
    /// A [`LinkedInProfile`] instance populated with public identifier, names, headline, and URN.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the network request fails or data cannot be parsed.
    pub async fn get_my_profile(&self) -> Result<LinkedInProfile, LinkedInError> {
        info!("Fetching your own profile info...");

        let response = self
            .0
            ._fetch::<()>("/me", None, RequestType::ApiRequest)
            .await?
            .verify_success("Can't fetch your profile informations")?;

        info!("Successfully fetched your profile informations");

        let raw: RawProfileResponse = response.json()?;
        let profile = LinkedInProfile::try_from(raw)?;

        Ok(profile)
    }
}
