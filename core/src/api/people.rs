//! Public profiles, network statistics, and follow management API.

use crate::api::{Linkedin, RequestType, ResponseExt};
use crate::errors::LinkedInError;
use crate::models::identifiers::{AsVanityStr, ProfileId};
use crate::models::people::*;
use log::{debug, warn};

/// Sub-client for fetching public profiles, network details, and managing follow relationships.
pub struct PeopleClient<'a>(pub(crate) &'a Linkedin);

impl<'a> PeopleClient<'a> {
    const QUERY_ID_PROFILE: &'static str =
        "voyagerIdentityDashProfiles.34ead06db82a2cc9a778fac97f69ad6a";
    const QUERY_ID_NETWORK: &'static str =
        "voyagerIdentityDashProfiles.a1a483e719b20537a256b6853cdca711";

    /// Retrieves network metrics (follower count, total connections, following state) for a profile.
    ///
    /// # Arguments
    ///
    /// * `vanity_name` - The target member's public identifier implementing [`AsVanityStr`].
    ///
    /// # Returns
    ///
    /// A [`NetworkInfo`] struct containing following state, follower count, and connection count.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the network request fails or data cannot be parsed.
    pub async fn get_network_infos_of_a_profile(
        &self,
        vanity_name: impl AsVanityStr,
    ) -> Result<NetworkInfo, LinkedInError> {
        let vanity_name = vanity_name.as_vanity_str()?;
        let url = format!(
            "/graphql?includeWebMetadata=true&variables=(vanityName:{})&queryId={}",
            vanity_name,
            Self::QUERY_ID_NETWORK
        );

        Ok(self
            .0
            ._fetch::<()>(&url, None, RequestType::ApiRequest)
            .await?
            .verify_success(format_args!(
                "Failed to fetch the network info of {}",
                vanity_name
            ))?
            .json::<DynamicRoot<NetworkElement>>()?
            .into())
    }

    /// Retrieves base public identity and career information for a profile.
    ///
    /// # Arguments
    ///
    /// * `vanity_name` - The target member's public identifier implementing [`AsVanityStr`].
    ///
    /// # Returns
    ///
    /// A [`BaseProfile`] struct containing names, headline, location, and distance.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the profile is not found or request fails.
    pub async fn get_infos_of_a_profile(
        &self,
        vanity_name: impl AsVanityStr,
    ) -> Result<BaseProfile, LinkedInError> {
        let vanity_name = vanity_name.as_vanity_str()?;
        let url = format!(
            "/graphql?includeWebMetadata=true&variables=(vanityName:{})&queryId={}",
            vanity_name,
            Self::QUERY_ID_PROFILE
        );

        let response = self
            .0
            ._fetch::<()>(&url, None, RequestType::ApiRequest)
            .await?
            .verify_success(format_args!("Failed to fetch infos of {}.", vanity_name))?;

        let elements = response
            .json::<DynamicRoot<RawBaseProfile>>()?
            .into_elements();

        let raw_profile = elements
            .into_iter()
            .next()
            .ok_or_else(|| LinkedInError::Api(format!("Profile {} not found", vanity_name)))?;

        raw_profile.try_into()
    }

    /// Retrieves comprehensive profile details, combining base identity and network metrics in one call.
    ///
    /// # Arguments
    ///
    /// * `vanity_name` - The target member's public identifier implementing [`AsVanityStr`].
    ///
    /// # Returns
    ///
    /// A [`FullProfile`] struct.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the base identity cannot be retrieved.
    pub async fn get_full_infos_of_a_profile(
        &self,
        vanity_name: impl AsVanityStr,
    ) -> Result<FullProfile, LinkedInError> {
        let vanity_name = vanity_name.as_vanity_str()?;
        let identity = self.get_infos_of_a_profile(vanity_name).await?;

        let network_info = match self.get_network_infos_of_a_profile(vanity_name).await {
            Ok(info) => {
                debug!("Network info successfully fetched for {}", vanity_name);
                Some(info)
            }
            Err(e) => {
                warn!(
                    "Failed to fetch network info for {} : {}. Continuing with base profile.",
                    vanity_name, e
                );
                None
            }
        };

        Ok(FullProfile::new(identity, network_info))
    }

    /// Follows a target LinkedIn member to receive their updates in your feed.
    ///
    /// # Arguments
    ///
    /// * `target_urn` - The validated [`ProfileId`] of the target member.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the follow action fails.
    pub async fn follow_people(&self, target_urn: &ProfileId) -> Result<(), LinkedInError> {
        let slug = format!(
            "/feed/dash/followingStates/urn:li:fsd_followingState:urn:li:fsd_profile:{}",
            target_urn.id()
        );

        let payload = FollowPayload {
            patch: FollowPatch {
                set: FollowSet { following: true },
            },
        };

        self.0
            ._post::<FollowPayload, ()>(&slug, Some(&payload), None, RequestType::ApiRequest)
            .await?
            .verify_success(format_args!("Failed to follow user {}", target_urn.id()))?;

        Ok(())
    }

    /// Unfollows a target LinkedIn member without disconnecting.
    ///
    /// # Arguments
    ///
    /// * `target_urn` - The validated [`ProfileId`] of the target member.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the unfollow action fails.
    pub async fn unfollow_people(&self, target_urn: &ProfileId) -> Result<(), LinkedInError> {
        let slug = format!(
            "/feed/dash/followingStates/urn:li:fsd_followingState:urn:li:fsd_profile:{}",
            target_urn.id()
        );

        let payload = FollowPayload {
            patch: FollowPatch {
                set: FollowSet { following: false },
            },
        };

        self.0
            ._post::<_, ()>(&slug, Some(&payload), None, RequestType::ApiRequest)
            .await?
            .verify_success(format_args!("Failed to unfollow user {}", target_urn.id()))?;

        Ok(())
    }
}
