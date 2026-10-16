//! LinkedIn messaging API for direct messages, group chats, and conversations.

use crate::api::{Linkedin, RequestType, ResponseExt};
use crate::errors::LinkedInError;
use crate::helpers::{generate_linkedin_tracking_id, generate_v4_uuid};
use crate::models::identifiers::{FsdProfileUrn, ProfileId};
use crate::models::messaging::*;
use log::info;

/// Sub-client for reading conversations and sending direct and group messages.
pub struct MessagingClient<'a>(pub(crate) &'a Linkedin);

impl<'a> MessagingClient<'a> {
    /// Sends a direct message to a specific LinkedIn member.
    ///
    /// Automatically resolves and attaches the authenticated user's mailbox URN.
    ///
    /// # Arguments
    ///
    /// * `message_to_send` - The body text of the message.
    /// * `urn_destinataire` - The validated [`ProfileId`] of the recipient.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the message payload cannot be serialized or the API request fails.
    pub async fn send_message(
        &self,
        message_to_send: &str,
        urn_destinataire: &ProfileId,
    ) -> Result<(), LinkedInError> {
        let my_urn = self.0.get_or_fetch_urn().await?;

        const PARAMS: [(&str, &str); 1] = [("action", "createMessage")];

        let origin_token = generate_v4_uuid().to_string();
        let tracking_id = generate_linkedin_tracking_id();

        let payload = MessageBodyPayload {
            message: MessageContent {
                body: MessageBody {
                    attributes: &[],
                    text: message_to_send,
                },
                origin_token: &origin_token,
                render_content_unions: &[],
            },
            mailbox_urn: FsdProfileUrn(my_urn),
            tracking_id,
            dedupe_by_client_generated_token: false,
            host_recipient_urns: vec![FsdProfileUrn(urn_destinataire)],
        };

        let body_string = serde_json::to_string(&payload)?;
        // The serialized body contains the message and recipient identifiers.
        // Transport logging reports only byte counts; never duplicate its body here.

        self.0
            ._post_raw(
                "/voyagerMessagingDashMessengerMessages",
                body_string,
                Some(&PARAMS),
                RequestType::ApiRequest,
            )
            .await?
            .verify_success("Failed to send message")?;

        info!("Successfully sent a direct message");
        Ok(())
    }

    /// Sends a group message to multiple LinkedIn members.
    ///
    /// # Arguments
    ///
    /// * `message_to_send` - The body text of the message.
    /// * `urn_destinataires` - A slice of validated [`ProfileId`] instances of all intended recipients.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the message fails to send.
    pub async fn send_group_message(
        &self,
        message_to_send: &str,
        urn_destinataires: &[ProfileId],
    ) -> Result<(), LinkedInError> {
        let my_urn = self.0.get_or_fetch_urn().await?;

        const PARAMS: [(&str, &str); 1] = [("action", "createMessage")];

        let origin_token = generate_v4_uuid().to_string();
        let tracking_id = generate_linkedin_tracking_id();

        let host_recipient_urns: Vec<FsdProfileUrn> =
            urn_destinataires.iter().map(FsdProfileUrn).collect();

        let payload = MessageBodyPayload {
            message: MessageContent {
                body: MessageBody {
                    attributes: &[],
                    text: message_to_send,
                },
                origin_token: &origin_token,
                render_content_unions: &[],
            },
            mailbox_urn: FsdProfileUrn(my_urn),
            tracking_id,
            dedupe_by_client_generated_token: false,
            host_recipient_urns,
        };

        let body_string = serde_json::to_string(&payload)?;

        self.0
            ._post_raw(
                "/voyagerMessagingDashMessengerMessages",
                body_string,
                Some(&PARAMS),
                RequestType::ApiRequest,
            )
            .await?
            .verify_success("Failed to send message")?;

        // Report the operation without exposing group membership in application logs.
        info!("Successfully sent a group message");
        Ok(())
    }

    /// Retrieves the list of human-to-human messaging conversations.
    ///
    /// Automatically filters out automated bot or system notification channels.
    ///
    /// # Returns
    ///
    /// A list of [`ConversationData`] instances.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError`] if the GraphQL query fails or conversation elements cannot be parsed.
    pub async fn get_conversations_list(&self) -> Result<Vec<ConversationData>, LinkedInError> {
        let my_urn = self.0.get_or_fetch_urn().await?;

        const QUERY_ID: &str = "messengerConversations.0d5e6781bbee71c3e51c8843c6519f48";

        let variables = format!("(mailboxUrn:urn%3Ali%3Afsd_profile%3A{})", my_urn);

        let url = format!(
            "/voyagerMessagingGraphQL/graphql?queryId={}&variables={}",
            QUERY_ID, variables
        );

        let binding = self
            .0
            ._fetch::<()>(&url, None, RequestType::ApiRequest)
            .await?
            .verify_success(format_args!("Failed to fetch your list of conversations."))?;

        let parsed_data: LinkedInApi = binding.json()?;

        let raw_conversations = parsed_data
            .data
            .messenger_conversations_by_sync_token
            .elements;

        let clean_conversations: Vec<ConversationData> = raw_conversations
            .into_iter()
            .map(ConversationData::try_from)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|c| c.is_human_conversation())
            .collect();

        Ok(clean_conversations)
    }
}
