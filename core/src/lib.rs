//! # leankedin-park
//!
//! A fast, ergonomic, and memory-efficient Rust library and Python binding for the LinkedIn Voyager API.
//!
//! ## Key Features
//!
//! - **Automatic Cookie Extraction**: Extracts session cookies (`li_at` and `JSESSIONID`) directly from local browser databases (Firefox, Chrome, Safari, Edge, Brave, Chromium) via the `rookie` engine, or manually via Netscape file or explicit values.
//! - **Anti-Bot Throttling & Evasion**: Configurable random delay between requests with an atomic timeline reservation and FIFO serialized transport.
//! - **Strongly Typed Identifiers**: Strongly validated URN and ID types ([`models::identifiers::ProfileId`], [`models::identifiers::VanityName`], [`models::identifiers::InvitationId`], [`models::identifiers::ConversationId`]) preventing runtime malformed request errors.
//! - **Modular Namespaced API**:
//!   - [`account`](api::Linkedin::account): Access current authenticated user profile and 90-day views.
//!   - [`people`](api::Linkedin::people): Fetch public profiles, follower counts, and follow/unfollow members.
//!   - [`connections`](api::Linkedin::connections): Inspect invitations, accept/reject invites, and send connection requests.
//!   - [`messaging`](api::Linkedin::messaging): Read conversations, send 1-on-1 and group direct messages.
//!   - [`posts`](api::Linkedin::posts): Publish shares and react to posts.
//!   - [`search`](api::Linkedin::search): Filter and search people by keywords, network distance, companies, regions, or directly from browser search URLs.
//!
//! ## Quickstart
//!
//! ```no_run
//! use leankedin_park::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), LinkedInError> {
//!     // Initialize configuration with Firefox cookies and human-like request jitter
//!     let config = ClientConfig::new(Browser::Firefox)
//!         .with_evasion_delay_in_seconds(1.0..=3.0)?;
//!
//!     let client = Linkedin::with_config(config)?;
//!
//!     // Fetch profile views over the last 90 days
//!     let views = client.account().get_current_profile_views_last_90_days().await?;
//!     println!("Profile views: {} ({})", views.num_views, views.time_frame);
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod errors;
pub mod models;

mod client;
mod cookies;
mod headers;
mod helpers;

pub use api::{Browser, ClientConfig, Linkedin};
pub use errors::LinkedInError;

pub mod prelude {
    pub use crate::api::{Browser, ClientConfig, Linkedin};
    pub use crate::errors::LinkedInError;
    pub use crate::models::connections::{ConnectionRequest, InvitationAction};
    pub use crate::models::identifiers::{
        AsVanityStr, ConversationId, InvitationId, ProfileId, VanityName,
    };
    pub use crate::models::people::ConnectionDegree;
    pub use crate::models::posts::{AllowedCommentersScope, NewPost, VisibilityType};
    pub use crate::models::search::SearchPeopleBuilder;
}
