"""
A high-performance Python client for the LinkedIn Voyager API powered by Rust.

Provides automated browser cookie extraction, anti-bot evasion delays,
profile data retrieval, messaging, connection invitations, post publishing,
and people searching.
"""

__all__ = [
    "AllowedCommentersScope",
    "BaseProfileData",
    "Browser",
    "ClientConfig",
    "ClientConfigError",
    "ConnectionDegree",
    "ConversationCategory",
    "ConversationData",
    "ConversationParticipantData",
    "CookieError",
    "DateData",
    "FullProfileData",
    "HttpApiError",
    "InvitationAction",
    "InvitationData",
    "LinkedInAPIError",
    "LinkedInProfileData",
    "Linkedin",
    "MessagingError",
    "NetworkDepth",
    "NetworkInfo",
    "NetworkInfoData",
    "People",
    "PeopleResult",
    "ProfileData",
    "ProfileDistance",
    "ProfileViews",
    "PublishPostError",
    "ReactionType",
    "ReceivedInvitation",
    "SearchError",
    "SearchPeople",
    "SearchPeopleBuilder",
    "UrlError",
    "UrnError",
    "VanityNameError",
    "VisibilityType",
]

from collections.abc import Sequence
from os import PathLike
from typing import ClassVar, Literal, NoReturn, overload

# ===========================================================================
# CONFIGURATION & BROWSER
# ===========================================================================
class Browser:
    """Supported browsers for automatic cookie extraction from the local machine."""

    Chrome: ClassVar[Browser]
    Firefox: ClassVar[Browser]
    Edge: ClassVar[Browser]
    Brave: ClassVar[Browser]
    Safari: ClassVar[Browser]
    Chromium: ClassVar[Browser]

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class ClientConfig:
    """
    Configuration for the LinkedIn client.

    Allows defining the target browser for cookie extraction, evasion delays (anti-bot),
    and providing manual cookie sources if necessary.
    """
    def __init__(self, /, browser: Browser | None = None) -> None:
        """
        Initializes a configuration. Firefox is used when no browser is given.

        Args:
            browser (Browser): The browser enum variant to extract cookies from.

        Returns:
            ClientConfig: A new instance of the client configuration.
        """

    @staticmethod
    def from_browser(browser: Browser) -> ClientConfig:
        """Creates a configuration using browser cookie extraction."""
        ...

    @staticmethod
    def from_cookies(li_at: str, jsessionid: str) -> ClientConfig:
        """Creates a configuration directly from li_at and JSESSIONID."""
        ...

    @staticmethod
    def from_cookies_file(path: str | PathLike[str]) -> ClientConfig:
        """Creates a configuration from a Netscape-format cookie file."""
        ...

    def with_evasion_delay_in_seconds(self, /, min: float, max: float) -> ClientConfig:
        """
        Applies a random delay in seconds between each request to avoid detection.

        Args:
            min (float): The minimum delay in seconds.
            max (float): The maximum delay in seconds.

        Returns:
            ClientConfig: The updated configuration instance (mutated in-place).

        Raises:
           ValueError: If either bound is non-finite, negative, or min > max.
           ClientConfigError: If the delay cannot be represented by the system clock.
        """

    def without_evasion_delay(self, /) -> ClientConfig:
        """
        Completely disables evasion delays.

        Warning: This is not recommended for production as it greatly increases the
        chances of your account being restricted by LinkedIn's anti-bot systems.

        Returns:
            ClientConfig: The updated configuration instance (mutated in-place).
        """

    def with_manual_cookies(self, /, li_at: str, jsessionid: str) -> ClientConfig:
        """
        Manually specifies the required cookies instead of extracting them from a browser.

        Args:
            li_at (str): The value of the `li_at` cookie.
            jsessionid (str): The value of the `JSESSIONID` cookie (including quotes if present).

        Returns:
            ClientConfig: The updated configuration instance (mutated in-place).
        """

    def with_cookies_file(self, /, path: str | PathLike[str]) -> ClientConfig:
        """
        Loads cookies from a text file formatted in the Netscape standard format.

        Args:
            path (str): The absolute or relative path to the cookies file.

        Returns:
            ClientConfig: The updated configuration instance (mutated in-place).
        """

# ===========================================================================
# ENUMS
# ===========================================================================
class InvitationAction:
    """Possible actions when responding to a LinkedIn connection invitation."""

    Accept: ClassVar[InvitationAction]
    Reject: ClassVar[InvitationAction]
    def as_str(self, /) -> str: ...

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class VisibilityType:
    """Visibility levels when publishing a new post."""

    Anyone: ClassVar[VisibilityType]
    ConnectionsOnly: ClassVar[VisibilityType]

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class AllowedCommentersScope:
    """Determines who is allowed to comment on your published posts."""

    Anyone: ClassVar[AllowedCommentersScope]
    ConnectionsOnly: ClassVar[AllowedCommentersScope]
    NoOne: ClassVar[AllowedCommentersScope]

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class ReactionType:
    """Types of reactions applicable to a LinkedIn post."""

    Like: ClassVar[ReactionType]
    Praise: ClassVar[ReactionType]
    Appreciation: ClassVar[ReactionType]
    Empathy: ClassVar[ReactionType]
    Interest: ClassVar[ReactionType]
    Entertainment: ClassVar[ReactionType]

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class NetworkDepth:
    """Network depths to target when executing searches (e.g., 1st degree connections)."""

    FirstDegree: ClassVar[NetworkDepth]
    SecondDegree: ClassVar[NetworkDepth]
    ThirdDegree: ClassVar[NetworkDepth]

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class ProfileDistance:
    """Relationship distance level relative to your current profile."""

    Myself: ClassVar[ProfileDistance]
    FirstDegree: ClassVar[ProfileDistance]
    SecondDegree: ClassVar[ProfileDistance]
    ThirdDegree: ClassVar[ProfileDistance]
    OutOfNetwork: ClassVar[ProfileDistance]
    Unknown: ClassVar[ProfileDistance]

    def search_code(self, /) -> str | None: ...
    @staticmethod
    def from_search_code(code: str) -> ProfileDistance | None: ...
    def as_str(self, /) -> str: ...

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

ConnectionDegree = ProfileDistance

class ConversationCategory:
    """Inbox categories for LinkedIn messaging."""

    Inbox: ClassVar[ConversationCategory]
    PrimaryInbox: ClassVar[ConversationCategory]
    Inmail: ClassVar[ConversationCategory]
    Archive: ClassVar[ConversationCategory]
    Unknown: ClassVar[ConversationCategory]

    __hash__ = None  # type: ignore[assignment]

    def __int__(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

# ===========================================================================
# RETURN STRUCTURES
# ===========================================================================
class ProfileViews:
    """Represents the viewing statistics of your profile."""
    @property
    def num_views(self, /) -> int: ...
    @property
    def time_frame(self, /) -> str: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class ConversationParticipantData:
    """Information regarding a participant within a LinkedIn conversation."""
    @property
    def profile_url(self, /) -> str | None: ...
    @property
    def first_name(self, /) -> str | None: ...
    @property
    def last_name(self, /) -> str | None: ...
    @property
    def distance(self, /) -> ProfileDistance | None: ...
    @property
    def urn(self, /) -> str: ...
    def __repr__(self, /) -> str: ...  # noqa: PYI029
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class ConversationData:
    """Core information about a messaging conversation."""
    @property
    def urn(self, /) -> str: ...
    @property
    def unread_count(self, /) -> int: ...
    @property
    def is_read(self, /) -> bool: ...
    @property
    def is_group(self, /) -> bool: ...
    @property
    def categories(self, /) -> list[ConversationCategory]: ...
    @property
    def created_at(self, /) -> int: ...
    @property
    def last_activity_at(self, /) -> int: ...
    @property
    def last_read_at(self, /) -> int | None: ...
    @property
    def participants(self, /) -> list[ConversationParticipantData]: ...
    def is_human_conversation(self, /) -> bool: ...
    def __repr__(self, /) -> str: ...  # noqa: PYI029
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class InvitationData:
    @property
    def first_name(self, /) -> str: ...
    @property
    def last_name(self, /) -> str: ...
    @property
    def occupation(self, /) -> str | None: ...
    @property
    def unseen(self, /) -> bool: ...
    @property
    def entity_urn(self, /) -> str: ...
    @property
    def invitation_type(self, /) -> str: ...
    @property
    def custom_message(self, /) -> bool: ...
    @property
    def shared_secret(self, /) -> str: ...
    @property
    def sent_time(self, /) -> int: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

ReceivedInvitation = InvitationData

class DateData:
    @property
    def month(self, /) -> int | None: ...
    @property
    def day(self, /) -> int | None: ...
    @property
    def year(self, /) -> int | None: ...
    def to_dict(self, /) -> dict[str, object]:
        """Returns a JSON-compatible dictionary; missing components are None."""

    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class LinkedInProfileData:
    @property
    def first_name(self, /) -> str: ...
    @property
    def last_name(self, /) -> str: ...
    @property
    def occupation(self, /) -> str | None: ...
    @property
    def vanity_name(self, /) -> str: ...
    @property
    def custom_pronoun(self, /) -> str | None: ...
    @property
    def profile_urn(self, /) -> str: ...
    @property
    def premium_subscriber(self, /) -> bool: ...

    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class BaseProfileData:
    @property
    def first_name(self, /) -> str: ...
    @property
    def last_name(self, /) -> str: ...
    @property
    def public_identifier(self, /) -> str: ...
    @property
    def profile_urn(self, /) -> str: ...
    @property
    def headline(self, /) -> str | None: ...
    @property
    def pronoun(self, /) -> str | None: ...
    @property
    def full_location(self, /) -> str | None: ...
    @property
    def country(self, /) -> str | None: ...
    @property
    def country_iso_code(self, /) -> str | None: ...
    @property
    def postal_code(self, /) -> str | None: ...
    @property
    def creator(self, /) -> bool | None: ...
    @property
    def premium(self, /) -> bool | None: ...
    @property
    def influencer(self, /) -> bool | None: ...
    @property
    def birth_date_on(self, /) -> DateData | None: ...
    @property
    def member_distance(self, /) -> ProfileDistance: ...
    @property
    def connected_since(self, /) -> int | None: ...

    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

class FullProfileData:
    @property
    def first_name(self, /) -> str: ...
    @property
    def last_name(self, /) -> str: ...
    @property
    def public_identifier(self, /) -> str: ...
    @property
    def headline(self, /) -> str | None: ...
    @property
    def pronoun(self, /) -> str | None: ...
    @property
    def is_creator(self, /) -> bool | None: ...
    @property
    def is_premium(self, /) -> bool | None: ...
    @property
    def is_influencer(self, /) -> bool | None: ...
    @property
    def birth_date_on(self, /) -> DateData | None: ...
    @property
    def full_location(self, /) -> str | None: ...
    @property
    def country(self, /) -> str | None: ...
    @property
    def country_iso_code(self, /) -> str | None: ...
    @property
    def postal_code(self, /) -> str | None: ...
    @property
    def member_distance(self, /) -> ProfileDistance: ...
    @property
    def connected_since(self, /) -> int | None: ...
    @property
    def follower_count(self, /) -> int | None: ...
    @property
    def total_connections(self, /) -> int | None: ...
    @property
    def is_following(self, /) -> bool | None: ...
    @property
    def profile_urn(self, /) -> str: ...
    # Historical ProfileData names remain available on the runtime class.
    @property
    def occupation(self, /) -> str | None: ...
    @property
    def distance(self, /) -> ProfileDistance: ...
    def to_dict(self, /) -> dict[str, object]:
        """Returns JSON-compatible values, including a nested birth date dictionary or None."""

    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

# Backwards-compatible aliases from the previous Python API.
ProfileData = FullProfileData

class NetworkInfoData:
    @property
    def following(self, /) -> bool | None: ...
    @property
    def follower_count(self, /) -> int | None: ...
    @property
    def total_connections(self, /) -> int | None: ...
    def to_dict(self, /) -> dict[str, object]: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

NetworkInfo = NetworkInfoData

class PeopleResult:
    @property
    def profile_urn(self, /) -> str | None: ...
    @property
    def full_name(self, /) -> str: ...
    @property
    def distance(self, /) -> ProfileDistance: ...
    @property
    def summary(self, /) -> str | None: ...
    @property
    def primary_subtitle(self, /) -> str | None: ...
    @property
    def secondary_subtitle(self, /) -> str | None: ...
    @property
    def navigation_url(self, /) -> str | None: ...
    @property
    def profile_id(self, /) -> str | None: ...
    def to_dict(self, /) -> dict[str, object]: ...
    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

People = PeopleResult

class SearchPeopleBuilder:
    """Powerful builder to construct and execute LinkedIn search queries."""
    @property
    def limit(self, /) -> int | None: ...
    @limit.setter
    def limit(self, /, value: int | None) -> None: ...
    @property
    def offset(self, /) -> int: ...
    @offset.setter
    def offset(self, /, value: int) -> None: ...
    @property
    def keywords(self, /) -> str | None: ...
    @keywords.setter
    def keywords(self, /, value: str | None) -> None: ...
    @property
    def connection_of(self, /) -> str | None: ...
    @connection_of.setter
    def connection_of(self, /, value: str | None) -> None: ...
    @property
    def network_depths(self, /) -> list[NetworkDepth] | None: ...
    @network_depths.setter
    def network_depths(self, /, value: Sequence[NetworkDepth] | None) -> None: ...
    @property
    def current_company(self, /) -> list[str] | None: ...
    @current_company.setter
    def current_company(self, /, value: list[str] | tuple[str, ...] | None) -> None: ...
    @property
    def past_companies(self, /) -> list[str] | None: ...
    @past_companies.setter
    def past_companies(self, /, value: list[str] | tuple[str, ...] | None) -> None: ...
    @property
    def nonprofit_interests(self, /) -> list[str] | None: ...
    @nonprofit_interests.setter
    def nonprofit_interests(
        self, /, value: list[str] | tuple[str, ...] | None
    ) -> None: ...
    @property
    def profile_languages(self, /) -> list[str] | None: ...
    @profile_languages.setter
    def profile_languages(
        self, /, value: list[str] | tuple[str, ...] | None
    ) -> None: ...
    @property
    def regions(self, /) -> list[str] | None: ...
    @regions.setter
    def regions(self, /, value: list[str] | tuple[str, ...] | None) -> None: ...
    @property
    def industries(self, /) -> list[str] | None: ...
    @industries.setter
    def industries(self, /, value: list[str] | tuple[str, ...] | None) -> None: ...
    @property
    def schools(self, /) -> list[str] | None: ...
    @schools.setter
    def schools(self, /, value: list[str] | tuple[str, ...] | None) -> None: ...
    @property
    def contact_interests(self, /) -> list[str] | None: ...
    @contact_interests.setter
    def contact_interests(
        self, /, value: list[str] | tuple[str, ...] | None
    ) -> None: ...
    @property
    def service_categories(self, /) -> list[str] | None: ...
    @service_categories.setter
    def service_categories(
        self, /, value: list[str] | tuple[str, ...] | None
    ) -> None: ...
    @property
    def include_private_profiles(self, /) -> bool: ...
    @include_private_profiles.setter
    def include_private_profiles(self, /, value: bool) -> None: ...
    @property
    def keyword_first_name(self, /) -> str | None: ...
    @keyword_first_name.setter
    def keyword_first_name(self, /, value: str | None) -> None: ...
    @property
    def keyword_last_name(self, /) -> str | None: ...
    @keyword_last_name.setter
    def keyword_last_name(self, /, value: str | None) -> None: ...
    @property
    def keyword_title(self, /) -> str | None: ...
    @keyword_title.setter
    def keyword_title(self, /, value: str | None) -> None: ...
    @property
    def keyword_company(self, /) -> str | None: ...
    @keyword_company.setter
    def keyword_company(self, /, value: str | None) -> None: ...
    @property
    def keyword_school(self, /) -> str | None: ...
    @keyword_school.setter
    def keyword_school(self, /, value: str | None) -> None: ...
    @property
    def follower_of(self, /) -> list[str] | None: ...
    @follower_of.setter
    def follower_of(self, /, value: list[str] | tuple[str, ...] | None) -> None: ...
    @property
    def open_to_volunteer(self, /) -> bool | None: ...
    @open_to_volunteer.setter
    def open_to_volunteer(self, /, value: bool | None) -> None: ...
    @staticmethod
    def from_url(url: str) -> SearchPeopleBuilder:
        """
        Generates a SearchPeopleBuilder directly from a standard LinkedIn search URL.

        Args:
            url (str): The copied URL from your LinkedIn web browser search.

        Returns:
            SearchPeopleBuilder: A builder instance pre-filled with the URL's parameters.

        Raises:
            UrlError: If the provided URL is malformed or unparseable.
        """

    def __init__(
        self,
        /,
        keywords: str | None = None,
        limit: int | None = None,
        offset: int = 0,
        connection_of: str | None = None,
        network_depths: Sequence[NetworkDepth] | None = None,
        current_company: list[str] | tuple[str, ...] | None = None,
        past_companies: list[str] | tuple[str, ...] | None = None,
        nonprofit_interests: list[str] | tuple[str, ...] | None = None,
        profile_languages: list[str] | tuple[str, ...] | None = None,
        regions: list[str] | tuple[str, ...] | None = None,
        industries: list[str] | tuple[str, ...] | None = None,
        schools: list[str] | tuple[str, ...] | None = None,
        contact_interests: list[str] | tuple[str, ...] | None = None,
        service_categories: list[str] | tuple[str, ...] | None = None,
        include_private_profiles: bool = False,
        keyword_first_name: str | None = None,
        keyword_last_name: str | None = None,
        keyword_title: str | None = None,
        keyword_company: str | None = None,
        keyword_school: str | None = None,
        follower_of: list[str] | tuple[str, ...] | None = None,
        open_to_volunteer: bool | None = None,
    ) -> None:
        """
        Constructs a new search query by specifying desired filters.

        Args:
            keywords (str, optional): The main search keyword. Defaults to None.
            limit (int, optional): The maximum number of results to retrieve. Defaults to None (fetches all).
            offset (int, optional): The starting offset for pagination. Defaults to 0.
            connection_of (str, optional): Filters results to connections of this specific profile URN. Defaults to None.
            current_company: LinkedIn internal company IDs as strings, taken from
                the currentCompany parameter in a people-search URL (not company names).
            network_depths (List[NetworkDepth], optional): Specifies the target network circles. Defaults to None.
            include_private_profiles (bool, optional): Whether to keep "LinkedIn Member" private results. Defaults to False.
            ... (other advanced demographic and professional filters).

        Returns:
            SearchPeopleBuilder: The initialized builder instance.
        """

class SearchPeople:
    """
    Python-friendly facade for common people-search filters.

    current_company and companies accept LinkedIn internal company IDs as strings,
    not names. Copy them from currentCompany in a people-search URL, or use from_url.
    String collections accept lists and tuples; a single string is not accepted.

    Use SearchPeopleBuilder for the complete advanced search surface.
    """

    @property
    def builder(self, /) -> SearchPeopleBuilder: ...
    @staticmethod
    def from_url(url: str) -> SearchPeople:
        """Imports a search directly from the browser URL.

        Raises:
            UrlError: If the provided URL is malformed or unparseable.
        """

    def __init__(
        self,
        /,
        keywords: str | None = None,
        limit: int | None = None,
        offset: int = 0,
        connection_of: str | None = None,
        network_depths: Sequence[NetworkDepth] | None = None,
        current_company: list[str] | tuple[str, ...] | None = None,
        past_companies: list[str] | tuple[str, ...] | None = None,
        nonprofit_interests: list[str] | tuple[str, ...] | None = None,
        profile_languages: list[str] | tuple[str, ...] | None = None,
        regions: list[str] | tuple[str, ...] | None = None,
        industries: list[str] | tuple[str, ...] | None = None,
        schools: list[str] | tuple[str, ...] | None = None,
        contact_interests: list[str] | tuple[str, ...] | None = None,
        service_categories: list[str] | tuple[str, ...] | None = None,
        include_private_profiles: bool = False,
        keyword_first_name: str | None = None,
        keyword_last_name: str | None = None,
        keyword_title: str | None = None,
        keyword_company: str | None = None,
        keyword_school: str | None = None,
        follower_of: list[str] | tuple[str, ...] | None = None,
        open_to_volunteer: bool | None = None,
    ) -> None: ...
    def keywords(self, /, keywords: str) -> SearchPeople: ...
    def limit(self, /, limit: int | None = None) -> SearchPeople: ...
    def offset(self, /, offset: int) -> SearchPeople: ...
    def connection_of(self, /, connection_of: str) -> SearchPeople: ...
    def network(self, /, network: Sequence[NetworkDepth]) -> SearchPeople: ...
    def network_depths(
        self, /, network_depths: Sequence[NetworkDepth]
    ) -> SearchPeople: ...
    def companies(self, /, companies: list[str] | tuple[str, ...]) -> SearchPeople:
        """Alias of current_company; accepts LinkedIn internal company IDs."""
    def current_company(
        self, /, current_company: list[str] | tuple[str, ...]
    ) -> SearchPeople:
        """Filter by company IDs from currentCompany in a LinkedIn search URL."""
    def past_companies(
        self, /, past_companies: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def schools(self, /, schools: list[str] | tuple[str, ...]) -> SearchPeople: ...
    def regions(self, /, regions: list[str] | tuple[str, ...]) -> SearchPeople: ...
    def industries(
        self, /, industries: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def profile_languages(
        self, /, profile_languages: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def service_categories(
        self, /, service_categories: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def contact_interests(
        self, /, contact_interests: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def nonprofit_interests(
        self, /, nonprofit_interests: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def follower_of(
        self, /, follower_of: list[str] | tuple[str, ...]
    ) -> SearchPeople: ...
    def keyword_first_name(self, /, first_name: str) -> SearchPeople: ...
    def first_name(self, /, first_name: str) -> SearchPeople: ...
    def keyword_last_name(self, /, last_name: str) -> SearchPeople: ...
    def last_name(self, /, last_name: str) -> SearchPeople: ...
    def keyword_title(self, /, title: str) -> SearchPeople: ...
    def title(self, /, title: str) -> SearchPeople: ...
    def keyword_company(self, /, company: str) -> SearchPeople: ...
    def company(self, /, company: str) -> SearchPeople: ...
    def keyword_school(self, /, school: str) -> SearchPeople: ...
    def school(self, /, school: str) -> SearchPeople: ...
    def include_private_profiles(self, /, include: bool = True) -> SearchPeople: ...
    def open_to_volunteer(self, /, open: bool = True) -> SearchPeople: ...
    def build(self, /) -> SearchPeopleBuilder: ...

# ===========================================================================
# MAIN CLIENT (LINKEDIN)
# ===========================================================================
class Linkedin:
    """
    The main client to interact with the LinkedIn API.

    This class maintains the active session, handles cookie extraction, bot evasion techniques,
    and securely centralizes all network calls.

    Example:
        ```python
        from leankedin_park import Linkedin, ClientConfig, Browser

        config = ClientConfig(Browser.Firefox)
        client = Linkedin.with_config(config)
        ```
    """
    @staticmethod
    def new(browser: Browser) -> Linkedin: ...
    def get_or_fetch_urn(self, /) -> str: ...
    @staticmethod
    def with_config(config: ClientConfig) -> Linkedin:
        """
        Initializes the client using the provided configuration.
        Launches the automatic extraction or imports the necessary cookies at startup.

        Args:
            config (ClientConfig): The configuration object containing browser and delay rules.

        Returns:
            Linkedin: A ready-to-use instance of the API client.

        Raises:
            ClientConfigError: If the client configuration is invalid.
            CookieError: If the authentication cookies cannot be loaded.
        """

    # --- Account ---
    def get_my_profile(self, /) -> LinkedInProfileData:
        """
        Retrieves detailed information about your own LinkedIn profile.

        Returns:
            LinkedInProfileData: Your personal data (URN, name, premium subscription, etc.).

        Raises:
            LinkedInAPIError: In case of a network or API problem.
        """

    def get_current_profile_views_last_90_days(self, /) -> ProfileViews:
        """
        Extracts statistics regarding your profile views (90-day history).

        Returns:
            ProfileViews: View count and the exact time frame assessed.

        Raises:
            LinkedInAPIError: If the profile view data cannot be retrieved.
        """

    # --- Connections ---
    def get_invitations(self, /, offset: int, limit: int) -> list[InvitationData]:
        """
        Retrieves the list of pending network connection invitations.

        Args:
            offset (int): Pagination offset (e.g., 0).
            limit (int): Maximum number of invitations to fetch.

        Returns:
            List[InvitationData]: The list of received invitations.

        Raises:
            LinkedInAPIError: In case of a network or API problem.
        """

    def reply_invitation(
        self, /, urn: str, secret: str, action: InvitationAction
    ) -> None:
        """
        Accepts or rejects a connection invitation.

        Args:
            urn (str): The entity URN of the invitation.
            secret (str): The shared secret associated with the invitation.
            action (InvitationAction): Action to perform (Accept or Reject).

        Raises:
            UrnError: If the invitation URN is incorrectly formatted.
            LinkedInAPIError: In case of a network or API problem.
        """

    def send_connection_request(
        self,
        /,
        vanity_name: str,
        profile_urn: str | None = None,
        message: str | None = None,
    ) -> None:
        """
        Sends a connection request to a third party with an optional custom message.

        Args:
            vanity_name (str): The target user's public identifier.
            profile_urn (str): The internal URN of the target user.
            message (str): The personal message to attach (limit 300 characters for premium, 200 for free).

        Raises:
            VanityNameError: If the vanity name format is invalid.
            UrnError: If the profile URN format is invalid.
            MessagingError: If the attached custom message exceeds the character limit.
            LinkedInAPIError: In case of an API error.
        """

    def remove_connection(self, /, vanity_name: str) -> None:
        """
        Removes an existing person from your network connections using their Vanity Name.

        Args:
            vanity_name (str): The public identifier of the person to disconnect from.

        Raises:
            VanityNameError: If the vanity name format is invalid.
            LinkedInAPIError: In case of an API or network error.
        """

    # --- Messaging ---
    def get_conversations_list(self, /) -> list[ConversationData]:
        """
        Retrieves the complete list of conversations from the inbox with participant details.

        Returns:
            List[ConversationData]: A list detailing your messages and group chats.

        Raises:
            LinkedInAPIError: In case of an API or network error.
        """

    def send_message(self, /, message: str, dest_urn: str) -> None:
        """
        Sends a direct message to a specific person.

        Args:
            message (str): The body content of the message.
            dest_urn (str): The internal URN of the destination profile.

        Raises:
            UrnError: If a profile URN is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    def send_group_message(
        self, /, message: str, dest_urns: list[str] | tuple[str, ...]
    ) -> None:
        """
        Sends a group message to a defined list of recipients.

        Args:
            message (str): The body content of the message.
            dest_urns (List[str]): A list containing the target URNs.

        Raises:
            UrnError: If a profile URN is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    # --- People ---
    def get_infos_of_a_profile(self, /, vanity_name: str) -> BaseProfileData:
        """
        Retrieves basic public identity data of a profile.

        Args:
            vanity_name (str): The public identifier of the target profile.

        Returns:
            BaseProfileData: Base information regarding the target profile.

        Raises:
            VanityNameError: If the vanity name format is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    def get_network_infos_of_a_profile(self, /, vanity_name: str) -> NetworkInfoData:
        """
        Retrieves network metric data associated with the profile (followers, network size).

        Args:
            vanity_name (str): The public identifier of the target profile.

        Returns:
            NetworkInfoData: Information regarding the profile's network statistics.

        Raises:
            VanityNameError: If the vanity name format is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    def get_full_infos_of_a_profile(self, /, vanity_name: str) -> FullProfileData:
        """
        All-in-one method to extract the entire profile and its network in a single call.

        Args:
            vanity_name (str): The public identifier of the target profile.

        Returns:
            FullProfileData: Complete information regarding the profile.

        Raises:
            VanityNameError: If the vanity name format is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    def follow_people(self, /, urn: str) -> None:
        """
        Subscribes to the publications and feed of a specific person.

        Args:
            urn (str): The target profile's internal URN.

        Raises:
            UrnError: If a profile URN is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    def unfollow_people(self, /, urn: str) -> None:
        """
        Unsubscribes from a person's feed (without necessarily removing them as a connection).

        Args:
            urn (str): The target profile's internal URN.

        Raises:
            UrnError: If a profile URN is invalid.
            LinkedInAPIError: In case of a network or API problem.
        """

    # --- Posts ---
    def publish_post(
        self,
        /,
        text: str,
        visibility: VisibilityType | None = None,
        comments_scope: AllowedCommentersScope | None = None,
    ) -> None:
        """
        Publishes a new LinkedIn post, configuring visibility and who is allowed to reply.

        Args:
            text (str): The text content of the post.
            visibility (VisibilityType): Who can view the post (e.g., Anyone).
            comments_scope (AllowedCommentersScope): Who can comment on the post (e.g., ConnectionsOnly).

        Raises:
            PublishPostError: If the content is empty or exceeds the character limit.
            LinkedInAPIError: In case of a network or API problem.
        """

    def react_to_post(self, /, reaction: ReactionType, post_urn: str) -> None:
        """
        Adds a reaction (Like, Praise, Support...) to an existing thread or post.

        Args:
            reaction (ReactionType): The exact reaction type to apply.
            post_urn (str): The activity URN of the targeted post.

        Raises:
            LinkedInAPIError: In case of a network or API problem.
        """

    # --- Search ---

    @overload
    def search_people(
        self, builder: SearchPeople | SearchPeopleBuilder, /
    ) -> list[PeopleResult]:
        """
        Executes a massive profile search based on multiple criteria.

        This method accepts a `SearchPeople` or `SearchPeopleBuilder` object in position 0.

        Args:
            builder (SearchPeople | SearchPeopleBuilder): The builder instance specifying filters.

        Returns:
            List[PeopleResult]: A list of profiles matching the search criteria.

        Raises:
            TypeError: For unknown keywords, extra positional arguments or mixed call styles.
            SearchError: If the response is invalid or retrieval cannot complete.
            LinkedInAPIError: In case of a network or API problem.
        """

    @overload
    def search_people(
        self,
        /,
        *,
        keywords: str | None = None,
        limit: int | None = None,
        offset: int | None = 0,
        connection_of: str | None = None,
        network_depths: Sequence[NetworkDepth] | None = None,
        current_company: list[str] | tuple[str, ...] | None = None,
        past_companies: list[str] | tuple[str, ...] | None = None,
        nonprofit_interests: list[str] | tuple[str, ...] | None = None,
        profile_languages: list[str] | tuple[str, ...] | None = None,
        regions: list[str] | tuple[str, ...] | None = None,
        industries: list[str] | tuple[str, ...] | None = None,
        schools: list[str] | tuple[str, ...] | None = None,
        contact_interests: list[str] | tuple[str, ...] | None = None,
        service_categories: list[str] | tuple[str, ...] | None = None,
        include_private_profiles: bool | None = False,
        keyword_first_name: str | None = None,
        keyword_last_name: str | None = None,
        keyword_title: str | None = None,
        keyword_company: str | None = None,
        keyword_school: str | None = None,
        follower_of: list[str] | tuple[str, ...] | None = None,
        open_to_volunteer: bool | None = None,
    ) -> list[PeopleResult]:
        """
        Executes a massive profile search based on multiple criteria.

        This method accepts direct keyword arguments (`kwargs`) for maximum flexibility.

        Args:
            current_company: LinkedIn internal company IDs as strings, copied from
                currentCompany in a people-search URL; company names are not accepted
                as company identifiers.
            **kwargs: Direct search keyword arguments (e.g., keywords="Rust", limit=10).

        Returns:
            List[PeopleResult]: A list of profiles matching the search criteria.

        Raises:
            TypeError: For unknown keywords, extra positional arguments or mixed call styles.
            SearchError: If the response is invalid or retrieval cannot complete.
            LinkedInAPIError: In case of a network or API problem.
        """

    def __enter__(self, /) -> Linkedin:
        """Enters the context manager and returns the active client.

        Raises:
            RuntimeError: If the client is already closed.
        """

    def __exit__(
        self,
        /,
        _exc_type: object | None = None,
        _exc_value: object | None = None,
        _traceback: object | None = None,
    ) -> Literal[False]:
        """Closes the Rust client and preserves exceptions from the with-block."""

    # Instances are supplied by the extension; direct construction raises TypeError.
    def __new__(cls) -> NoReturn: ...

# ===========================================================================
# EXCEPTIONS
# ===========================================================================
class LinkedInAPIError(Exception):
    """Base exception for all errors raised by leankedin_park."""

    ...

class CookieError(LinkedInAPIError):
    """Raised when cookie extraction from a browser or file fails."""

    ...

class ClientConfigError(LinkedInAPIError):
    """Raised when client configuration is invalid or incomplete."""

    ...

class MessagingError(LinkedInAPIError):
    """Raised when an error occurs during messaging operations."""

    ...

class UrnError(LinkedInAPIError):
    """Raised when a LinkedIn URN identifier cannot be parsed or resolved."""

    ...

class VanityNameError(LinkedInAPIError):
    """Raised when a vanity name or profile URL is invalid."""

    ...

class UrlError(LinkedInAPIError):
    """Raised when a URL cannot be parsed or formatted."""

    ...

class PublishPostError(LinkedInAPIError):
    """Raised when publishing a post fails."""

    ...

class HttpApiError(LinkedInAPIError):
    """HTTP error; status and context are attached when Rust raises this error.

    Directly constructing HttpApiError does not populate these attributes.
    """

    status: int
    context: str

class SearchError(LinkedInAPIError):
    """The search response is incompatible or pagination could not complete.

    No silently partial result list is returned for these failures.
    """
