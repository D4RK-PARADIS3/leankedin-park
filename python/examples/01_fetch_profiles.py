import logging

from leankedin_park import (
    Browser,
    ClientConfig,
    Linkedin,
    LinkedInAPIError,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger(__name__)


def main() -> None:
    config = ClientConfig(Browser.Firefox).with_evasion_delay_in_seconds(2.0, 5.0)

    try:
        with Linkedin.with_config(config) as linkedin:
            logger.info("Initializing LinkedIn client...")

            # The Python binding exposes the current account profile as
            # LinkedInProfileData.
            my_profile = linkedin.get_my_profile()
            logger.info(
                "Connected as %s %s",
                my_profile.first_name,
                my_profile.last_name,
            )
            logger.info("My public identifier: %s", my_profile.vanity_name)
            logger.info("My profile URN: %s", my_profile.profile_urn)
            logger.info("Premium subscriber: %s", my_profile.premium_subscriber)

            logger.info("My URN from the client: %s", linkedin.get_or_fetch_urn())

            """
            target = linkedin.get_infos_of_a_profile("williamhgates")
            logger.info(
                "Profile found: %s %s",
                target.first_name,
                target.last_name,
            )
            logger.info("Headline: %s", target.headline)
            logger.info("Location: %s", target.full_location)
            logger.info("Network distance: %s", target.member_distance)
            """
            # Full profile information is returned as FullProfileData.
            full = linkedin.get_full_infos_of_a_profile("williamhgates")
            logger.info("Full profile: %s %s", full.first_name, full.last_name)
            logger.info("Followers: %s", full.follower_count)
            logger.info("Following: %s", full.is_following)

            if full.birth_date_on is not None:
                logger.info(
                    "Birth date: %s-%s-%s",
                    full.birth_date_on.year,
                    full.birth_date_on.month,
                    full.birth_date_on.day,
                )

    except LinkedInAPIError as exc:
        logger.error("LinkedIn API error: %s", exc)


if __name__ == "__main__":
    main()
