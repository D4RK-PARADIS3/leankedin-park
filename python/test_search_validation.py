"""Offline regression tests for the public extension, run with unittest.

Build/install the extension first. Dummy manual cookies avoid reading any browser
profile, and limit=0 prevents every accepted search from sending a request.
"""

import unittest

from leankedin_park import (
    ClientConfig,
    Linkedin,
    SearchError,
    SearchPeople,
    SearchPeopleBuilder,
)


class SearchValidationTests(unittest.TestCase):
    def setUp(self):
        # Construction loads no real cookie or account data; no authenticated
        # method is called. Testing the extension itself also exercises PyO3.
        self.client = Linkedin.with_config(
            ClientConfig.from_cookies("dummy", "ajax:123")
        )

    def tearDown(self):
        self.client.__exit__()

    def test_unknown_keywords_raise_instead_of_silently_changing_the_search(self):
        with self.assertRaisesRegex(TypeError, "limt"):
            self.client.search_people(limit=0, limt=10)

    def test_extra_positional_argument_is_rejected(self):
        with self.assertRaisesRegex(TypeError, "at most one"):
            self.client.search_people(SearchPeopleBuilder(limit=0), "unexpected")

    def test_mixed_call_styles_are_rejected(self):
        with self.assertRaisesRegex(TypeError, "not both"):
            self.client.search_people(SearchPeopleBuilder(limit=0), limit=0)

    def test_both_builder_types_and_keyword_style_remain_supported(self):
        for builder in (SearchPeopleBuilder(limit=0), SearchPeople(limit=0)):
            self.assertEqual(self.client.search_people(builder), [])
        self.assertEqual(self.client.search_people(limit=0), [])

    def test_empty_keyword_expansion_preserves_the_positional_builder(self):
        # If **{} replaced the builder with a default search, this would attempt
        # HTTP. limit=0 must continue to suppress the retrieval loop entirely.
        self.assertEqual(
            self.client.search_people(SearchPeopleBuilder(limit=0), **{}), []
        )

    def test_mutated_builder_pagination_is_used_by_search(self):
        builder = SearchPeopleBuilder(limit=10)
        builder.limit = 0
        builder.offset = 5
        self.assertEqual(self.client.search_people(builder), [])

    def test_invalid_filter_types_are_rejected_before_transport(self):
        with self.assertRaises(TypeError):
            self.client.search_people(limit=0, keywords=123)

    def test_argument_validation_does_not_depend_on_an_open_session(self):
        self.client.__exit__()
        with self.assertRaisesRegex(TypeError, "limt"):
            self.client.search_people(limt=10)

    def test_search_error_is_part_of_the_public_exception_hierarchy(self):
        from leankedin_park import LinkedInAPIError

        self.assertTrue(issubclass(SearchError, LinkedInAPIError))

    def test_invalid_search_urls_raise_url_error(self):
        from leankedin_park import UrlError

        for factory in (SearchPeople, SearchPeopleBuilder):
            with self.subTest(factory=factory), self.assertRaises(UrlError):
                factory.from_url("not a URL")

    def test_invalid_identifiers_raise_specific_errors_before_transport(self):
        from leankedin_park import UrnError, VanityNameError

        for name in (
            "remove_connection",
            "get_infos_of_a_profile",
            "get_network_infos_of_a_profile",
            "get_full_infos_of_a_profile",
            "send_connection_request",
        ):
            with self.subTest(method=name), self.assertRaises(VanityNameError):
                getattr(self.client, name)("invalid/name")
        calls = (
            ("send_message", ("Example message", "")),
            ("send_group_message", ("Example message", [""])),
            ("follow_people", ("",)),
            ("unfollow_people", ("",)),
            ("send_connection_request", ("example", "")),
        )
        for name, args in calls:
            with self.subTest(method=name), self.assertRaises(UrnError):
                getattr(self.client, name)(*args)

    def test_entering_closed_client_raises_runtime_error(self):
        self.client.__exit__()
        with self.assertRaisesRegex(RuntimeError, "already closed"):
            self.client.__enter__()


if __name__ == "__main__":
    unittest.main()
