//! Connection requests, invitations, and relationship management API.

use super::Linkedin;
use crate::api::RequestType;
use crate::api::ResponseExt;
use crate::errors::LinkedInError;
use crate::errors::MessagingError;
use crate::models::connections::*;
use crate::models::identifiers::{AsVanityStr, FsdProfileUrn, InvitationId, ProfileId};
use log::{info, warn};

/// Sub-client for inspecting, accepting, rejecting, and creating LinkedIn connections.
pub struct ConnectionsClient<'a>(pub(crate) &'a Linkedin);

impl<'a> ConnectionsClient<'a> {
    /// Retrieves pending received connection invitations with pagination.
    ///
    /// # Arguments
    ///
    /// * `start` - The pagination offset index (starts at 0).
    /// * `limit` - The maximum number of invitations to return.
    ///
    /// # Returns
    ///
    /// A list of [`ReceivedInvitation`] instances.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the network request fails or data cannot be parsed.
    pub async fn get_invitations(
        &self,
        start: u32,
        limit: u32,
    ) -> Result<Vec<ReceivedInvitation>, LinkedInError> {
        let params = InvitationQueryParams {
            start,
            count: limit,
            include_insights: "true",
            q: "receivedInvitation",
        };

        let binding = self
            .0
            ._fetch(
                "/relationships/invitationViews",
                Some(&params),
                RequestType::ApiRequest,
            )
            .await?
            .verify_success(format_args!(
                "Failed to retrieve the invitations starting from {start} with limit of {limit}."
            ))?; // must be done in two steps otherwise it gets freed

        let resp: RawInvitationResponse = binding.json()?;

        let invitations = resp
            .elements
            .into_iter()
            .filter_map(|wrapper| match wrapper {
                ElementWrapper::Valid(strict) => {
                    let inv = strict.invitation;
                    let member = inv.from_member;

                    Some(ReceivedInvitation {
                        entity_urn: inv.entity_urn,
                        from_member_urn: member.entity_urn,
                        invitation_type: inv.invitation_type.into_owned(),
                        custom_message: inv.custom_message,
                        shared_secret: inv.shared_secret.into_owned(),
                        unseen: inv.unseen,
                        sent_time: inv.sent_time,
                        first_name: member.first_name.into_owned(),
                        last_name: member.last_name.into_owned(),
                        occupation: member.occupation.map(|o| o.into_owned()),
                    })
                }
                ElementWrapper::Ignored(_) => None,
            })
            .collect();

        Ok(invitations)
    }

    /// Responds to a received connection invitation by accepting or rejecting it.
    ///
    /// # Arguments
    ///
    /// * `invitation_entity_urn` - The validated [`InvitationId`] of the invitation.
    /// * `invitation_shared_secret` - The shared secret string attached to the invitation.
    /// * `action` - Whether to [`InvitationAction::Accept`] or [`InvitationAction::Reject`].
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the API fails to register the response.
    pub async fn reply_invitation(
        &self,
        invitation_entity_urn: &InvitationId,
        invitation_shared_secret: &str,
        action: InvitationAction,
    ) -> Result<(), LinkedInError> {
        let urn = invitation_entity_urn;
        let invitation_id = urn.id();

        let url = format!("/relationships/invitations/{}", invitation_id);
        let params = [("action", action.as_str())];
        let payload = ReplyPayload {
            invitation_id: urn,
            invitation_shared_secret,
            is_generic_invitation: false,
        };

        self.0
            ._post(&url, Some(&payload), Some(&params), RequestType::ApiRequest)
            .await?
            .verify_success(format_args!(
                "Reply '{}' to invitation {} has failed",
                action.as_str(),
                invitation_id
            ))?;

        Ok(())
    }

    /// Sends a connection request to another LinkedIn user.
    ///
    /// # Arguments
    ///
    /// * `request` - The [`ConnectionRequest`] containing the target identifier, optional message, and optional cached URN.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the custom message exceeds 300 characters, or if the request cannot be sent.
    pub async fn send_connection_request(
        &self,
        request: &ConnectionRequest,
    ) -> Result<(), LinkedInError> {
        if let Some(msg) = &request.custom_message {
            let char_count = msg.chars().count();
            // 300 is the max length for premium users. It's 200 for free users.
            if char_count > 300 {
                warn!("Message too long ({char_count} > 300). Connection request aborted.");
                return Err(MessagingError::MessageTooLong.into());
            }
        }

        let fetched_profile;
        let urn_ref: &ProfileId = match &request.profile_urn {
            Some(urn) => urn,
            None => {
                fetched_profile = self
                    .0
                    .people()
                    .get_infos_of_a_profile(&request.target_vanity_name)
                    .await?;
                &fetched_profile.profile_urn
            }
        };

        let payload = AddConnectionPayload {
            invitee: InviteeWrapper {
                invitee_union: MemberProfileWrapper {
                    member_profile: FsdProfileUrn(urn_ref),
                },
            },
            custom_message: request.custom_message.as_deref(),
        };

        let params = [
            ("action", "verifyQuotaAndCreateV2"),
            (
                "decorationId",
                "com.linkedin.voyager.dash.deco.relationships.InvitationCreationResultWithInvitee-2",
            ),
        ];

        self.0
            ._post::<AddConnectionPayload, _>(
                "/voyagerRelationshipsDashMemberRelationships",
                Some(&payload),
                Some(&params),
                RequestType::ApiRequest,
            )
            .await?
            .verify_success(format_args!(
                "Failed to send connection request to {}",
                request.target_vanity_name
            ))?;

        info!(
            "Connection request sent successfully to {}",
            request.target_vanity_name
        );

        Ok(())
    }

    /// Removes an existing 1st-degree connection from your network.
    ///
    /// # Arguments
    ///
    /// * `vanity_name` - The target user's vanity name or profile identifier implementing [`AsVanityStr`].
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the disconnect action fails.
    pub async fn remove_connection(
        &self,
        vanity_name: impl AsVanityStr,
    ) -> Result<(), LinkedInError> {
        let vanity_name = vanity_name.as_vanity_str()?;
        let url = format!("/identity/profiles/{}/profileActions", vanity_name);

        let params = [("action", "disconnect")];

        self.0
            ._post::<(), _>(&url, None, Some(&params), RequestType::ApiRequest)
            .await?
            .verify_success(format_args!(
                "Failed to remove connection of user {}",
                vanity_name
            ))?;

        Ok(())
    }
}
