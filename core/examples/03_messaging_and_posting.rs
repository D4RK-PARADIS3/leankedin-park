//! Example 3: Content Creation and Messaging
//! This workflow outlines how to publish a post to your feed
//! and send a direct message to a connection.

use leankedin_park::models::identifiers::ProfileId;
use leankedin_park::models::posts::{AllowedCommentersScope, NewPost, VisibilityType};
use leankedin_park::{Browser, ClientConfig, Linkedin};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let config = ClientConfig::new(Browser::Edge).with_evasion_delay_in_seconds(1.5..=3.0)?;
    let linkedin = Linkedin::with_config(config)?;

    // 1. Publish a new post
    println!("Drafting a new post...");
    let post = NewPost::builder()
        .text("Hello LinkedIn! I'm automating my workflow using Rust! 🦀🚀".to_string())
        .visibility(VisibilityType::ConnectionsOnly)
        .comments_scope(AllowedCommentersScope::ConnectionsOnly)
        .build();

    println!("Publishing post...");
    match linkedin.posts().publish_post(&post).await {
        Ok(_) => println!("Post published successfully!"),
        Err(e) => println!("Failed to publish post: {}", e), // E.g., if post is empty
    }

    // 2. Send a direct message to a specific connection
    // Ensure you are connected to the target URN to avoid API rejections.
    // Replace this placeholder with the intended recipient's profile URN.
    let target_urn = "ACoAA...".parse::<ProfileId>()?;
    let message_text = "Hello! This is an example message sent with leankedin-park.";

    println!("Sending message to URN: {}...", target_urn.as_str());
    match linkedin
        .messaging()
        .send_message(message_text, &target_urn)
        .await
    {
        Ok(_) => println!("Message sent successfully!"),
        Err(e) => println!("Failed to send message: {}", e),
    }

    Ok(())
}
