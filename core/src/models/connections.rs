//! Domain models for connection requests, invitation payloads, and responses.

use crate::models::identifiers::{FsdProfileUrn, InvitationId, ProfileId, VanityName};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typed_builder::TypedBuilder;

/// Parameters for sending a connection request to a LinkedIn member.
///
/// Built using [`ConnectionRequest::builder()`].
#[derive(Debug, Clone, TypedBuilder)]
pub struct ConnectionRequest {
    /// The public profile vanity identifier (e.g., `"williamhgates"`).
    pub(crate) target_vanity_name: VanityName,

    /// An optional custom message note (maximum 300 characters).
    #[builder(default, setter(into, strip_option))]
    pub(crate) custom_message: Option<String>,

    /// The exact profile URN/ID of the target member, if already known.
    #[builder(default, setter(strip_option))]
    pub(crate) profile_urn: Option<ProfileId>,
}

/// Action taken when responding to a received LinkedIn connection invitation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvitationAction {
    /// Accepts the connection invitation, adding the sender to 1st-degree connections.
    Accept,
    /// Rejects the connection invitation.
    Reject,
}

impl InvitationAction {
    /// Returns the API string parameter (`"accept"` or `"reject"`).
    pub fn as_str(&self) -> &str {
        match self {
            InvitationAction::Accept => "accept",
            InvitationAction::Reject => "reject",
        }
    }
}

/// Detailed information about a received connection invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedInvitation {
    /// Pure invitation ID (e.g., `"7442834960828465152"`), used to respond to the invitation.
    pub entity_urn: InvitationId,

    /// Sender profile identifier / URN.
    pub from_member_urn: ProfileId,

    /// First / given name of the sender.
    pub first_name: String,
    /// Last / family name of the sender.
    pub last_name: String,
    /// Headline or occupation of the sender.
    pub occupation: Option<String>,
    /// Whether the invitation is marked as unseen in the LinkedIn notification center.
    pub unseen: bool,
    /// Classification of the invitation (e.g., `"CONNECTION"`).
    pub invitation_type: String,
    /// Whether the sender included a custom personal note.
    pub custom_message: bool,
    /// Security token required to accept or reject the invitation.
    pub shared_secret: String,
    /// Milliseconds since Unix epoch when the invitation was dispatched.
    pub sent_time: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawInvitationResponse<'a> {
    #[serde(borrow)]
    pub elements: Vec<ElementWrapper<'a>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ElementWrapper<'a> {
    // 1. Serde first attempts to parse this strict structure.
    // If "fromMember" or "firstName" is missing, this branch fails immediately.
    // 2. On failure, Serde falls through here.
    // IgnoredAny "swallows" the rest of this element's JSON WITHOUT allocating memory.
    #[serde(borrow)]
    Valid(StrictElement<'a>),
    Ignored(serde::de::IgnoredAny),
}

#[derive(Debug, Deserialize)]
pub(crate) struct StrictElement<'a> {
    #[serde(borrow)]
    pub invitation: StrictInvitation<'a>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StrictInvitation<'a> {
    // Use default for fields that could be missing even on a valid invitation
    pub entity_urn: InvitationId,

    #[serde(borrow, default)]
    pub invitation_type: Cow<'a, str>,
    #[serde(default)]
    pub custom_message: bool,
    #[serde(borrow, default)]
    pub shared_secret: Cow<'a, str>,
    #[serde(default)]
    pub unseen: bool,
    #[serde(default)]
    pub sent_time: u64,

    // The fromMember field is REQUIRED. If missing, StrictElement parsing fails.
    #[serde(borrow, rename = "fromMember")]
    pub from_member: StrictFromMember<'a>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StrictFromMember<'a> {
    // firstName and lastName are REQUIRED.
    #[serde(borrow)]
    pub first_name: Cow<'a, str>,
    #[serde(borrow)]
    pub last_name: Cow<'a, str>,
    #[serde(borrow)]
    pub occupation: Option<Cow<'a, str>>,
    pub entity_urn: ProfileId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddConnectionPayload<'a> {
    pub invitee: InviteeWrapper<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_message: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct InviteeWrapper<'a> {
    #[serde(rename = "inviteeUnion")]
    pub invitee_union: MemberProfileWrapper<'a>,
}

#[derive(Serialize)]
pub(crate) struct MemberProfileWrapper<'a> {
    #[serde(rename = "memberProfile")]
    pub member_profile: FsdProfileUrn<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplyPayload<'a> {
    pub invitation_id: &'a InvitationId,
    pub invitation_shared_secret: &'a str,
    pub is_generic_invitation: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InvitationQueryParams {
    pub start: u32,
    pub count: u32,
    pub include_insights: &'static str,
    pub q: &'static str,
}
