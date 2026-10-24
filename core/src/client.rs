use crate::api::{Browser, ClientConfig};
use crate::cookies;
use crate::errors::LinkedInError;
use crate::headers::create_headers;
use log::{debug, info};
use std::env::consts::OS as HOST_OS;
use std::sync::Arc;
use wreq::{Client, EmulationFactory, header::USER_AGENT};
use wreq_util::{Emulation, EmulationOS, EmulationOption};

impl From<Browser> for Emulation {
    fn from(browser: Browser) -> Self {
        match browser {
            Browser::Firefox => Emulation::Firefox147,
            Browser::Chrome | Browser::Chromium | Browser::Brave => Emulation::Chrome145,
            Browser::Edge => Emulation::Edge145,
            Browser::Safari => Emulation::Safari26_2,
        }
    }
}

#[inline]
fn detect_emulation_os() -> EmulationOS {
    if cfg!(target_os = "linux") {
        EmulationOS::Linux
    } else if cfg!(target_os = "macos") {
        EmulationOS::MacOS
    } else {
        EmulationOS::Windows
    }
}

/// Builds emulation configuration dynamically
fn get_emulation_options(browser: Browser) -> Result<wreq::Emulation, LinkedInError> {
    debug!(
        "Detected host OS: {}; requested browser: {}",
        HOST_OS, browser
    );

    let os_emulation = detect_emulation_os();
    let browser_emulation = Emulation::from(browser);

    let mut emulation = EmulationOption::builder()
        .emulation(browser_emulation)
        .emulation_os(os_emulation)
        .build()
        .emulation();

    update_user_agent(browser, &mut emulation)?;

    Ok(emulation)
}

// Stable desktop versions checked on 2026-10-23; this is a fixed snapshot,
// not an installed-browser detector or a runtime update service.
// Firefox: https://product-details.mozilla.org/1.0/firefox_versions.json
// Chrome/Chromium: https://chromereleases.googleblog.com/2026/10/stable-channel-update-for-desktop_02018666648.html
// Chrome 157 is only Early Stable; use the generally released Chrome 156.
// Brave's UA uses its Chromium version, not its product version (1.98.60):
// https://github.com/brave/brave-browser/releases/tag/v1.98.60
// Edge: https://learn.microsoft.com/en-us/deployedge/microsoft-edge-relnote-stable-channel
// Safari 27.0.1: https://bugs.webkit.org/show_bug.cgi?id=276931#c12
fn update_user_agent(
    browser: Browser,
    emulation: &mut wreq::Emulation,
) -> Result<(), LinkedInError> {
    let versions: &[(&str, &str)] = match browser {
        // Firefox's UA exposes major.0, including on patch releases such as 158.0.1.
        Browser::Firefox => &[("rv:", "158.0"), ("Firefox/", "158.0")],
        Browser::Chrome | Browser::Chromium | Browser::Brave => &[("Chrome/", "156.0.0.0")],
        Browser::Edge => &[("Chrome/", "156.0.0.0"), ("Edg/", "156.0.0.0")],
        // Safari/ and AppleWebKit/ identify the engine; only Version/ is changed.
        Browser::Safari => &[("Version/", "27.0.1")],
    };
    let headers = emulation.headers_mut();
    let mut user_agent = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| LinkedInError::Api(format!("Missing {browser} emulation User-Agent")))?
        .to_owned();
    for &(token, version) in versions {
        let start = user_agent
            .find(token)
            .map(|index| index + token.len())
            .ok_or_else(|| {
                LinkedInError::Api(format!("Missing {token} token in {browser} User-Agent"))
            })?;
        let length = user_agent[start..]
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || *byte == b'.')
            .count();
        if length == 0 {
            return Err(LinkedInError::Api(format!(
                "Missing {token} version in {browser} User-Agent"
            )));
        }
        user_agent.replace_range(start..start + length, version);
    }
    headers.insert(USER_AGENT, user_agent.parse()?);
    Ok(())
}

pub(crate) fn new_client(config: &ClientConfig) -> Result<wreq::Client, LinkedInError> {
    info!(
        "Initializing new LinkedIn client from browser profile: '{}'",
        config.browser
    );

    debug!("Loading cookies for browser '{}'...", config.browser);

    let (jar, csrf_token) = cookies::new(config.browser, &config.cookie_source)?;

    debug!("Generating HTTP headers...");

    let headers = create_headers(&csrf_token)?;

    let emulation_opts = get_emulation_options(config.browser)?;

    debug!("HTTP headers generated.");

    let client = Client::builder()
        .emulation(emulation_opts)
        .default_headers(headers)
        .cookie_provider(Arc::new(jar))
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    info!("LinkedIn client created successfully.");

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_only_browser_version_tokens_in_real_desktop_profiles() {
        for os in [EmulationOS::Linux, EmulationOS::MacOS, EmulationOS::Windows] {
            for browser in [
                Browser::Firefox,
                Browser::Chrome,
                Browser::Chromium,
                Browser::Brave,
                Browser::Edge,
                Browser::Safari,
            ] {
                let mut profile = EmulationOption::builder()
                    .emulation(Emulation::from(browser))
                    .emulation_os(os)
                    .build()
                    .emulation();
                let original_headers = profile.headers_mut().clone();
                let original_ua = original_headers[USER_AGENT].to_str().unwrap();
                let expected = match browser {
                    Browser::Firefox => original_ua
                        .replace("rv:147.0", "rv:158.0")
                        .replace("Firefox/147.0", "Firefox/158.0"),
                    Browser::Chrome | Browser::Chromium | Browser::Brave => {
                        original_ua.replace("Chrome/145.0.0.0", "Chrome/156.0.0.0")
                    }
                    Browser::Edge => original_ua
                        .replace("Chrome/145.0.0.0", "Chrome/156.0.0.0")
                        .replace("Edg/145.0.0.0", "Edg/156.0.0.0"),
                    Browser::Safari => original_ua.replace("Version/26.2", "Version/27.0.1"),
                };
                assert_ne!(
                    expected, original_ua,
                    "unchanged source profile for {browser}"
                );
                update_user_agent(browser, &mut profile).unwrap();
                let mut expected_headers = original_headers;
                expected_headers.insert(USER_AGENT, expected.parse().unwrap());
                assert_eq!(*profile.headers_mut(), expected_headers, "{browser}");
                // Updating a profile twice must not change it further.
                update_user_agent(browser, &mut profile).unwrap();
                assert_eq!(*profile.headers_mut(), expected_headers, "{browser}");
            }
        }
    }

    #[test]
    fn rejects_missing_or_unexpected_user_agent_formats() {
        let mut profile = wreq::Emulation::default();
        assert!(update_user_agent(Browser::Firefox, &mut profile).is_err());
        for ua in ["CustomClient/1.0", "Firefox/158.0 rv:unknown"] {
            profile
                .headers_mut()
                .insert(USER_AGENT, ua.parse().unwrap());
            assert!(update_user_agent(Browser::Firefox, &mut profile).is_err());
        }
    }
}
