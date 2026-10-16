//! Example 5: Reading Conversations
//! This workflow demonstrates how to fetch the authenticated user's active
//! conversations, inspect thread metadata, and display participant profiles.

use leankedin_park::{Browser, ClientConfig, Linkedin};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Logger initialization (monitor GraphQL requests and delays)
    env_logger::init();

    // 2. Client configuration with anti-detection evasion delay
    let config = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(1.5..=3.0)?;

    println!("Initializing the LinkedIn client...");
    let linkedin = Linkedin::with_config(config)?;

    // 3. Messaging API call
    println!("Fetching your inbox...");
    let conversations = linkedin.messaging().get_conversations_list().await?;

    println!(
        "Success: found {} active human conversation(s).\n",
        conversations.len()
    );

    // 4. Iteration and structured data display
    for (index, conv) in conversations.iter().enumerate() {
        println!("[{:>2}] Conversation URN: {}", index + 1, conv.urn.as_str());
        println!(
            "     Status        : Unread: {} | Read: {} | Group: {}",
            conv.unread_count, conv.is_read, conv.is_group
        );

        // Inspection of categories (PrimaryInbox, Inmail, etc.)
        let categories = conv
            .categories
            .iter()
            .map(|c| format!("{c:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("     Folders       : [{categories}]");

        // Participant details
        println!("     Participants  ({}):", conv.participants.len());
        for p in &conv.participants {
            let name = match (&p.first_name, &p.last_name) {
                (Some(f), Some(l)) => format!("{f} {l}"),
                (Some(f), None) => f.clone(),
                (None, Some(l)) => l.clone(),
                (None, None) => "Private / Unknown Profile".to_string(),
            };

            let distance = p
                .distance
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "Unknown distance".to_string());

            println!("       • {name} (Relationship: {distance})");
            println!("         - URN    : {}", p.urn.as_str());

            if let Some(ref url) = p.profile_url {
                println!("         - Profile: {url}");
            }
        }
        println!("{}", "-".repeat(70));
    }

    Ok(())
}
