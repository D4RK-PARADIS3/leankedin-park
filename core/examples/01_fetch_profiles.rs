//! Example 1: Fetching Profiles
//! This example demonstrates how to initialize the client,
//! authenticate using default browser cookies, and fetch profile information.

use leankedin_park::models::identifiers::VanityName;
use leankedin_park::{Browser, ClientConfig, Linkedin};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger to monitor API interactions
    env_logger::init();

    // Configure the client to use Firefox cookies and set a random delay between requests.
    // The evasion delay is critical to prevent rate-limiting or getting flagged by LinkedIn.
    let config = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(2.0..=5.0)?;

    println!("Initializing LinkedIn client...");
    let linkedin = Linkedin::with_config(config)?;

    // 1. Fetch the authenticated user's profile
    let my_profile = linkedin.account().get_my_profile().await?;
    println!(
        "Successfully connected as: {} {}",
        my_profile.first_name, my_profile.last_name
    );
    println!("My URN: {}", my_profile.profile_urn);

    // 2. Fetch another user's profile using their vanity name (URL slug)
    // e.g., https://www.linkedin.com/in/williamhgates/ -> "williamhgates"
    let target_vanity = "williamhgates".parse::<VanityName>()?;

    println!("Fetching profile for {}...", target_vanity.as_str());
    let target_profile = linkedin
        .people()
        .get_infos_of_a_profile(&target_vanity)
        .await?;

    println!(
        "Found profile: {} {} ({:?})",
        target_profile.first_name, target_profile.last_name, target_profile.full_location
    );

    Ok(())
}
