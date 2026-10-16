use crate::errors::LinkedInError;
use log::debug;
use wreq::header::{ACCEPT, ACCEPT_LANGUAGE, CONNECTION, HeaderMap, HeaderValue};

/// Creates HeaderMap with static headers and dynamic CSRF token.
pub(crate) fn create_headers(csrf_token: &str) -> Result<HeaderMap, LinkedInError> {
    let mut headers_map = HeaderMap::with_capacity(4);

    headers_map.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers_map.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.5"));
    headers_map.insert(CONNECTION, HeaderValue::from_static("keep-alive"));

    inject_csrf_token_into_headers(&mut headers_map, csrf_token)?;

    Ok(headers_map)
}

/// Utility function to inject dynamic CSRF token.
pub(crate) fn inject_csrf_token_into_headers(
    headers_map: &mut HeaderMap,
    csrf_token: &str,
) -> Result<(), LinkedInError> {
    debug!("Injecting dynamic csrf-token into headers.");

    // try_from is used because token comes from an external source (cookies)
    // and must be validated to ensure it contains no illegal characters.
    let val = HeaderValue::try_from(csrf_token)?;
    headers_map.insert("csrf-token", val);

    debug!("csrf-token successfully injected.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_headers_contains_required_fields() {
        let token = "ajax:123456";
        let headers = create_headers(token).unwrap();

        assert_eq!(headers.get(ACCEPT).unwrap(), "application/json");
        assert_eq!(headers.get(ACCEPT_LANGUAGE).unwrap(), "en-US,en;q=0.5");
        assert_eq!(headers.get(CONNECTION).unwrap(), "keep-alive");
        assert_eq!(headers.get("csrf-token").unwrap(), token);
    }
}
