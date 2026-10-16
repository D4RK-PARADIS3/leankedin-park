//! Strongly typed, validated URNs and identifiers for LinkedIn entities.
//!
//! Provides [`VanityName`], [`ProfileId`], [`InvitationId`], [`ConversationId`],
//! and the conversion trait [`AsVanityStr`].

use crate::errors::{LinkedInError, UrnError, VanityNameError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::str::FromStr;

/// Helper trait enabling ergonomic zero-allocation passing of vanity profile names.
///
/// Implemented for [`VanityName`], `&str`, `String`, and references.
pub trait AsVanityStr {
    /// Validates and returns the vanity name as a string slice without heap allocation.
    ///
    /// # Errors
    ///
    /// Returns [`LinkedInError::VanityName`] if the vanity string is empty or contains invalid characters.
    fn as_vanity_str(&self) -> Result<&str, LinkedInError>;
}

impl AsVanityStr for VanityName {
    #[inline]
    fn as_vanity_str(&self) -> Result<&str, LinkedInError> {
        Ok(self.as_str())
    }
}

impl AsVanityStr for &VanityName {
    #[inline]
    fn as_vanity_str(&self) -> Result<&str, LinkedInError> {
        Ok(self.as_str())
    }
}

impl AsVanityStr for str {
    #[inline]
    fn as_vanity_str(&self) -> Result<&str, LinkedInError> {
        let cleaned = self.trim();
        VanityName::validate_str(cleaned)?;
        Ok(cleaned)
    }
}

impl AsVanityStr for &str {
    #[inline]
    fn as_vanity_str(&self) -> Result<&str, LinkedInError> {
        (*self).as_vanity_str()
    }
}

impl AsVanityStr for String {
    #[inline]
    fn as_vanity_str(&self) -> Result<&str, LinkedInError> {
        self.as_str().as_vanity_str()
    }
}

impl AsVanityStr for &String {
    #[inline]
    fn as_vanity_str(&self) -> Result<&str, LinkedInError> {
        self.as_str().as_vanity_str()
    }
}

/// Strongly typed, validated public profile vanity identifier (e.g., `"williamhgates"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct VanityName(String);

impl VanityName {
    /// Returns the vanity identifier as a string slice.
    #[inline]
    pub fn id(&self) -> &str {
        &self.0
    }

    /// Returns the vanity identifier as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn validate_str(s: &str) -> Result<(), VanityNameError> {
        if s.is_empty() {
            return Err(VanityNameError::Empty);
        }
        if !s.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(VanityNameError::InvalidCharacters(s.to_string()));
        }
        Ok(())
    }
}

impl FromStr for VanityName {
    type Err = VanityNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleaned = s.trim();
        Self::validate_str(cleaned)?;
        Ok(Self(cleaned.to_string()))
    }
}

impl TryFrom<&str> for VanityName {
    type Error = VanityNameError;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<&VanityName> for VanityName {
    #[inline]
    fn from(v: &VanityName) -> Self {
        v.clone()
    }
}

impl TryFrom<String> for VanityName {
    type Error = VanityNameError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        Self::validate_str(trimmed)?;
        if value.len() == trimmed.len() {
            Ok(Self(value))
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }
}

impl TryFrom<Cow<'_, str>> for VanityName {
    type Error = VanityNameError;
    fn try_from(value: Cow<'_, str>) -> Result<Self, Self::Error> {
        match value {
            Cow::Borrowed(value) => value.parse(),
            Cow::Owned(value) => Self::try_from(value),
        }
    }
}

impl std::borrow::Borrow<str> for VanityName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for VanityName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Cow::<'de, str>::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl std::ops::Deref for VanityName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for VanityName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Shared construction and conversions; each type supplies its parsing rules.
// `is_pure_candidate` must only return true when validation is sufficient and
// the input can be stored unchanged: no extraction or normalization is needed.
// An already normalized owned String reuses its buffer; other inputs may allocate.
macro_rules! impl_urn_id {
    (
        $(#[$meta:meta])*
        pub struct $name:ident,
        validator = $validator:path,
        extractor = $extractor:path,
        is_pure_candidate = $is_pure:path
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
        #[serde(transparent)]
        pub struct $name(pub(crate) std::string::String);

        impl $name {
            /// Creates a validated identifier from a string or Cow.
            #[inline]
            pub fn new<T>(value: T) -> Result<Self, $crate::errors::UrnError>
            where
                T: std::convert::TryInto<Self, Error = $crate::errors::UrnError>,
            {
                value.try_into()
            }

            /// Validates a pure identifier without allocating an instance.
            #[inline]
            pub fn validate(value: &str) -> Result<(), $crate::errors::UrnError> {
                $validator(value)
            }

            /// Returns the stored identifier without its URN prefix.
            #[inline]
            pub fn id(&self) -> &str {
                &self.0
            }

            /// Returns the stored identifier without its URN prefix.
            #[inline]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::errors::UrnError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if $is_pure(value) {
                    $validator(value)?;
                    return Ok(Self(value.to_owned()));
                }
                let pure_id = $extractor(value)?;
                $validator(pure_id)?;
                Ok(Self(pure_id.to_owned()))
            }
        }

        impl std::convert::TryFrom<std::string::String> for $name {
            type Error = $crate::errors::UrnError;

            fn try_from(value: std::string::String) -> Result<Self, Self::Error> {
                if $is_pure(&value) {
                    $validator(&value)?;
                    return Ok(Self(value));
                }
                let pure_id = $extractor(&value)?;
                $validator(pure_id)?;
                Ok(Self(pure_id.to_owned()))
            }
        }

        impl std::convert::TryFrom<&str> for $name {
            type Error = $crate::errors::UrnError;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl std::convert::TryFrom<&std::string::String> for $name {
            type Error = $crate::errors::UrnError;
            fn try_from(value: &std::string::String) -> Result<Self, Self::Error> {
                value.as_str().parse()
            }
        }

        impl std::convert::TryFrom<std::borrow::Cow<'_, str>> for $name {
            type Error = $crate::errors::UrnError;
            fn try_from(value: std::borrow::Cow<'_, str>) -> Result<Self, Self::Error> {
                match value {
                    std::borrow::Cow::Borrowed(value) => value.parse(),
                    std::borrow::Cow::Owned(value) => Self::try_from(value),
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = <std::borrow::Cow<'de, str> as serde::Deserialize<'de>>::deserialize(
                    deserializer,
                )?;
                Self::try_from(raw).map_err(serde::de::Error::custom)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            #[inline]
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::convert::AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::borrow::Borrow<str> for $name {
            #[inline]
            fn borrow(&self) -> &str {
                &self.0
            }
        }
    };
}

impl_urn_id! {
    /// Strongly typed, validated internal profile identifier (Profile ID / URN).
    ///
    /// Guaranteed to store only the pure ID (e.g., `"ACoAAA..."`), stripping any `"urn:li:fsd_profile:"` prefix.
    pub struct ProfileId,
    validator = validate_profile_id,
    extractor = extract_id_from_urn,
    is_pure_candidate = is_pure_simple_id_candidate
}

impl_urn_id! {
    /// Typed and strictly validated identifier for a LinkedIn invitation.
    pub struct InvitationId,
    validator = validate_invitation_id,
    extractor = extract_id_from_urn,
    is_pure_candidate = is_pure_simple_id_candidate
}

/// Network payload wrapper that serializes a [`ProfileId`] as `urn:li:fsd_profile:<id>`.
#[derive(Debug, Clone)]
pub struct FsdProfileUrn<'a>(pub &'a ProfileId);

impl<'a> Serialize for FsdProfileUrn<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("urn:li:fsd_profile:{}", self.0.id()))
    }
}

/// Network payload wrapper that serializes an activity string as `urn:li:activity:<id>`.
#[derive(Debug, Clone)]
pub struct ActivityUrn<'a>(pub &'a str);

impl<'a> Serialize for ActivityUrn<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let id = self.0.strip_prefix("urn:li:activity:").unwrap_or(self.0);
        serializer.collect_str(&format_args!("urn:li:activity:{id}"))
    }
}

/// Strictly validates a pure invitation identifier (without URN prefix).
pub(crate) fn validate_invitation_id(s: &str) -> Result<(), UrnError> {
    validate_profile_id(s)
}

/// Strictly validates a pure profile identifier (without URN prefix).
pub(crate) fn validate_profile_id(s: &str) -> Result<(), UrnError> {
    if s.is_empty() {
        return Err(UrnError::MissingId(s.to_string()));
    }
    if s.contains(':') || s.chars().any(char::is_whitespace) {
        return Err(UrnError::InvalidFormat(s.to_string()));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(UrnError::InvalidFormat(s.to_string()));
    }
    Ok(())
}

/// Central extraction and validation function (Zero-Cost).
/// Reads string, finds ID, and returns &str without allocating anything.
/// Internal function to clean and extract strict ID
fn extract_id_from_urn(s: &str) -> Result<&str, UrnError> {
    let extracted = s.split(':').next_back().unwrap_or(s).trim();
    if extracted.is_empty() {
        return Err(UrnError::MissingId(s.to_string()));
    }
    Ok(extracted)
}

/// Strictly validates a pure LinkedIn conversation identifier.
///
/// A valid identifier is a non-empty token, without URN separator (:),
/// without comma, without residual parentheses, without whitespace, made of
/// safe Base64 / alphanumeric characters (`[a-zA-Z0-9_-=+/]`).
pub(crate) fn validate_conversation_id(s: &str) -> Result<(), UrnError> {
    if s.is_empty() {
        return Err(UrnError::MissingId(s.to_string()));
    }
    if s.contains(':')
        || s.contains(',')
        || s.contains('(')
        || s.contains(')')
        || s.chars().any(char::is_whitespace)
    {
        return Err(UrnError::InvalidFormat(s.to_string()));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '=' | '+' | '/'))
    {
        return Err(UrnError::InvalidFormat(s.to_string()));
    }
    Ok(())
}

/// Extraction helper tolerant of simple and compound messaging URNs.
pub(crate) fn extract_id_from_urn_for_conversations(s: &str) -> Result<&str, UrnError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(UrnError::MissingId(s.to_string()));
    }

    // Handles LinkedIn compound wrappers:
    // "urn:li:msg_conversation:(urn:li:fsd_profile:XXX,2-YYY)"
    let token = trimmed
        .trim_matches(|c| c == '(' || c == ')')
        .rsplit(',')
        .next()
        .unwrap_or(trimmed)
        .trim();

    let extracted = token
        .split(':')
        .next_back()
        .unwrap_or(token)
        .trim()
        .trim_matches(|c| c == '(' || c == ')');

    if extracted.is_empty() {
        return Err(UrnError::MissingId(s.to_string()));
    }
    Ok(extracted)
}

#[inline]
fn is_pure_simple_id_candidate(s: &str) -> bool {
    !s.contains(':') && s.len() == s.trim().len()
}

#[inline]
fn is_pure_id_candidate(s: &str) -> bool {
    !s.contains(':')
        && !s.contains(',')
        && !s.contains('(')
        && !s.contains(')')
        && s.len() == s.trim().len()
        && !s.is_empty()
}

impl_urn_id! {
    /// Typed and strictly validated identifier for a LinkedIn conversation.
    pub struct ConversationId,
    validator = validate_conversation_id,
    extractor = extract_id_from_urn_for_conversations,
    is_pure_candidate = is_pure_id_candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW_CONV_ID: &str = "2-xK9pL2mQ0vR8tY1wZ4bN7cD3fG6hJ5sA8eU1iO4kL7mP0rT3vW6yZ9==";

    #[test]
    fn test_conversation_id_new_and_parse_consistency() {
        let from_new = ConversationId::new(RAW_CONV_ID).unwrap();
        let from_parse = RAW_CONV_ID.parse::<ConversationId>().unwrap();
        let from_try_str = ConversationId::try_from(RAW_CONV_ID).unwrap();
        let from_try_string = ConversationId::try_from(RAW_CONV_ID.to_string()).unwrap();

        assert_eq!(from_new, from_parse);
        assert_eq!(from_new, from_try_str);
        assert_eq!(from_new, from_try_string);
        assert_eq!(from_new.as_str(), RAW_CONV_ID);
    }

    #[test]
    fn test_conversation_id_from_backend_urn() {
        let urn = format!("urn:li:messagingThread:{RAW_CONV_ID}");
        let parsed = ConversationId::new(&urn).unwrap();
        assert_eq!(parsed.as_str(), RAW_CONV_ID);
    }

    #[test]
    fn test_conversation_id_from_compound_entity_urn() {
        let compound = format!(
            "urn:li:msg_conversation:(urn:li:fsd_profile:ACoAA9pL2vR8tY1wZ4bN7cD3fG6hJ-5sA8eU1iO,{RAW_CONV_ID})"
        );
        let parsed = ConversationId::new(compound).unwrap();
        assert_eq!(parsed.as_str(), RAW_CONV_ID);
    }

    #[test]
    fn test_conversation_id_rejects_invalid() {
        assert!(matches!(
            ConversationId::new("bad conversation id"),
            Err(UrnError::InvalidFormat(_))
        ));

        assert!(matches!(
            ConversationId::new(""),
            Err(UrnError::MissingId(_))
        ));

        assert!(matches!(
            ConversationId::new("2-ABC#123!"),
            Err(UrnError::InvalidFormat(_))
        ));
    }

    #[test]
    fn test_conversation_id_serde_roundtrip() {
        let json = format!("\"urn:li:messagingThread:{RAW_CONV_ID}\"");
        let conv: ConversationId = serde_json::from_str(&json).unwrap();
        assert_eq!(conv.as_str(), RAW_CONV_ID);

        let serialized = serde_json::to_string(&conv).unwrap();
        assert_eq!(serialized, format!("\"{RAW_CONV_ID}\""));
    }

    #[test]
    fn test_invitation_id_new_and_parse_match() {
        let raw = "7442834960828465152";
        let from_new = InvitationId::new(raw).unwrap();
        let from_parse = raw.parse::<InvitationId>().unwrap();
        let from_try_str = InvitationId::try_from(raw).unwrap();
        let from_try_string = InvitationId::try_from(raw.to_string()).unwrap();

        assert_eq!(from_new, from_parse);
        assert_eq!(from_new, from_try_str);
        assert_eq!(from_new, from_try_string);
        assert_eq!(from_new.as_str(), raw);
    }

    #[test]
    fn test_invitation_id_from_full_urn() {
        let urn = "urn:li:fsd_invitation:7442834960828465152";
        let parsed = InvitationId::new(urn).unwrap();
        assert_eq!(parsed.as_str(), "7442834960828465152");
    }

    #[test]
    fn test_invitation_id_fast_path_rejects_invalid() {
        let res = InvitationId::try_from("bad invitation id".to_string());
        assert!(matches!(res, Err(UrnError::InvalidFormat(_))));

        let res_empty = InvitationId::try_from("".to_string());
        assert!(matches!(res_empty, Err(UrnError::MissingId(_))));

        let res_chars = InvitationId::new("invitation#123");
        assert!(matches!(res_chars, Err(UrnError::InvalidFormat(_))));
    }

    #[test]
    fn test_invitation_id_serde_roundtrip() {
        let raw_json = "\"urn:li:fsd_invitation:7442834960828465152\"";
        let id: InvitationId = serde_json::from_str(raw_json).unwrap();
        assert_eq!(id.as_str(), "7442834960828465152");

        let serialized = serde_json::to_string(&id).unwrap();
        assert_eq!(serialized, "\"7442834960828465152\"");
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn vanity_conversions_agree_on_normalization_and_errors() {
        for input in [
            "bill",
            " bill ",
            "\tÉlodie-123\n",
            "",
            "   ",
            "invalid/name",
            "a_b",
        ] {
            let parsed = input.parse::<VanityName>();
            let owned = VanityName::try_from(input.to_owned());
            let borrowed = VanityName::try_from(Cow::Borrowed(input));
            let cow_owned = VanityName::try_from(Cow::Owned(input.to_owned()));
            let json = serde_json::to_string(input).unwrap();
            let decoded = serde_json::from_str::<VanityName>(&json);
            for result in [&owned, &borrowed, &cow_owned] {
                assert_eq!(
                    parsed
                        .as_ref()
                        .map(VanityName::as_str)
                        .map_err(ToString::to_string),
                    result
                        .as_ref()
                        .map(VanityName::as_str)
                        .map_err(ToString::to_string)
                );
            }
            assert_eq!(parsed.is_ok(), decoded.is_ok());
            if let Ok(parsed) = parsed {
                assert_eq!(parsed, decoded.unwrap());
            }
        }
    }

    #[test]
    fn owned_clean_identifiers_reuse_their_buffers() {
        macro_rules! check {
            ($name:ty, $input:expr) => {{
                let input = $input.to_owned();
                let pointer = input.as_ptr();
                let result = <$name>::try_from(input).unwrap();
                assert_eq!(result.as_str().as_ptr(), pointer);
            }};
        }
        check!(VanityName, "example-person");
        check!(ProfileId, "ACoAAA");
        check!(InvitationId, "12345");
        check!(ConversationId, "2-ABC==");
    }

    #[test]
    fn urn_conversions_keep_type_specific_parsing() {
        macro_rules! check {
            ($name:ty, $input:expr, $expected:expr) => {{
                let input = $input.to_owned();
                let parsed = input.parse::<$name>().unwrap();
                assert_eq!(parsed.id(), $expected);
                assert_eq!(parsed, <$name>::new(input.as_str()).unwrap());
                assert_eq!(parsed, <$name>::new(&input).unwrap());
                assert_eq!(parsed, <$name>::new(input.clone()).unwrap());
                assert_eq!(parsed, <$name>::new(Cow::Borrowed(input.as_str())).unwrap());
                assert_eq!(parsed, <$name>::new(Cow::Owned(input.clone())).unwrap());
                let json = serde_json::to_string(&input).unwrap();
                assert_eq!(parsed, serde_json::from_str::<$name>(&json).unwrap());
                assert_eq!(
                    serde_json::to_string(&parsed).unwrap(),
                    serde_json::to_string($expected).unwrap()
                );
                assert!(<$name>::new("bad id").is_err());
                assert!(<$name>::new(String::from("bad id")).is_err());
            }};
        }
        check!(ProfileId, " urn:li:fsd_profile:ACoAAA ", "ACoAAA");
        check!(InvitationId, "urn:li:fsd_invitation:123", "123");
        check!(
            ConversationId,
            "urn:li:msg_conversation:(urn:li:fsd_profile:XXX,2-ABC==)",
            "2-ABC=="
        );
    }

    #[test]
    fn collections_can_look_up_canonical_identifiers_without_allocating() {
        macro_rules! check {
            ($name:ty, $value:expr) => {{
                let id = $value.parse::<$name>().unwrap();
                let set = HashSet::from([id.clone()]);
                let map = HashMap::from([(id, 42)]);
                assert!(set.contains($value));
                assert_eq!(map.get($value), Some(&42));
                assert_eq!(map.get("missing"), None);
            }};
        }
        check!(VanityName, "example-person");
        check!(ProfileId, "ACoAAA");
        check!(InvitationId, "123");
        check!(ConversationId, "2-ABC==");
    }
}
