import logging

from leankedin_park import (
    Browser,
    ClientConfig,
    Linkedin,
    LinkedInAPIError,
    NetworkDepth,
    SearchPeople,
    SearchPeopleBuilder,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger(__name__)


def main() -> None:
    config = ClientConfig(Browser.Firefox).with_evasion_delay_in_seconds(2.0, 4.0)

    try:
        with Linkedin.with_config(config) as linkedin:
            logger.info("Searching for Python developers...")

            # Recommended: direct keyword arguments for one-off/simple searches.
            results = linkedin.search_people(
                keywords="Python",
                limit=10,
                # regions=["101620260"], # Paris GeoURN
                include_private_profiles=True,
            )

            logger.info("Found %d results.", len(results))

            for person in results:
                logger.info(
                    "- %s | %s | %s",
                    person.full_name,
                    person.primary_subtitle or "(no title)",
                    person.secondary_subtitle or "(no location)",
                )

                if person.summary:
                    logger.info("  Summary: %s", person.summary)

                if person.profile_id:
                    logger.info("  Profile ID: %s", person.profile_id)

                if person.navigation_url:
                    logger.info("  Profile URL: %s", person.navigation_url)

            # The explicit SearchPeopleBuilder path is also supported.
            builder = SearchPeopleBuilder(
                keywords="Rust Developer",
                limit=10,
                regions=["105646813"],
            )

            # Builder fields are readable and writable from Python.
            builder.limit = 5

            builder_results = linkedin.search_people(builder)
            logger.info(
                "Builder search returned %d results.",
                len(builder_results),
            )

            # Python-friendly fluent facade: use this for reusable searches.
            search = (
                SearchPeople()
                .keywords("Rust")
                .companies(
                    ["12345", "67890"]
                )  # Example IDs: replace with IDs from your search URL
                .network([NetworkDepth.FirstDegree, NetworkDepth.SecondDegree])
                .limit(20)
            )
            fluent_results = linkedin.search_people(search)

            logger.info(
                "Fluent search returned %d results.",
                len(fluent_results),
            )

            # LinkedIn search URLs can be converted back to a builder.
            search_url = (
                "https://www.linkedin.com/search/results/people/"
                "?keywords=Rust%20Developer"
                "&network=%5B%22F%22%2C%22S%22%5D"
                "&contactInterests=%5B%22networking%22%5D"
            )
            builder_from_url = SearchPeopleBuilder.from_url(search_url)
            logger.info(
                "Builder created from URL: %r",
                builder_from_url,
            )

    except LinkedInAPIError as exc:
        logger.error("LinkedIn API error: %s", exc)


if __name__ == "__main__":
    main()
