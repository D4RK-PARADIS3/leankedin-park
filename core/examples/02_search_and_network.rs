//! Example 2: Search and Network
//! This workflow demonstrates how to search for people based on keywords
//! and dynamically interact with the search results.

use leankedin_park::models::search::SearchPeopleBuilder;
use leankedin_park::{Browser, ClientConfig, Linkedin};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let config = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(2.0..=4.0)?;

    let linkedin = Linkedin::with_config(config)?;

    // Build a search query for a specific persona in a specific region
    let search_query = SearchPeopleBuilder {
        keywords: Some("Python".to_string()),
        limit: Some(10),
        include_private_profiles: true,
        current_company: vec!["1035".to_string()],
        regions: vec!["106383538".to_string()],
        ..Default::default()
    };

    println!("Searching for Rust Developers...");
    let results = linkedin.search().search_people(&search_query).await?;

    println!("Found {} results. Processing...", results.len());

    for person in &results {
        println!("- Discovered candidate: {:#?}", person);

        // In a real application, you might extract the profile URN here
        // to send an invitation or follow the user.
        // Example logic:
        /*
        if let Some(urn_str) = &person.urn {
            if let Ok(urn) = urn_str.parse::<ProfileId>() {
                println!("Following {}...", urn.0);
                match linkedin.people().follow_people(urn).await {
                    Ok(_) => println!("Successfully followed!"),
                    Err(e) => println!("Failed to follow: {}", e),
                }
            }
        }
        */
    }

    Ok(())
}
