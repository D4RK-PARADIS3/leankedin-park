# leankedin-park

A fast, ergonomic, and memory-efficient client for the LinkedIn Voyager API, implemented in Rust with first-class Python bindings.

## Key Features

-   🍪 **Automatic Cookie Extraction**: Effortlessly extract active session cookies directly from local browsers (Firefox, Chrome, Safari, Edge, Brave, Chromium) or provide manual cookies / Netscape format cookie files.
    
-   🛡️ **Anti-Bot Throttling & Evasion**: Configurable random delay between requests with atomic timeline reservation and FIFO serialized transport to keep your account safe.
    
-   🎯 **Strongly Typed & Validated**: Strictly typed identifiers (URNs, Vanity names, Invitation IDs, Conversation IDs) preventing runtime request errors.
    
-   👥 **Comprehensive Voyager API Coverage**:
    
-   **Search**: Advanced people search with fine-grained filters (keywords, company, location, schools, network depth) or directly import filters from LinkedIn search URLs.
    
-   **Profiles**: Fetch profile information, follower counts, premium status, connection statuses.
    
-   **Connections & Invitations**: Inspect pending invitations, accept/reject requests, and send connection requests with personalized notes.
    
-   **Messaging**: Fetch inbox conversations, send 1-on-1 direct messages and group chats.
    
-   **Posts**: Publish new shares with custom audience visibility (`Anyone`, `ConnectionsOnly`) and react to posts.
    
-   **Account**: Retrieve your own profile details, including your view counts.
    

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
    

---

## Project Architecture

This repository is organized as a Cargo workspace:

-   [`core/`](./core): The core Rust library. This is where the core logic lives, along with the reverse-engineered LinkedIn private API. The Rust crate works entirely as a standalone library with zero Python dependencies; the Python package is simply a wrapper around it.
    
-   [`python/`](./python): Python extension built with PyO3 and Maturin, featuring complete type stubs (`.pyi`). The Python library is powered by the Rust implementation in the `core/` directory.
    

---

## Why ?

Coming from a Python web scraping background, I wanted to learn Rust by getting my hands dirty. So I jumped into this project.

While plenty of similar tools exist, most rely on JavaScript or Python to drive headless browsers using libraries like Puppeteer, Selenium, or Playwright. These solutions are notorious resource hogs, chewing through CPU and RAM between the language runtime and the browser instance. On top of that, LinkedIn generates massive network overhead: open DevTools, check the Network tab while loading a profile, and you'll see several dozen network requests firing (APIs, ads, images, tracking).

Thanks to its Rust core, this project is dramatically lighter on CPU, memory, and network usage. Even when using the Python bindings and runtime, you avoid spinning up an entire browser instance just to make direct HTTP requests to LinkedIn's API.

---

## Python Usage

### Installation

Build and install locally using [Maturin](https://github.com/PyO3/maturin):

```bash
cd  python
maturin  develop  --release
```

### Quickstart Example

```python  
from leankedin_park import Browser, ClientConfig, Linkedin

# Initialize configuration with browser cookie extraction and human-like request jitter
config = ClientConfig(Browser.Firefox).with_evasion_delay_in_seconds(2.0, 5.0)

with Linkedin.with_config(config) as linkedin:
    # Get current user profile
    me = linkedin.get_my_profile()
    print(f"Logged in as: {me.first_name}  {me.last_name} ({me.vanity_name})")

    # Fetch 90-day profile views
    views = linkedin.get_current_profile_views_last_90_days()
    print(f"Profile views: {views.num_views} ({views.time_frame})")
    
    # Search for people
    results = linkedin.search_people(
        keywords="Rust Developer",
        limit=10,
        include_private_profiles=True,
    )

    for person in results:
        print(f"- {person.full_name}: {person.primary_subtitle}") 

```

### Advanced People Search

You can construct searches using the fluent `SearchPeople` facade or `SearchPeopleBuilder`, or parse an existing LinkedIn search URL:

```python
from leankedin_park import Linkedin, SearchPeople, NetworkDepth, SearchPeopleBuilder

# Fluent builder

search = (
    SearchPeople()
    .keywords("Rust")
    .companies(["12345", "67890"])  # Example IDs: replace with IDs from your search URL
    .network([NetworkDepth.FirstDegree, NetworkDepth.SecondDegree])
    .limit(20)
)

results = linkedin.search_people(search)

# Or directly from a copied browser search URL

url = "https://www.linkedin.com/search/results/people/?keywords=Rust&network=%5B%22F%22%5D"

builder = SearchPeopleBuilder.from_url(url)

results = linkedin.search_people(builder)

```

Company filters (`current_company` and its `companies` alias) take LinkedIn internal
company IDs as strings, not company names. Select companies in a LinkedIn people
search and copy the IDs from the URL’s `currentCompany` parameter. For example,
`currentCompany=%5B%2212345%22%2C%2267890%22%5D` encodes `["12345", "67890"]`
(illustrative IDs). You can also pass the entire search URL to
`SearchPeople.from_url(url)` or `SearchPeopleBuilder.from_url(url)`.

Check out [`python/examples/`](./python/examples) for more detailed examples:

-   `01_fetch_profiles.py`: Profile inspection and follower metrics.
    
-   `02_search_and_network.py`: Advanced search builder and URL parsing.
    
-   `03_messaging_and_posting.py`: Publishing posts and messaging.
    
-   `04_manage_connections.py`: Managing pending invitations and connection requests.
    

---

## 🦀 Rust Usage

Add `leankedin-park` to your `Cargo.toml`:

```toml
[dependencies]
leankedin-park = { path = "core" } 
tokio = { version = "1", features = ["full"] } 
```

### Quickstart Example

```rust
use leankedin_park::prelude::*;

#[tokio::main]
async fn main() -> Result<(), LinkedInError> {
    // Configure client with automatic browser cookie extraction

    let config = ClientConfig::new(Browser::Firefox).with_evasion_delay_in_seconds(1.0..=3.0)?;
    let client = Linkedin::with_config(config)?;

    // Fetch account views
    let views = client
        .account()
        .get_current_profile_views_last_90_days()
        .await?;
    println!("Profile views: {} ({})", views.num_views, views.time_frame);

    // Fetch a public profile
    let vanity = VanityName::new("williamhgates")?; // this is Bill
    let profile = client.people().get_full_infos_of_a_profile(&vanity).await?;
    println!("Found: {} {}", profile.first_name, profile.last_name);

    Ok(())
}

```

Check out [`core/examples/`](./core/examples) for more detailed examples:

-   `01_fetch_profiles.rs`: Profile inspection and follower metrics.
    
-   `02_search_and_network.rs`: Advanced search builder and URL parsing.
    
-   `03_messaging_and_posting.rs`: Publishing posts and messaging.
    
-   `04_manage_connections.rs`: Managing pending invitations and connection requests.
    
-   `05_read_conversations.rs`: Fetching conversations and participant metadata.
    

---

## Reliability and diagnostics

Requests acquire the FIFO transport permit before reserving their next start
time. A slow response therefore cannot consume the spacing of requests waiting
behind it. Cancelling a request releases its permit, including during the delay.

Search filters encode user-provided values according to the Rest.li URL grammar.
Python searches reject unknown keywords, extra positional arguments, and mixed
builder/keyword calls with `TypeError`; misspelled filters are never ignored.

Searches return a list only after reaching the requested limit or the end reported
by the server. Missing response fields, malformed profile actions, invalid paging,
and safety-limit interruptions are errors rather than silently partial results.
Rust exposes these failures through `LinkedInError::Search`; Python exposes
`SearchError`, a subclass of `LinkedInAPIError`. An explicit empty collection is
still a successful empty search. Profiles excluded by the visibility policy do
not count as malformed results or empty server pages.

The client's HTTP logs contain operation, status, timing and byte counts. They
omit URLs, queries, payloads, message text, invitation secrets and recipients.
This policy applies to this project's logs; independently enabled dependency
loggers have their own logging behavior.

Run the Rust regression tests with `cargo test -p leankedin-park`. With the Python
extension installed in the active environment, run the offline Python tests with:

```bash
python -m unittest discover -s python -p 'test_search_validation.py'
python -m unittest discover -s python -p 'test_stub_contract.py'
```

These tests use synthetic data and do not contact LinkedIn or read browser cookies.

See [CI and publishing](.github/README.md) for release checks and the publishing procedure.

---

## ⚠️ Disclaimer

This project is not intended for large-scale profile scraping. At worst, LinkedIn may ban the account used for automation. We strongly recommend setting up a 'burner' account, with an established network if you plan to use the profile search feature. With few or no connections, many profiles will remain out of your network (see LinkedIn's connection degree system for details).

This project is intended for educational, research, and personal automation purposes only.

It is not affiliated with, endorsed, or certified by LinkedIn Corporation. Using automated scripts or scrapers may violate LinkedIn's User Agreement and lead to account restrictions. Use responsibly with appropriate delays.

---

## 📄 License

Licensed under the [PolyForm Perimeter License 1.0.1](LICENSE.md).
