# leankedin-park

[![PyPI version](https://img.shields.io/pypi/v/leankedin-park.svg)](https://pypi.org/project/leankedin-park/)
[![Python versions](https://img.shields.io/pypi/pyversions/leankedin-park.svg)](https://pypi.org/project/leankedin-park/)
[![License: PolyForm Perimeter 1.0.1](https://img.shields.io/badge/License-PolyForm_Perimeter_1.0.1-blue.svg)](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/LICENSE.md)

A fast, ergonomic, and memory-efficient Python client for the LinkedIn Voyager API, powered by a native Rust core.

## Key Features

- ⚡ **Ultra-Fast & Lightweight**: Built on native Rust HTTP stack. No resource-heavy browser automation (Puppeteer/Playwright/Selenium) eating CPU and RAM.
- 🍪 **Automatic Cookie Extraction**: Effortlessly extract session cookies directly from local browsers (Firefox, Chrome, Safari, Edge, Brave, Chromium) or supply manual cookie headers.
- 🛡️ **Anti-Bot Throttling**: Built-in human-like request jitter, atomic timeline reservation, and serialized FIFO request queueing.
- 🐍 **First-Class Python Ergonomics**: Clean context managers (`with Linkedin...`), fluent query builders, and complete type stubs (`.pyi`) for full autocompletion in VS Code, PyCharm, and type checkers (Pyright, Mypy).
- 👥 **Comprehensive Voyager API Coverage**:
  - **People Search**: Advanced filters (keywords, companies, locations, network depth) or direct import from copied LinkedIn search URLs.
  - **Profiles**: Profile inspection, follower metrics, view counts, connection statuses.
  - **Connections & Invitations**: Manage pending invitations, accept/reject requests, and send connection requests with personalized notes.
  - **Messaging**: Retrieve inbox conversations, send direct messages and group chats.
  - **Posts**: Publish new posts with custom audience visibility (`Anyone`, `ConnectionsOnly`) and react to posts.
  - **Account**: Retrieve self profile and 90-day profile view metrics.

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

```bash
pip install leankedin-park
```

Pre-built binary wheels are provided for **Linux (x86_64)**, **macOS (Apple Silicon & Intel)**, and **Windows (x86_64)** on Python 3.10+. **No Rust toolchain is needed to install and run.**

These wheels target standard CPython. Linux requires glibc 2.28 or newer;
Alpine Linux (musl) and Linux ARM64 are not covered.

> [!NOTE]
> For pre-release alpha versions:
> ```bash
> pip install --pre leankedin-park
> # or specifically:
> pip install leankedin-park==0.1.0a1
> ```

---

## Quickstart

```python
import json
from pprint import pformat

from leankedin_park import Browser, ClientConfig, Linkedin

# Initialize configuration with browser cookie extraction and human-like request jitter
config = ClientConfig(Browser.Firefox).with_evasion_delay_in_seconds(2.0, 4.0)

with Linkedin.with_config(config) as linkedin:
    # Get current user profile
    me = linkedin.get_my_profile()
    print(f"Logged in as: {me.first_name} {me.last_name} ({me.vanity_name})")

    # Fetch 90-day profile views
    views = linkedin.get_current_profile_views_last_90_days()
    print(f"Profile views: {views.num_views} ({views.time_frame})")

    # Fetch a public profile and display all identity, location, and network fields
    profile = linkedin.get_full_infos_of_a_profile("williamhgates")
    profile_data = profile.to_dict()
    print(f"Full profile:\n{pformat(profile_data, sort_dicts=False)}")

    # Save the full profile as indented UTF-8 JSON
    with open("williamhgates.json", "w", encoding="utf-8") as file:
        json.dump(profile_data, file, indent=4, ensure_ascii=False)

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
from leankedin_park import Browser, Linkedin, SearchPeople, NetworkDepth, SearchPeopleBuilder

with Linkedin.new(Browser.Firefox) as linkedin:
    # 1. Fluent builder
    search = (
        SearchPeople()
        .keywords("Rust")
        .companies(["12345", "67890"])  # Example IDs: replace with IDs from your search URL
        .network([NetworkDepth.FirstDegree, NetworkDepth.SecondDegree])
        .limit(20)
    )
    results = linkedin.search_people(search)

    # 2. Or directly from a copied browser search URL
    url = "https://www.linkedin.com/search/results/people/?keywords=Rust&network=%5B%22F%22%5D"
    builder = SearchPeopleBuilder.from_url(url)
    results = linkedin.search_people(builder)
```

`SearchPeopleBuilder` fields are readable and writable: use `builder.limit = 10`
and `builder.offset = 5`. Set `builder.limit = None` to clear the result limit.
These assignments replace the former `builder.limit(10)` and `builder.offset(5)`
calls. Fluent chaining remains available on `SearchPeople` via
`SearchPeople().limit(10).offset(5)`.

Search values are encoded as Rest.li primitive values, including punctuation and
Unicode. A search returns a list only when its limit or the server-reported end
is reached. Invalid responses, malformed profile actions, invalid pagination and
safety-limit interruptions raise `SearchError` (a `LinkedInAPIError` subclass),
rather than returning silently partial results. Missing profile actions and
profiles excluded by visibility are allowed.

Company filters (`current_company` and its `companies` alias) take LinkedIn internal
company IDs as strings, not company names. Select companies in a LinkedIn people
search and copy the IDs from the URL’s `currentCompany` parameter. For example,
`currentCompany=%5B%2212345%22%2C%2267890%22%5D` encodes `["12345", "67890"]`
(illustrative IDs). You can also pass the entire search URL to
`SearchPeople.from_url(url)` or `SearchPeopleBuilder.from_url(url)`.

---

## Examples

Explore full Python examples in the repository:

- [`01_fetch_profiles.py`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/python/examples/01_fetch_profiles.py): Profile inspection and follower metrics.
- [`02_search_and_network.py`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/python/examples/02_search_and_network.py): Advanced search builder and URL parsing.
- [`03_messaging_and_posting.py`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/python/examples/03_messaging_and_posting.py): Publishing posts and messaging.
- [`04_manage_connections.py`](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/python/examples/04_manage_connections.py): Managing pending invitations and connection requests.

---

## Contributing & Local Development

To build the extension locally for development:

```bash
# Create virtual environment
python -m venv .venv
source .venv/bin/activate

# Install build tool
pip install 'maturin>=1.15,<2.0'

# Compile and install in editable development mode
cd python
maturin develop --release
```

---

## ⚠️ Disclaimer

This project is intended for educational, research, and personal automation purposes only.
It is not affiliated with, endorsed, or certified by LinkedIn Corporation. Using automated tools may violate LinkedIn's User Agreement and may result in account restrictions. We strongly recommend testing with a secondary/burner account and using sensible evasion delays.

---

## 📄 License

Licensed under the [PolyForm Perimeter License 1.0.1](https://github.com/D4RK-PARADIS3/leankedin-park/blob/main/LICENSE.md).
