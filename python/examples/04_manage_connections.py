import logging

from leankedin_park import (
    Browser,
    ClientConfig,
    Linkedin,
    LinkedInAPIError,
    UrnError,
    VanityNameError,
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
            logger.info("Fetching pending connection invitations...")
            invitations = linkedin.get_invitations(0, 10)
            logger.info("Found %d invitations.", len(invitations))

            for invitation in invitations:
                logger.info(
                    "Invitation from %s %s",
                    invitation.first_name,
                    invitation.last_name,
                )
                logger.info("Occupation: %s", invitation.occupation)
                logger.info("Unseen: %s", invitation.unseen)
                logger.info("Invitation type: %s", invitation.invitation_type)
                logger.info("Custom message: %s", invitation.custom_message)
                logger.info("Sent time: %s", invitation.sent_time)

                logger.info(
                    "Shared secret is available: %s",
                    bool(invitation.shared_secret),
                )

                # Deliberately do not accept/reject automatically.
                # Review the invitation first before performing an action.
                #
                # try:
                #     linkedin.reply_invitation(
                #         invitation.entity_urn,
                #         invitation.shared_secret,
                #         InvitationAction.Accept,
                #     )
                # except UrnError as exc:
                #     logger.error("Invalid invitation URN: %s", exc)
                # except LinkedInAPIError as exc:
                #     logger.error("Failed to reply to invitation: %s", exc)

            try:
                linkedin.send_connection_request(
                    "bad/vanity/name",
                    profile_urn="not-a-valid-profile-id",
                    message=None,
                )
            except VanityNameError as exc:
                logger.info("VanityNameError correctly raised: %s", exc)
            except UrnError as exc:
                logger.info("UrnError correctly raised: %s", exc)

                # linkedin.remove_connection("target-vanity-name")

    except LinkedInAPIError as exc:
        logger.error("LinkedIn API error: %s", exc)


if __name__ == "__main__":
    main()
