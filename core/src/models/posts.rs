//! Domain models for publishing LinkedIn posts, setting audience visibility, and reacting.

use crate::models::identifiers::ActivityUrn;
use serde::Serialize;
use typed_builder::TypedBuilder;

/* -------------------------------------------------------------------------- */
/*                    Public Structures (Builder)                             */
/* -------------------------------------------------------------------------- */

/// Builder configuration for creating and publishing a new LinkedIn post.
///
/// Constructed using [`NewPost::builder()`].
#[derive(Debug, Clone, TypedBuilder)]
pub struct NewPost {
    /// Post text content (max 3,000 characters).
    #[builder(setter(into))]
    pub(crate) text: String,

    /// Target audience visibility setting. Defaults to [`VisibilityType::Anyone`].
    #[builder(default)]
    pub(crate) visibility: VisibilityType,

    /// Commenting permission scope. Defaults to [`AllowedCommentersScope::Anyone`].
    #[builder(default)]
    pub(crate) comments_scope: AllowedCommentersScope,
}

/// Target audience visibility setting for a published post.
#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisibilityType {
    /// Publicly visible to all members on and off LinkedIn.
    #[default]
    Anyone,
    /// Visible only to 1st-degree connections.
    ConnectionsOnly,
}

/// Controls who is permitted to comment on a published post.
#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AllowedCommentersScope {
    /// Anyone who can view the post is permitted to comment.
    #[default]
    #[serde(rename = "ALL")]
    Anyone,
    /// Only 1st-degree connections may leave comments.
    ConnectionsOnly,
    /// Commenting is disabled for all users.
    #[serde(rename = "NONE")]
    NoOne,
}

/* -------------------------------------------------------------------------- */
/* Internal Structures (JSON Payload)                      */
/* -------------------------------------------------------------------------- */

/// Getter for NewPost structs
impl NewPost {
    /// Returns a reference to the post text (zero allocation).
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns configured visibility for this post (Copy type).
    #[inline]
    pub fn visibility(&self) -> VisibilityType {
        self.visibility
    }

    /// Returns allowed commenters scope (Copy type).
    #[inline]
    pub fn comments_scope(&self) -> AllowedCommentersScope {
        self.comments_scope
    }

    /// Allows modifying text in place without reallocating structure.
    #[inline]
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Consumes post and transfers ownership of String without copying in RAM.
    #[inline]
    pub fn into_text(self) -> String {
        self.text
    }
}

impl NewPost {
    /// Transforms data (by borrowing) into a Payload ready to be serialized.
    /// Does not consume self, and does not clone Strings.
    pub(crate) fn to_payload<'a>(&'a self, query_id: &'static str) -> Payload<'a> {
        const SHARE_STATE: &str = "PUBLISHED";
        const ORIGIN_FEED: &str = "FEED";

        let final_comment_scope = match (self.visibility, self.comments_scope) {
            (VisibilityType::ConnectionsOnly, AllowedCommentersScope::Anyone) => {
                AllowedCommentersScope::ConnectionsOnly
            }
            (_, scope) => scope,
        };

        let post = Post {
            allowed_commenters_scope: final_comment_scope,
            intended_share_life_cycle_state: SHARE_STATE,
            origin: ORIGIN_FEED,
            visibility_data_union: VisibilityDataUnion {
                visibility_type: self.visibility,
            },
            commentary: Commentary {
                text: &self.text,
                attributes_v2: &[],
            },
        };

        Payload {
            variables: Variables { post },
            query_id,
            include_web_metadata: true,
        }
    }
}

/// Root structure of sent JSON (Private)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Payload<'a> {
    variables: Variables<'a>,
    query_id: &'static str,
    include_web_metadata: bool,
}

#[derive(Serialize)]
struct Variables<'a> {
    post: Post<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Post<'a> {
    allowed_commenters_scope: AllowedCommentersScope,
    intended_share_life_cycle_state: &'static str,
    origin: &'static str,
    visibility_data_union: VisibilityDataUnion,
    commentary: Commentary<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VisibilityDataUnion {
    visibility_type: VisibilityType,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Commentary<'a> {
    text: &'a str,
    attributes_v2: &'a [()],
}

/* -------------------------------------------------------------------------- */
/*                      Enums for REACT_TO_POST                               */
/* -------------------------------------------------------------------------- */

/// Reaction types available for interacting with LinkedIn posts and activities.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReactionType {
    /// Standard thumbs-up Like reaction.
    Like,
    /// Clapping Celebrate / Praise reaction.
    Praise,
    /// Heart Love / Appreciation reaction.
    Appreciation,
    /// Supporting hand / Empathy reaction.
    Empathy,
    /// Lightbulb Insightful / Interest reaction.
    Interest,
    /// Laughing Funny / Entertainment reaction.
    Entertainment,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReactionEntity {
    pub reaction_type: ReactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReactionVariables<'a> {
    pub entity: ReactionEntity,
    pub thread_urn: ActivityUrn<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReactionPayload<'a> {
    pub variables: ReactionVariables<'a>,
    pub query_id: &'a str,
    pub include_web_metadata: bool,
}
