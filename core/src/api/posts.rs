//! Post publishing and activity reaction API.

use crate::api::{Linkedin, RequestType, ResponseExt};
use crate::errors::{LinkedInError, PublishPostError};
use crate::models::identifiers::ActivityUrn;
use crate::models::posts::*;

/// Sub-client for publishing posts and reacting to activities.
pub struct PostsClient<'a>(pub(crate) &'a Linkedin);

impl<'a> PostsClient<'a> {
    /// Reacts to a post or activity with a specified reaction type (Like, Praise, etc.).
    ///
    /// # Arguments
    ///
    /// * `type_of_the_reaction` - The [`ReactionType`] to apply.
    /// * `urn_of_the_post` - The post or activity URN string (e.g., `"urn:li:activity:..."`).
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the reaction could not be registered by the API.
    pub async fn react_to_post(
        &self,
        type_of_the_reaction: ReactionType,
        urn_of_the_post: &str,
    ) -> Result<(), LinkedInError> {
        const QUERY_ID_HASH: &str = "voyagerSocialDashReactions.b731222600772fd42464c0fe19bd722b";
        const PARAMS: [(&str, &str); 2] = [("action", "execute"), ("queryId", QUERY_ID_HASH)];

        let payload = ReactionPayload {
            variables: ReactionVariables {
                entity: ReactionEntity {
                    reaction_type: type_of_the_reaction,
                },
                thread_urn: ActivityUrn(urn_of_the_post),
            },
            query_id: QUERY_ID_HASH,
            include_web_metadata: true,
        };

        self.0
            ._post(
                "/graphql",
                Some(&payload),
                Some(&PARAMS),
                RequestType::ApiRequest,
            )
            .await?
            .verify_success("Failed to react to post.")?;

        Ok(())
    }

    /// Publishes a new post to the authenticated member's LinkedIn feed.
    ///
    /// # Arguments
    ///
    /// * `post` - The [`NewPost`] configuration containing text, visibility, and commenting settings.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError::PublishPost`] if the content is empty or exceeds 3,000 characters.
    /// Returns [`LinkedInError`] if the GraphQL publication request fails.
    pub async fn publish_post(&self, post: &NewPost) -> Result<(), LinkedInError> {
        const QUERY_ID: &str = "voyagerContentcreationDashShares.279996efa5064c01775d5aff003d9377";
        const MAX_LENGTH: u32 = 3_000;
        let trimmed_text = post.text.trim();
        if trimmed_text.is_empty() {
            return Err(PublishPostError::EmptyContent.into());
        }
        let char_count = trimmed_text.chars().count();

        if char_count > 3_000 {
            return Err(PublishPostError::ContentTooLong(char_count, MAX_LENGTH).into());
        }

        let payload = post.to_payload(QUERY_ID);

        let params = [("action", "execute"), ("queryId", QUERY_ID)];

        self.0
            ._post(
                "/graphql",
                Some(&payload),
                Some(&params),
                RequestType::ApiRequest,
            )
            .await?
            .verify_success("Failed to publish post.")?;

        Ok(())
    }
}
