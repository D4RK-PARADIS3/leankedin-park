import logging

from leankedin_park import (
    AllowedCommentersScope,
    Browser,
    ClientConfig,
    Linkedin,
    LinkedInAPIError,
    PublishPostError,
    VisibilityType,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)

logger = logging.getLogger(__name__)


def main() -> None:
    config = ClientConfig(Browser.Firefox).with_evasion_delay_in_seconds(1.5, 3.0)

    try:
        with Linkedin.with_config(config) as linkedin:
            # The visibility and comment scope arguments are optional.
            # Omitting them uses the Rust core defaults (Anyone / Anyone).
            logger.info("Publishing a post with core defaults...")
            try:
                linkedin.publish_post(
                    "Hello LinkedIn! I'm automating my workflow with Rust and Python! "
                    "🦀🐍🚀"
                )
                logger.info("Post published successfully.")
            except PublishPostError as exc:
                logger.error("Post validation/publication error: %s", exc)
            except LinkedInAPIError as exc:
                logger.error("LinkedIn API error while publishing: %s", exc)

            # The explicit form remains available when custom visibility is needed.
            try:
                linkedin.publish_post(
                    text="Connections-only example",
                    visibility=VisibilityType.ConnectionsOnly,
                    comments_scope=AllowedCommentersScope.ConnectionsOnly,
                )
                logger.info("Connections-only post published successfully.")
            except PublishPostError as exc:
                logger.error("Post validation/publication error: %s", exc)
            except LinkedInAPIError as exc:
                logger.error("LinkedIn API error while publishing: %s", exc)

            # Replace this placeholder with the intended recipient's profile URN.
            target_urn = "ACoAA..."
            message_text = "Hello! This is an example message sent with leankedin-park."

            try:
                linkedin.send_message(message_text, target_urn)
                logger.info("Message sent successfully.")
            except LinkedInAPIError as exc:
                logger.error("Failed to send message: %s", exc)

            # Group-message API example. Kept commented to avoid an accidental send.
            #
            # linkedin.send_group_message(
            #     "Hello everyone!",
            #     [
            #         "ACoAA...",
            #         "ACoBB...",
            #     ],
            # )

    except LinkedInAPIError as exc:
        logger.error("General LinkedIn error: %s", exc)


if __name__ == "__main__":
    main()
