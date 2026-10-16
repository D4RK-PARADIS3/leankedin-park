# leankedin-park

[![Crates.io](https://img.shields.io/crates/v/leankedin-park.svg)](https://crates.io/crates/leankedin-park)
[![Documentation](https://docs.rs/leankedin-park/badge.svg)](https://docs.rs/leankedin-park)
[![License: PolyForm Perimeter 1.0.1](https://img.shields.io/badge/License-PolyForm_Perimeter_1.0.1-blue.svg)](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/LICENSE.md)

A fast, ergonomic, and lightweight client for the LinkedIn Voyager API, implemented in pure Rust.

## Key Features

- **100% Pure Rust**: Zero runtime Python dependencies.
- 🍪 **Automatic Cookie Extraction**: Effortlessly extract active session cookies directly from local browsers (Firefox, Chrome, Safari, Edge, Brave, Chromium) or provide manual cookies / Netscape format cookie files.
- 🛡️ **Anti-Bot Throttling & Evasion**: Configurable random delay between requests with atomic timeline reservation and FIFO serialized transport to protect your account.
- 🎯 **Strongly Typed & Validated**: Strictly typed identifiers (URNs, Vanity names, Invitation IDs, Conversation IDs) preventing runtime request errors.
- ⚡ **Lightweight & High-Performance**: Built on `tokio` and `wreq` without running heavy headless browser instances (Playwright/Puppeteer/Selenium).
- 👥 **Comprehensive Voyager API Coverage**:
  - **Search**: Advanced people search with fine-grained filters (keywords, company, location, schools, network depth) or direct import from LinkedIn search URLs.
  - **Profiles**: Profile details, follower metrics, view counts, connection statuses.
  - **Connections & Invitations**: Inspect pending invitations, accept/reject requests, and send connection requests with personalized notes.
  - **Messaging**: Fetch inbox conversations, send 1-on-1 direct messages and group chats.
  - **Posts**: Publish new posts with custom audience visibility (`Anyone`, `ConnectionsOnly`) and react to posts.
  - **Account**: Retrieve self profile and 90-day view counts.

---

## Getting Started

> [!WARNING]
> Before getting started, keep in mind that using automation tools violates LinkedIn's Terms of Service. Because of this, we strongly recommend creating a secondary LinkedIn account to use `leankedin-park`.  
> Ask yourself: *"Is losing my LinkedIn account a big deal for me? Will I lose sleep if it vanishes overnight? Can I afford to lose the network I spent years building?"*

> [!TIP]
> If you believe in "better safe than sorry" and opt for a burner account, remember to build up its network first. If you plan to use the profile search feature, having few or no connections will leave most profiles outside your reach (see LinkedIn's connection degree system for details).

Although several web browsers work (Firefox, Chrome, Safari, Edge, Brave, Chromium), Firefox is strongly recommended for simplified access to its internal database.

1.  If Firefox is open, exit the browser completely.
    
2.  (Re)open Firefox (**do not use Private Browsing**, as the tool will fail).
    
3.  Go to LinkedIn:
    

-   **Case 1: Already signed in.** If you are already connected to the account you plan to use with `leankedin-park`, no further action is required. Just be sure to **VERIFY THE ACTIVE PROFILE** (switch to your burner account to avoid putting your real account at risk).
    
-   **Case 2: Not signed in.** Sign in using your credentials, ensuring the **"Keep me signed in"** option is checked. Close Firefox, open it again, and return to LinkedIn. If you are still logged into your account, you are ready to run `leankedin-park`.


Add `leankedin-park` and `tokio` to your `Cargo.toml`:

```toml
[dependencies]
leankedin-park = "0.1.0-alpha.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Quickstart Example

```rust
use leankedin_park::prelude::*;

#[tokio::main]
async fn main() -> Result<(), LinkedInError> {
    // Configure client with automatic browser cookie extraction and jitter delay
    let config = ClientConfig::new(Browser::Firefox)
        .with_evasion_delay_in_seconds(1.0..=3.0)?;
    
    let client = Linkedin::with_config(config)?;

    // Fetch account views
    let views = client
        .account()
        .get_current_profile_views_last_90_days()
        .await?;
    println!("Profile views: {} ({})", views.num_views, views.time_frame);

    // Fetch a public profile and display all identity, location, and network fields
    let vanity: VanityName = "williamhgates".parse()?;
    let profile = client.people().get_full_infos_of_a_profile(&vanity).await?;
    println!("Full profile:\n{profile:#?}");

    Ok(())
}
```

### Advanced People Search

```rust
use leankedin_park::prelude::*;

#[tokio::main]
async fn main() -> Result<(), LinkedInError> {
    let client = Linkedin::new(Browser::Firefox)?;

    // Search people using fluent builder
    let query = SearchPeopleBuilder::builder()
        .keywords("Rust Developer")
        .network_depths(std::collections::HashSet::from([
            ConnectionDegree::First,
            ConnectionDegree::Second,
        ]))
        .limit(10)
        .build();

    let results = client.search().search_people(&query).await?;
    for person in results {
        println!(
            "- {}: {}",
            person.full_name,
            person.primary_subtitle.as_deref().unwrap_or("")
        );
    }

    Ok(())
}
```

Search filter values are encoded according to the Rest.li URL grammar. Malformed
responses, invalid profile actions or pagination, and safety-limit interruptions
return `LinkedInError::Search`; successful lists reach the requested limit or the
server-reported end. Missing profile actions and visibility exclusions are allowed.

---

## Examples

Check out the full examples in the repository:

- [`01_fetch_profiles.rs`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/core/examples/01_fetch_profiles.rs): Profile inspection and follower metrics.
- [`02_search_and_network.rs`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/core/examples/02_search_and_network.rs): Advanced search builder and URL parsing.
- [`03_messaging_and_posting.rs`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/core/examples/03_messaging_and_posting.rs): Publishing posts and messaging.
- [`04_manage_connections.rs`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/core/examples/04_manage_connections.rs): Managing pending invitations and connection requests.
- [`05_read_conversations.rs`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/core/examples/05_read_conversations.rs): Reading messages and conversation metadata.

---

## ⚠️ Disclaimer

This project is intended for educational, research, and personal automation purposes only.
It is not affiliated with, endorsed, or certified by LinkedIn Corporation. Using automated tools may violate LinkedIn's User Agreement and may result in account restrictions. We strongly recommend testing with a secondary/burner account and using sensible evasion delays.

---

## 📄 License

Licensed under the [PolyForm Perimeter License 1.0.1](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/LICENSE.md).
