use crate::api::{Browser, CookieSource};
use crate::errors::{ClientConfigError, CookieError, LinkedInError};
use log::{debug, info, trace};
use rookie::enums::Cookie;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use wreq::header::HeaderValue;
use wreq::{
    Uri,
    cookie::{CookieStore, Jar},
};

#[cfg(target_os = "macos")]
use rookie::safari;
use rookie::{brave, chrome, chromium, edge, firefox};

const LINKEDIN_DOMAIN: &str = "linkedin.com";
const LINKEDIN_DOMAIN_FOR_MANDATORY_COOKIES: &str = ".www.linkedin.com"; // "li_at" and "JSESSIONID" cookies, this domain is only used for manually loaded cookies
const COOKIE_LI_AT: &str = "li_at";
const COOKIE_JSESSIONID: &str = "JSESSIONID";

const ESSENTIAL_COOKIES: &[&str] = &[COOKIE_LI_AT, COOKIE_JSESSIONID];

pub(crate) fn new(browser: Browser, source: &CookieSource) -> Result<(Jar, String), LinkedInError> {
    let cookies = match source {
        CookieSource::Auto => {
            info!("Trying to import cookies from browser profile: '{browser}'...");
            get_linkedin_cookies_from_browser(browser)?
        }
        CookieSource::Manual { li_at, jsessionid } => {
            info!("Using manually provided li_at and JSESSIONID cookies.");
            vec![
                Cookie {
                    domain: LINKEDIN_DOMAIN_FOR_MANDATORY_COOKIES.to_string(),
                    path: "/".to_string(),
                    secure: true,
                    expires: None,
                    name: COOKIE_LI_AT.to_string(),
                    value: li_at.clone(),
                    http_only: true,
                    same_site: 0,
                },
                Cookie {
                    domain: LINKEDIN_DOMAIN_FOR_MANDATORY_COOKIES.to_string(),
                    path: "/".to_string(),
                    secure: true,
                    expires: None,
                    name: COOKIE_JSESSIONID.to_string(),
                    value: jsessionid.clone(),
                    http_only: true,
                    same_site: 0,
                },
            ]
        }
        CookieSource::NetscapeFile(path) => {
            info!("Loading cookies from Netscape file: {:?}", path);
            let content = fs::read_to_string(path)?;
            parse_netscape_cookies(&content)
        }
    };

    debug!(
        "Found {} raw cookies matching '{LINKEDIN_DOMAIN_FOR_MANDATORY_COOKIES}'",
        cookies.len()
    );

    if cookies.is_empty() {
        return Err(CookieError::ZeroCookieFound.into());
    }

    validate_essential_cookies(&cookies)?;

    let csrf_token: &str =
        get_csrf_token_from_cookies_vec(&cookies).ok_or(CookieError::CantCreateCsrfToken)?;

    let jar = import_cookie_jar(&cookies)?;

    info!("Successfully loaded the cookies");

    Ok((jar, csrf_token.to_string()))
}

// Keep each cookie's original scope instead of flattening all cookies onto
// www.linkedin.com. In particular, .linkedin.com cookies also apply to uploads
// on another LinkedIn subdomain, while /voyager cookies stay on that path.
fn import_cookie_jar(cookies: &[Cookie]) -> Result<Jar, LinkedInError> {
    let jar = Jar::default();
    for cookie in cookies {
        trace!(
            "Processing cookie: '{}' (Domain: {}, Path: {})",
            cookie.name, cookie.domain, cookie.path
        );
        let host = cookie.domain.trim_start_matches('.');
        let origin = format!("https://{host}/")
            .parse::<Uri>()
            .map_err(|_| LinkedInError::Api("Invalid imported cookie domain".into()))?;
        let path = if cookie.path.is_empty() {
            "/"
        } else {
            &cookie.path
        };
        let mut cookie_str = format!("{}={}; Path={}", cookie.name, cookie.value, path);
        if cookie.domain.starts_with('.') {
            cookie_str.push_str("; Domain=");
            cookie_str.push_str(&cookie.domain);
        }
        if cookie.secure {
            cookie_str.push_str("; Secure");
        }
        if cookie.http_only {
            cookie_str.push_str("; HttpOnly");
        }
        let header = HeaderValue::from_str(&cookie_str)?;
        jar.set_cookies(&mut std::iter::once(&header), &origin);
    }
    Ok(jar)
}

pub(crate) fn get_linkedin_cookies_from_browser(
    browser: Browser,
) -> Result<Vec<Cookie>, LinkedInError> {
    let domains = Some(vec![LINKEDIN_DOMAIN.to_string()]);

    debug!("Querying system keychain/database for browser: '{browser}'");

    let result = match browser {
        Browser::Brave => brave(domains),
        Browser::Chrome => chrome(domains),
        Browser::Chromium => chromium(domains),
        Browser::Edge => edge(domains),
        Browser::Firefox => firefox(domains),
        #[cfg(target_os = "macos")]
        Browser::Safari => safari(domains),
        #[cfg(not(target_os = "macos"))]
        Browser::Safari => return Err(ClientConfigError::SafariCanOnlyBeUsedOnMacOS.into()),
    }
    .map_err(|e| ClientConfigError::RookieCrate(e.to_string()))?;

    Ok(result)
}

pub(crate) fn validate_essential_cookies(cookies: &[Cookie]) -> Result<(), LinkedInError> {
    let missing_cookies: Vec<&'static str> = ESSENTIAL_COOKIES
        .iter()
        .copied()
        .filter(|&req_cookie| !cookies.has_cookie(req_cookie))
        .collect();

    if missing_cookies.len() == ESSENTIAL_COOKIES.len() {
        return Err(CookieError::MissingAllEssential(ESSENTIAL_COOKIES).into());
    } else if !missing_cookies.is_empty() {
        return Err(CookieError::MissingSomeEssential {
            cookies: missing_cookies,
        }
        .into());
    }

    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(li_at_cookie) = cookies.iter().find(|c| c.name == COOKIE_LI_AT)
        && let Some(expiration_date) = li_at_cookie.expires
        && expiration_date > 0
        && expiration_date < current_timestamp
    {
        return Err(CookieError::Expired(COOKIE_LI_AT.to_string()).into());
    }

    Ok(())
}

pub(crate) trait CookieSliceExt {
    /// Checks if a cookie with this exact name exists in the list
    fn has_cookie(&self, name: &str) -> bool;
}

impl CookieSliceExt for [Cookie] {
    fn has_cookie(&self, name: &str) -> bool {
        self.iter().any(|c| c.name == name)
    }
}

pub(crate) fn get_csrf_token_from_cookies_vec(cookies: &[Cookie]) -> Option<&str> {
    cookies
        .iter()
        .find(|c| c.name == COOKIE_JSESSIONID)
        .map(|c| c.value.trim_matches('"'))
}

/// Helper: Parses a Netscape cookie file and returns rookie::Cookie.
/// Zero dependencies, fast, and handles format subtleties (like the #HttpOnly_ flag).
fn parse_netscape_cookies(content: &str) -> Vec<Cookie> {
    content
        .lines()
        .map(str::trim)
        // Ignore empty lines and normal comments
        // IMPORTANT: Keep lines starting with "#HttpOnly_" because they are valid cookies!
        .filter(|line| {
            !line.is_empty() && (!line.starts_with('#') || line.starts_with("#HttpOnly_"))
        })
        .filter_map(|line| {
            let is_http_only = line.starts_with("#HttpOnly_");
            let clean_line = if is_http_only {
                line.strip_prefix("#HttpOnly_").unwrap_or(line).trim_start()
            } else {
                line
            };

            // Netscape format separates 7 columns with tabs
            let mut cols = clean_line.split('\t');

            let domain_str = cols.next()?;
            if !domain_str.contains("linkedin.com") {
                return None;
            }
            let domain = domain_str.to_string();

            let _ = cols.next()?; // Flag "include_subdomains" (unused by rookie)
            let path = cols.next()?.to_string();
            let secure = cols.next()?.eq_ignore_ascii_case("true");
            let expires_str = cols.next()?;
            // Try parsing as f64 (accepts "123" AND "123.45"),
            // then convert to u64 (which truncates decimal part)
            let expires = expires_str
                .parse::<f64>()
                .ok()
                .filter(|&v| v > 0.0)
                .map(|v| v as u64);
            let name = cols.next()?.to_string();
            let value = cols.next()?.to_string();

            Some(Cookie {
                domain,
                path,
                secure,
                expires,
                name,
                value,
                http_only: is_http_only,
                same_site: 0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rookie::enums::Cookie;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Generates a Unix timestamp (in seconds) relative to today.
    /// Helps avoid "test rot" (tests that fail in 3 years).
    fn get_timestamp_offset(days: i64) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let offset_secs = (days.abs() * 24 * 3600) as u64;

        if days >= 0 {
            now + offset_secs
        } else {
            now.saturating_sub(offset_secs)
        }
    }

    /// Existing helper to mock simple cookies (without parsing)
    fn mock_cookie(name: &str, value: &str) -> Cookie {
        Cookie {
            domain: LINKEDIN_DOMAIN_FOR_MANDATORY_COOKIES.to_string(),
            path: "/".to_string(),
            secure: true,
            expires: None,
            name: name.to_string(),
            value: value.to_string(),
            http_only: true,
            same_site: 0,
        }
    }

    fn cookie_header(jar: &dyn CookieStore, url: &str) -> String {
        match jar.cookies(&url.parse::<Uri>().unwrap()) {
            wreq::cookie::Cookies::Compressed(value) => value.to_str().unwrap().to_owned(),
            wreq::cookie::Cookies::Uncompressed(values) => values
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>()
                .join("; "),
            wreq::cookie::Cookies::Empty => String::new(),
            _ => panic!("Unexpected cookie representation"),
        }
    }

    #[test]
    fn imported_cookies_keep_domain_path_and_secure_scope() {
        let mut shared = mock_cookie("shared", "root");
        shared.domain = ".linkedin.com".into();
        let mut web = mock_cookie("web", "www");
        web.domain = "www.linkedin.com".into();
        let mut scoped = mock_cookie("scoped", "root");
        scoped.domain = ".linkedin.com".into();
        scoped.name = "scoped".into();
        scoped.path = "/voyager".into();
        let jar = import_cookie_jar(&[shared, web, scoped]).unwrap();
        let upload = cookie_header(&jar, "https://upload.linkedin.com/media");
        assert_eq!(upload, "shared=root");
        let messaging = cookie_header(&jar, "https://www.linkedin.com/voyager/api/me");
        for expected in ["shared=root", "web=www", "scoped=root"] {
            assert!(messaging.contains(expected));
        }
        assert!(!cookie_header(&jar, "https://www.linkedin.com/voyager-other").contains("scoped="));
        assert!(cookie_header(&jar, "https://linkedin.com.example.org/media").is_empty());
        assert!(cookie_header(&jar, "https://example.org/media").is_empty());
        assert!(cookie_header(&jar, "http://upload.linkedin.com/media").is_empty());
    }

    #[test]
    fn same_name_cookies_on_distinct_paths_are_not_overwritten() {
        let mut root = mock_cookie("session", "root");
        root.domain = ".linkedin.com".into();
        let mut scoped = mock_cookie("session", "scoped");
        scoped.domain = ".linkedin.com".into();
        scoped.path = "/voyager".into();
        scoped.value = "scoped".into();
        let jar = import_cookie_jar(&[root, scoped]).unwrap();
        assert_eq!(
            cookie_header(&jar, "https://www.linkedin.com/"),
            "session=root"
        );
        let header = cookie_header(&jar, "https://www.linkedin.com/voyager/api");
        assert!(header.contains("session=root"));
        assert!(header.contains("session=scoped"));
    }

    #[test]
    fn test_parse_netscape_and_validate_future_cookie() {
        let future_ts = get_timestamp_offset(365);

        let raw_file = format!(
            "# Netscape HTTP Cookie File\n\
            # This is a generated file!  Do not edit.\n\
            \n\
            .linkedin.com\tTRUE\t/\tTRUE\t{}\tli_at\tAQEDAR_SUPER_TOKEN\n\
            #HttpOnly_.linkedin.com\tTRUE\t/\tTRUE\t{}.454\tJSESSIONID\t\"ajax:123456\"\n",
            future_ts, future_ts
        );

        let cookies = parse_netscape_cookies(&raw_file);

        assert_eq!(cookies.len(), 2, "Should parse exactly 2 cookies");

        let li_at = &cookies[0];
        assert_eq!(li_at.name, "li_at");
        assert_eq!(li_at.domain, ".linkedin.com");
        assert_eq!(li_at.expires, Some(future_ts));

        let jsessionid = &cookies[1];
        assert_eq!(jsessionid.name, "JSESSIONID");
        assert!(jsessionid.http_only, "HttpOnly flag should be true");
        assert_eq!(
            jsessionid.domain, ".linkedin.com",
            "HttpOnly prefix should be stripped from domain"
        );
        assert_eq!(
            jsessionid.expires,
            Some(future_ts),
            "Floating point timestamp should be cleanly truncated to u64"
        );

        let validation = validate_essential_cookies(&cookies);
        assert!(
            validation.is_ok(),
            "Validation should pass because li_at is in the future"
        );
    }

    #[test]
    fn test_validate_expired_li_at_cookie() {
        // Arrange: li_at expired 2 days ago, but JSESSIONID is still valid.
        let past_ts = get_timestamp_offset(-2);
        let future_ts = get_timestamp_offset(365);

        let raw_file = format!(
            ".linkedin.com\tTRUE\t/\tTRUE\t{}\tli_at\tAQEDAR_OLD_TOKEN\n\
            .linkedin.com\tTRUE\t/\tTRUE\t{}\tJSESSIONID\t\"ajax:123456\"\n",
            past_ts, future_ts
        );

        let cookies = parse_netscape_cookies(&raw_file);

        let result = validate_essential_cookies(&cookies);

        match result {
            Err(LinkedInError::Cookie(CookieError::Expired(cookie_name))) => {
                assert_eq!(cookie_name, "li_at", "Should specify that li_at expired");
            }
            other => panic!("Expected CookieExpired error, but got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_essential_cookies_success() {
        let cookies = vec![
            mock_cookie("li_at", "secret_token"),
            mock_cookie("JSESSIONID", "\"ajax:123456789\""),
        ];

        assert!(validate_essential_cookies(&cookies).is_ok());
    }

    #[test]
    fn test_validate_essential_cookies_missing_some() {
        // Provide `li_at`, therefore missing `JSESSIONID`
        let cookies = vec![mock_cookie("li_at", "secret_token")];

        let result = validate_essential_cookies(&cookies);

        let err = result.expect_err("Expected an error due to missing JSESSIONID cookie");

        match err {
            LinkedInError::Cookie(CookieError::MissingSomeEssential { cookies: missing }) => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0], "JSESSIONID");
            }
            other_err => panic!("Expected MissingSomeEssential, but got: {:?}", other_err),
        }
    }

    #[test]
    fn test_validate_essential_cookies_missing_all() {
        let cookies = vec![mock_cookie("unrelated_cookie", "value")];

        let result = validate_essential_cookies(&cookies);
        assert!(matches!(
            result,
            Err(LinkedInError::Cookie(CookieError::MissingAllEssential(_)))
        ));
    }

    #[test]
    fn test_get_csrf_token_extraction() {
        let cookies = vec![mock_cookie("JSESSIONID", "\"ajax:98765\"")];

        let token = get_csrf_token_from_cookies_vec(&cookies);
        assert_eq!(token, Some("ajax:98765"));
    }
}
