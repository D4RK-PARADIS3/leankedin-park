//! Example 4: Managing Connections
//! This example shows how to list pending invitations and
//! pragmatically accept or reject them.

use leankedin_park::{Browser, ClientConfig, Linkedin, models::connections::InvitationAction};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let config = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(2.0..=5.0)?;
    let linkedin = Linkedin::with_config(config)?;

    // Fetch up to 10 pending invitations
    println!("Fetching pending connection requests...");
    let invitations = linkedin.connections().get_invitations(0, 10).await?;

    println!("You have {} pending invitations.", invitations.len());
    println!("{:?}", invitations);

    for inv in invitations {
        // Here you could filter based on 'inv' metadata
        println!("Processing invitation...");

        let action = InvitationAction::Accept; // Or InvitationAction::Reject

        match linkedin
            .connections()
            .reply_invitation(&inv.entity_urn, &inv.shared_secret, action)
            .await
        {
            Ok(_) => println!(
                "Successfully processed invitation from {} {}",
                inv.first_name, inv.last_name
            ),
            Err(e) => println!("Error processing invitation: {}", e),
        }
    }

    Ok(())
}
