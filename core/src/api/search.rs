//! People search with encoded filters and explicit completeness checks.

use crate::api::{Linkedin, RequestType, ResponseExt};
use crate::errors::{LinkedInError, SearchError};
use crate::models::people::ConnectionDegree;
use crate::models::search::{People, SearchPeopleBuilder, dto};
use log::{debug, info};
use std::fmt::{self, Write};

const VOYAGER_SEARCH_QUERY_ID: &str = "voyagerSearchDashClusters.b0928897b71bd00a5a7291755dcd64f0";
const GLOBAL_SEARCH_ORIGIN: &str = "GLOBAL_SEARCH_HEADER";
const DEFAULT_PAGE_SIZE: usize = 10;
const COLLECTION_TYPE: &str = "com.linkedin.restli.common.CollectionResponse";
const CLUSTER_TYPE: &str = "com.linkedin.voyager.dash.search.SearchClusterViewModel";
const ITEM_TYPE: &str = "com.linkedin.voyager.dash.search.SearchItem";

/// Sub-client for searching people with the requested filters.
pub struct SearchClient<'a>(pub(crate) &'a Linkedin);

impl SearchClient<'_> {
    /// Searches from the requested offset and returns up to the requested limit.
    ///
    /// Cards without profile data and profiles excluded by visibility are skipped.
    /// Malformed responses and pagination failures return `LinkedInError::Search`.
    /// Safety guards never return silently partial results.
    pub async fn search_people(
        &self,
        builder: &SearchPeopleBuilder,
    ) -> Result<Vec<People>, LinkedInError> {
        let filters = build_filters(builder);
        let mut results = match builder.limit {
            Some(limit) => Vec::with_capacity(limit),
            None => Vec::new(),
        };
        let mut state = SearchState::new(builder.offset, builder.limit);

        for _ in 0..Linkedin::MAX_REPEATED_REQUESTS {
            if state.should_stop(results.len()) {
                break;
            }

            let params = VoyagerParams {
                start: state.next_start,
                keywords: builder.keywords.as_deref(),
                filters: &filters,
            };
            let uri = format!("/graphql?variables={params}&queryId={VOYAGER_SEARCH_QUERY_ID}");
            info!("Fetching search page at offset {}", state.next_start);

            let response = self
                .0
                ._fetch::<()>(&uri, None, RequestType::ApiRequest)
                .await?
                .verify_success("Failed to perform search")?;
            debug!("Search request was successful");

            state.collect_page(
                &response.bytes,
                builder.include_private_profiles,
                &mut results,
            )?;
            if state.should_stop(results.len()) {
                break;
            }
            state.check_safety(results.len())?;
        }

        state.ensure_complete(results.len())?;
        if let Some(limit) = builder.limit {
            results.truncate(limit);
        }
        Ok(results)
    }
}

fn build_filters(builder: &SearchPeopleBuilder) -> String {
    let mut filters = String::with_capacity(512);
    filters.push_str("List((key:resultType,value:List(PEOPLE))");

    append_filter(
        &mut filters,
        "connectionOf",
        builder.connection_of.as_deref(),
    );
    if builder.open_to_volunteer == Some(true) {
        filters.push_str(",(key:openToVolunteer,value:List(true))");
    }

    // Preserve HashSet iteration without imposing a new order.
    // A non-empty set with no searchable degree codes still produces List().
    if let Some(depths) = &builder.network_depths
        && !depths.is_empty()
    {
        let codes = depths
            .iter()
            .filter_map(|depth| depth.search_code())
            .collect::<Vec<_>>()
            .join(",");
        let _ = write!(filters, ",(key:network,value:List({codes}))");
    }

    for (key, values) in [
        ("followerOf", &builder.follower_of),
        ("geoUrn", &builder.regions),
        ("industry", &builder.industries),
        ("currentCompany", &builder.current_company),
        ("pastCompany", &builder.past_companies),
        ("profileLanguage", &builder.profile_languages),
        ("nonprofitInterest", &builder.nonprofit_interests),
        ("schools", &builder.schools),
        ("serviceCategory", &builder.service_categories),
        ("contactInterests", &builder.contact_interests),
    ] {
        append_filter(&mut filters, key, values.iter().map(String::as_str));
    }

    for (key, value) in [
        ("firstName", builder.keyword_first_name.as_deref()),
        ("lastName", builder.keyword_last_name.as_deref()),
        ("title", builder.keyword_title.as_deref()),
        ("company", builder.keyword_company.as_deref()),
        ("school", builder.keyword_school.as_deref()),
    ] {
        append_filter(&mut filters, key, value);
    }

    filters.push(')');
    filters
}

// The same format handles lists and optional values. It distinguishes
// an absent value from Some(""): the empty string is emitted as in the original.
fn append_filter<'a>(
    destination: &mut String,
    key: &str,
    values: impl IntoIterator<Item = &'a str>,
) {
    let mut values = values.into_iter();
    let Some(first) = values.next() else {
        return;
    };

    destination.push_str(",(key:");
    destination.push_str(key);
    destination.push_str(",value:List(");
    write_restli_encoded(destination, first).expect("writing to String cannot fail");
    for value in values {
        destination.push(',');
        write_restli_encoded(destination, value).expect("writing to String cannot fail");
    }
    destination.push_str("))");
}

struct VoyagerParams<'a> {
    start: usize,
    keywords: Option<&'a str>,
    filters: &'a str,
}

impl fmt::Display for VoyagerParams<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(start:{},origin:{GLOBAL_SEARCH_ORIGIN},query:(",
            self.start
        )?;
        if let Some(keywords) = self.keywords {
            f.write_str("keywords:")?;
            write_restli_encoded(f, keywords)?;
            f.write_str(",")?;
        }
        write!(
            f,
            "flagshipSearchIntent:SEARCH_SRP,queryParameters:{},includeFiltersInResponse:false))",
            self.filters
        )
    }
}

/// Encode a primitive value, leaving the surrounding Rest.li structure intact.
fn write_restli_encoded(out: &mut impl fmt::Write, value: &str) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.write_char(char::from(byte))?;
        } else {
            out.write_char('%')?;
            out.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
            out.write_char(char::from(HEX[usize::from(byte & 15)]))?;
        }
    }
    Ok(())
}

struct SearchState {
    next_start: usize,
    total_available: Option<usize>,
    limit: Option<usize>,
    consecutive_empty_pages: usize,
}

impl SearchState {
    fn new(offset: usize, limit: Option<usize>) -> Self {
        Self {
            next_start: offset,
            total_available: None,
            limit,
            consecutive_empty_pages: 0,
        }
    }

    fn should_stop(&self, current_count: usize) -> bool {
        self.limit.is_some_and(|limit| current_count >= limit)
            || self
                .total_available
                .is_some_and(|total| self.next_start >= total)
    }

    fn collect_page(
        &mut self,
        bytes: &[u8],
        include_private_profiles: bool,
        results: &mut Vec<People>,
    ) -> Result<(), SearchError> {
        let root: dto::ResponseRoot<'_> =
            serde_json::from_slice(bytes).map_err(SearchError::InvalidResponse)?;
        let clusters = root
            .data
            .and_then(|data| data.clusters)
            .ok_or(SearchError::UnexpectedResponse("missing search collection"))?;
        if clusters.type_name != COLLECTION_TYPE {
            return Err(SearchError::UnexpectedResponse(
                "unexpected search collection type",
            ));
        }
        let paging = clusters
            .paging
            .ok_or(SearchError::UnexpectedResponse("missing pagination"))?;
        let mut raw_page_count = 0;
        for cluster in clusters.elements {
            if cluster.type_name != CLUSTER_TYPE {
                continue;
            }
            let items = cluster.items.ok_or(SearchError::UnexpectedResponse(
                "missing search cluster items",
            ))?;
            for item in items {
                if item.type_name != ITEM_TYPE {
                    continue;
                }
                let Some(item) = item.item.entity_result else {
                    continue;
                };
                let index = raw_page_count;
                raw_page_count += 1;
                if !include_private_profiles
                    && item.entity_custom_tracking_info.member_distance
                        == ConnectionDegree::OutOfNetwork
                {
                    continue;
                }
                // Results beyond the requested limit need not be converted.
                if self.limit.is_some_and(|limit| results.len() >= limit) {
                    continue;
                }
                results.push(People::try_from(item).map_err(|source| {
                    SearchError::InvalidResult {
                        index,
                        source: Box::new(source),
                    }
                })?);
            }
        }
        self.update(&paging, raw_page_count)
    }

    fn update(&mut self, paging: &dto::Paging, raw_page_count: usize) -> Result<(), SearchError> {
        if paging.start != self.next_start {
            return Err(SearchError::UnexpectedResponse(
                "pagination start does not match requested offset",
            ));
        }
        let next = paging
            .start
            .checked_add(paging.count)
            .ok_or(SearchError::UnexpectedResponse("pagination overflow"))?;
        if paging.count == 0 && paging.start < paging.total {
            return Err(SearchError::UnexpectedResponse(
                "pagination does not advance",
            ));
        }
        self.total_available = Some(paging.total);
        self.next_start = next;
        self.consecutive_empty_pages = if raw_page_count == 0 {
            self.consecutive_empty_pages + 1
        } else {
            0
        };
        Ok(())
    }

    fn check_safety(&self, collected: usize) -> Result<(), SearchError> {
        let reason = if self.consecutive_empty_pages >= 3 {
            Some("three consecutive empty server pages")
        } else if collected / DEFAULT_PAGE_SIZE >= Linkedin::MAX_REPEATED_REQUESTS {
            Some("result safety limit reached")
        } else {
            None
        };
        match reason {
            Some(reason) => Err(SearchError::Incomplete { collected, reason }),
            None => Ok(()),
        }
    }

    fn ensure_complete(&self, collected: usize) -> Result<(), SearchError> {
        if self.should_stop(collected) {
            Ok(())
        } else {
            Err(SearchError::Incomplete {
                collected,
                reason: "request safety limit reached",
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn person(distance: &str, action: Option<&str>) -> Value {
        let mut entity = json!({
            "title": {"text": "Example Person"},
            "entityCustomTrackingInfo": {"memberDistance": distance}
        });
        if let Some(action) = action {
            entity["lazyLoadedActions"] = json!({"entityUrn": action});
        }
        json!({"_type": ITEM_TYPE, "item": {"entityResult": entity}})
    }

    fn page(start: usize, count: usize, total: usize, items: Vec<Value>) -> Value {
        json!({"data": {"searchDashClustersByAll": {
            "_type": COLLECTION_TYPE,
            "paging": {"start": start, "count": count, "total": total},
            "elements": [{"_type": CLUSTER_TYPE, "items": items}]
        }}})
    }

    fn collect(
        state: &mut SearchState,
        value: &Value,
        results: &mut Vec<People>,
    ) -> Result<(), SearchError> {
        state.collect_page(&serde_json::to_vec(value).unwrap(), false, results)
    }

    #[test]
    fn encodes_values_once_in_final_url() {
        let value = "Rust, Python (é): &#+% /'~_-.*";
        let encoded = "Rust%2C%20Python%20%28%C3%A9%29%3A%20%26%23%2B%25%20%2F%27~_-.%2A";
        let builder = SearchPeopleBuilder {
            keywords: Some(value.into()),
            keyword_title: Some(value.into()),
            current_company: vec!["a,b".into(), "urn:li:company:1".into()],
            ..Default::default()
        };
        let filters = build_filters(&builder);
        let params = VoyagerParams {
            start: 5,
            keywords: builder.keywords.as_deref(),
            filters: &filters,
        };
        let uri = format!("https://www.linkedin.com/graphql?variables={params}&queryId=test");
        let parsed = url::Url::parse(&uri).unwrap();
        assert_eq!(parsed.as_str(), uri);
        assert_eq!(parsed.query_pairs().count(), 2);
        assert!(parsed.fragment().is_none());
        assert!(uri.contains(&format!("keywords:{encoded},")));
        assert!(uri.contains(&format!("key:title,value:List({encoded})")));
        assert!(uri.contains("key:currentCompany,value:List(a%2Cb,urn%3Ali%3Acompany%3A1)"));
    }

    #[test]
    fn encoding_preserves_empty_and_absent_filters() {
        let filters = build_filters(&SearchPeopleBuilder {
            keyword_title: Some(String::new()),
            ..Default::default()
        });
        assert!(filters.contains("key:title,value:List()"));
        assert!(!filters.contains("key:company"));
    }

    #[test]
    fn malformed_or_missing_response_fields_are_errors() {
        let mut state = SearchState::new(0, None);
        let mut results = Vec::new();
        assert!(matches!(
            state.collect_page(b"{", false, &mut results),
            Err(SearchError::InvalidResponse(_))
        ));
        assert!(matches!(
            collect(&mut state, &json!({}), &mut results),
            Err(SearchError::UnexpectedResponse(_))
        ));
        let mut value = page(0, 10, 10, vec![]);
        value["data"]["searchDashClustersByAll"]
            .as_object_mut()
            .unwrap()
            .remove("elements");
        assert!(matches!(
            collect(&mut state, &value, &mut results),
            Err(SearchError::InvalidResponse(_))
        ));
        for key in ["_type", "paging"] {
            let mut value = page(0, 10, 10, vec![]);
            value["data"]["searchDashClustersByAll"][key] = if key == "_type" {
                json!("changed")
            } else {
                Value::Null
            };
            assert!(matches!(
                collect(&mut state, &value, &mut results),
                Err(SearchError::UnexpectedResponse(_))
            ));
        }
        let mut value = page(0, 10, 10, vec![]);
        value["data"]["searchDashClustersByAll"]["elements"][0]
            .as_object_mut()
            .unwrap()
            .remove("items");
        assert!(matches!(
            collect(&mut state, &value, &mut results),
            Err(SearchError::UnexpectedResponse(_))
        ));
    }

    #[test]
    fn absent_actions_and_non_profile_cards_are_allowed() {
        let mut state = SearchState::new(0, Some(100));
        let mut results = Vec::new();
        collect(
            &mut state,
            &page(
                0,
                10,
                2,
                vec![
                    json!({"_type": ITEM_TYPE, "item": {}}),
                    person("DISTANCE_2", None),
                ],
            ),
            &mut results,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].profile_id.is_none());
        state.ensure_complete(results.len()).unwrap();
    }

    #[test]
    fn malformed_action_is_an_indexed_error_and_valid_id_is_retained() {
        let mut state = SearchState::new(0, None);
        let mut results = Vec::new();
        let value = page(
            0,
            10,
            10,
            vec![
                person("DISTANCE_1", Some("urn:li:fsd_profileActions:(ACoAAA,foo)")),
                person("DISTANCE_2", Some("broken")),
            ],
        );
        assert!(matches!(
            collect(&mut state, &value, &mut results),
            Err(SearchError::InvalidResult { index: 1, .. })
        ));
        assert_eq!(results[0].profile_id.as_ref().unwrap().as_str(), "ACoAAA");
    }

    #[test]
    fn visibility_exclusions_are_neither_errors_nor_empty_pages() {
        let mut state = SearchState::new(0, None);
        let mut results = Vec::new();
        for start in [0, 10, 20] {
            collect(
                &mut state,
                &page(
                    start,
                    10,
                    40,
                    vec![person("OUT_OF_NETWORK", Some("broken"))],
                ),
                &mut results,
            )
            .unwrap();
            state.check_safety(results.len()).unwrap();
        }
        assert!(!state.should_stop(0));
        collect(
            &mut state,
            &page(30, 10, 40, vec![person("DISTANCE_2", None)]),
            &mut results,
        )
        .unwrap();
        state.ensure_complete(results.len()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(matches!(
            SearchState::new(0, None).collect_page(
                &serde_json::to_vec(&page(
                    0,
                    10,
                    10,
                    vec![person("OUT_OF_NETWORK", Some("broken"))]
                ))
                .unwrap(),
                true,
                &mut Vec::new()
            ),
            Err(SearchError::InvalidResult { .. })
        ));
    }

    #[test]
    fn empty_collection_is_a_success_at_reported_end() {
        let mut value = page(0, 0, 0, vec![]);
        value["data"]["searchDashClustersByAll"]["elements"] = json!([]);
        let mut state = SearchState::new(0, None);
        collect(&mut state, &value, &mut Vec::new()).unwrap();
        state.ensure_complete(0).unwrap();
    }

    #[test]
    fn invalid_paging_cannot_succeed_even_if_limit_was_reached() {
        for (start, count, total) in [(0, 0, 10), (1, 10, 20), (usize::MAX, 1, usize::MAX)] {
            let offset = if start == usize::MAX { start } else { 0 };
            let mut state = SearchState::new(offset, Some(1));
            assert!(matches!(
                collect(
                    &mut state,
                    &page(start, count, total, vec![person("DISTANCE_1", None)]),
                    &mut Vec::new()
                ),
                Err(SearchError::UnexpectedResponse(_))
            ));
        }
    }

    #[test]
    fn safety_stops_are_errors_but_requested_limit_and_last_page_succeed() {
        let mut state = SearchState::new(0, None);
        for start in [0, 10, 20] {
            collect(&mut state, &page(start, 10, 100, vec![]), &mut Vec::new()).unwrap();
        }
        assert!(matches!(
            state.check_safety(0),
            Err(SearchError::Incomplete { collected: 0, .. })
        ));
        assert!(matches!(
            state.ensure_complete(0),
            Err(SearchError::Incomplete { .. })
        ));
        let mut state = SearchState::new(0, Some(1));
        let mut results = Vec::new();
        collect(
            &mut state,
            &page(
                0,
                10,
                100,
                vec![
                    person("DISTANCE_1", None),
                    person("DISTANCE_2", Some("broken")),
                ],
            ),
            &mut results,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        state.ensure_complete(1).unwrap();
        let mut state = SearchState::new(0, None);
        for n in 0..Linkedin::MAX_REPEATED_REQUESTS {
            state
                .update(
                    &dto::Paging {
                        start: n * 10,
                        count: 10,
                        total: Linkedin::MAX_REPEATED_REQUESTS * 10,
                    },
                    1,
                )
                .unwrap();
        }
        state.ensure_complete(200).unwrap();
        assert!(SearchState::new(0, Some(0)).ensure_complete(0).is_ok());
        assert!(!SearchState::new(usize::MAX, None).should_stop(0));
    }
}
