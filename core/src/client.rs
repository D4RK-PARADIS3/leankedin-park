use crate::api::{Browser, ClientConfig};
use crate::cookies;
use crate::errors::LinkedInError;
use crate::headers::create_headers;
use log::{debug, info};
use std::env::consts::OS as HOST_OS;
use std::sync::Arc;
use wreq::Client;
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
fn get_emulation_options(browser: Browser) -> EmulationOption {
    debug!(
        "Detected host OS: {}; requested browser: {}",
        HOST_OS, browser
    );

    let os_emulation = detect_emulation_os();
    let browser_emulation = Emulation::from(browser);

    EmulationOption::builder()
        .emulation(browser_emulation)
        .emulation_os(os_emulation)
        .build()
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

    let emulation_opts = get_emulation_options(config.browser);

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
