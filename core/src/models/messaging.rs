//! Domain models for messaging conversations, categories, and participants.

use crate::errors::LinkedInError;
use crate::models::identifiers::{ConversationId, FsdProfileUrn, ProfileId};
use crate::models::people::{ConnectionDegree, RawConnectionDegree};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageBodyPayload<'a> {
    pub message: MessageContent<'a>,
    pub mailbox_urn: FsdProfileUrn<'a>,
    pub tracking_id: String,
    pub dedupe_by_client_generated_token: bool,
    pub host_recipient_urns: Vec<FsdProfileUrn<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageContent<'a> {
    pub body: MessageBody<'a>,
    pub origin_token: &'a str,
    pub render_content_unions: &'a [&'a str],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageBody<'a> {
    pub attributes: &'a [()],
    pub text: &'a str,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct LinkedInApi<'a> {
    #[serde(borrow)]
    pub data: Data<'a>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Data<'a> {
    #[serde(borrow)]
    pub messenger_conversations_by_sync_token: MessengerConversations<'a>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct MessengerConversations<'a> {
    #[serde(borrow)]
    pub elements: Vec<Conversation<'a>>,
}

/// Mailbox categories and folders for LinkedIn messaging threads.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    /// Standard inbox.
    Inbox,
    /// Primary priority inbox folder.
    PrimaryInbox,
    /// InMail sponsored / premium messages folder.
    Inmail,
    /// Archived conversation folder.
    Archive,
    /// Fallback for unhandled or newly introduced LinkedIn categories.
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Conversation<'a> {
    pub backend_urn: Cow<'a, str>,
    pub unread_count: u32,
    pub read: bool,
    pub group_chat: bool,

    #[serde(default)]
    pub categories: Vec<Category>,

    pub created_at: u64,
    pub last_activity_at: u64,
    pub last_read_at: Option<u64>,
    #[serde(borrow)]
    pub conversation_participants: Vec<Participant<'a>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Participant<'a> {
    pub backend_urn: Cow<'a, str>,
    #[serde(borrow)]
    pub participant_type: ParticipantType<'a>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct ParticipantType<'a> {
    #[serde(borrow)]
    pub member: Option<Member<'a>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Member<'a> {
    #[serde(
        borrow,
        default,
        deserialize_with = "crate::helpers::deserialize_optional_text"
    )]
    pub first_name: Option<Cow<'a, str>>,
    #[serde(
        borrow,
        default,
        deserialize_with = "crate::helpers::deserialize_optional_text"
    )]
    pub last_name: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub profile_url: Option<Cow<'a, str>>,
    pub distance: Option<RawConnectionDegree>,
}

/// Cleaned and validated representation of a LinkedIn messaging conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationData {
    /// Unique identifier of the conversation.
    pub urn: ConversationId,
    /// Number of unread messages in the thread.
    pub unread_count: u32,
    /// Indicates whether the conversation has been read.
    pub is_read: bool,
    /// Indicates whether the conversation is a group chat.
    pub is_group: bool,
    /// Folder categories to which this conversation belongs.
    pub categories: Vec<Category>,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at: u64,
    /// Last message or event timestamp in milliseconds since Unix epoch.
    pub last_activity_at: u64,
    /// Timestamp when the conversation was last viewed.
    pub last_read_at: Option<u64>,
    /// List of participants participating in this conversation.
    pub participants: Vec<ConversationParticipant>,
}

/// Information regarding a participant in a LinkedIn messaging thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationParticipant {
    /// Public profile URL if available.
    pub profile_url: Option<String>,
    /// Given / first name of the participant.
    pub first_name: Option<String>,
    /// Family / last name of the participant.
    pub last_name: Option<String>,
    /// Network connection distance relative to the authenticated user.
    pub distance: Option<ConnectionDegree>,
    /// Validated profile ID of the participant.
    pub urn: ProfileId,
}

impl ConversationData {
    /// Returns `true` if the conversation contains at least two human participants.
    ///
    /// Used to distinguish actual conversations from automated bots or system notifications.
    pub fn is_human_conversation(&self) -> bool {
        self.participants.len() >= 2
    }
}

impl<'a> TryFrom<Conversation<'a>> for ConversationData {
    type Error = LinkedInError;

    fn try_from(raw: Conversation<'a>) -> Result<Self, Self::Error> {
        let mut participants = Vec::new();

        for p in raw.conversation_participants {
            if let Some(member) = p.participant_type.member {
                participants.push(ConversationParticipant {
                    first_name: member.first_name.map(|n| n.into_owned()),
                    last_name: member.last_name.map(|n| n.into_owned()),
                    profile_url: member.profile_url.map(|n| n.into_owned()),
                    distance: member.distance.map(Into::into),
                    urn: p.backend_urn.parse()?,
                });
            }
        }

        Ok(ConversationData {
            urn: raw.backend_urn.parse()?,
            unread_count: raw.unread_count,
            is_read: raw.read,
            is_group: raw.group_chat,
            categories: raw.categories,
            created_at: raw.created_at,
            last_activity_at: raw.last_activity_at,
            last_read_at: raw.last_read_at,
            participants,
        })
    }
}
