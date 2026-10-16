use crate::errors::UrnError;
use crate::models::identifiers::ProfileId;
use rand::Rng;
use serde::{Deserialize, Deserializer};
use std::borrow::Cow;
use uuid::Uuid;

use crate::LinkedInError;

pub(crate) fn latin1_to_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(b as char);
    }
    s
}

pub(crate) fn generate_linkedin_tracking_id() -> String {
    let mut rng = rand::rng();
    let mut random_bytes = [0u8; 16];
    rng.fill(&mut random_bytes);

    latin1_to_string(&random_bytes)
}

pub(crate) fn generate_v4_uuid() -> Uuid {
    Uuid::new_v4()
}

pub(crate) fn extract_id_from_people_search(raw_text: &str) -> Result<ProfileId, LinkedInError> {
    let extracted_str = raw_text
        .split("fsd_profileActions:(")
        .nth(1)
        .and_then(|part| part.split(',').next())
        .ok_or_else(|| {
            UrnError::FailToExtractUrn(format!("'fsd_profileActions:(' not found in : {raw_text}"))
        })?;

    Ok(extracted_str.parse::<ProfileId>()?)
}

/// Magic deserializer to directly extract the "text" value from a Voyager object
/// Example of intercepted JSON: {"text": "Rust Developer"} -> Directly produces "Rust Developer"
pub(crate) fn deserialize_text<'de, D>(deserializer: D) -> Result<Cow<'de, str>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct TextWrapper<'a> {
        #[serde(borrow)]
        text: Cow<'a, str>,
    }
    TextWrapper::deserialize(deserializer).map(|w| w.text)
}
/// Variant for optional fields (if the complete object can be missing or null)
pub(crate) fn deserialize_optional_text<'de, D>(
    deserializer: D,
) -> Result<Option<Cow<'de, str>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct TextWrapper<'a> {
        #[serde(borrow, default)]
        // Add default in case "LinkedIn sends an accessibility node without a text field"
        text: Option<Cow<'a, str>>,
    }
    let opt = Option::<TextWrapper>::deserialize(deserializer)?;
    Ok(opt.and_then(|w| w.text))
}
