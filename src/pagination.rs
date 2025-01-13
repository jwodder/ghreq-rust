use serde::{de::DeserializeOwned, Deserialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(bound = "T: DeserializeOwned", try_from = "RawPage<T>")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: Option<u64>,
    pub incomplete: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
enum RawPage<T> {
    Array(Vec<T>),
    Map(serde_json::Map<String, serde_json::Value>),
}

impl<T: DeserializeOwned> TryFrom<RawPage<T>> for Page<T> {
    type Error = ParsePageError;

    fn try_from(value: RawPage<T>) -> Result<Page<T>, ParsePageError> {
        match value {
            RawPage::Array(items) => Ok(Page {
                items,
                total: None,
                incomplete: None,
            }),
            RawPage::Map(map) => {
                let total = map
                    .get("total_count")
                    .and_then(|v| v.as_number())
                    .and_then(serde_json::Number::as_u64);
                let incomplete = map
                    .get("incomplete_results")
                    .and_then(serde_json::Value::as_bool);
                let mut lists = map
                    .into_values()
                    .filter(serde_json::Value::is_array)
                    .collect::<Vec<_>>();
                if lists.len() == 1 {
                    let Some(lst) = lists.pop() else {
                        unreachable!("Vec with 1 item should have something to pop");
                    };
                    match serde_json::from_value::<Vec<T>>(lst) {
                        Ok(items) => Ok(Page {
                            items,
                            total,
                            incomplete,
                        }),
                        Err(e) => Err(ParsePageError::DeserList(e)),
                    }
                } else {
                    Err(ParsePageError::ListQty(lists.len()))
                }
            }
        }
    }
}

#[derive(Debug, Error)]
enum ParsePageError {
    #[error("expected exactly one array field in map page response, got {0} array fields")]
    ListQty(usize),

    #[error("failed to deserialize an element of array field in map page response")]
    DeserList(#[source] serde_json::Error),
}

// PaginationResponse
// PaginationResponseParser
// PaginationRequest

#[cfg(test)]
mod tests {
    use super::*;

    mod deser_page {
        use super::*;
        use indoc::indoc;

        #[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
        struct Widget {
            name: String,
            color: String,
            power: u64,
        }

        #[test]
        fn from_list() {
            let src = indoc! {r#"
            [
                {
                    "name": "Steve",
                    "color": "aquamarine",
                    "power": 9001
                },
                {
                    "name": "Widget O'Malley",
                    "color": "taupe",
                    "power": 42
                }
            ]
            "#};
            let page = serde_json::from_str::<Page<Widget>>(src).unwrap();
            assert_eq!(
                page,
                Page {
                    items: vec![
                        Widget {
                            name: "Steve".into(),
                            color: "aquamarine".into(),
                            power: 9001,
                        },
                        Widget {
                            name: "Widget O'Malley".into(),
                            color: "taupe".into(),
                            power: 42,
                        },
                    ],
                    total: None,
                    incomplete: None,
                }
            );
        }

        #[test]
        fn from_map() {
            let src = indoc! {r#"
            {
                "total_count": 17,
                "widgets": [
                    {
                        "name": "Steve",
                        "color": "aquamarine",
                        "power": 9001
                    },
                    {
                        "name": "Widget O'Malley",
                        "color": "taupe",
                        "power": 42
                    }
                ]
            }
            "#};
            let page = serde_json::from_str::<Page<Widget>>(src).unwrap();
            assert_eq!(
                page,
                Page {
                    items: vec![
                        Widget {
                            name: "Steve".into(),
                            color: "aquamarine".into(),
                            power: 9001,
                        },
                        Widget {
                            name: "Widget O'Malley".into(),
                            color: "taupe".into(),
                            power: 42,
                        },
                    ],
                    total: Some(17),
                    incomplete: None,
                }
            );
        }

        #[test]
        fn from_map_no_total() {
            let src = indoc! {r#"
            {
                "widgets": [
                    {
                        "name": "Steve",
                        "color": "aquamarine",
                        "power": 9001
                    },
                    {
                        "name": "Widget O'Malley",
                        "color": "taupe",
                        "power": 42
                    }
                ]
            }
            "#};
            let page = serde_json::from_str::<Page<Widget>>(src).unwrap();
            assert_eq!(
                page,
                Page {
                    items: vec![
                        Widget {
                            name: "Steve".into(),
                            color: "aquamarine".into(),
                            power: 9001,
                        },
                        Widget {
                            name: "Widget O'Malley".into(),
                            color: "taupe".into(),
                            power: 42,
                        },
                    ],
                    total: None,
                    incomplete: None,
                }
            );
        }

        #[test]
        fn from_map_extra_field() {
            let src = indoc! {r#"
            {
                "total_count": 17,
                "widgets": [
                    {
                        "name": "Steve",
                        "color": "aquamarine",
                        "power": 9001
                    },
                    {
                        "name": "Widget O'Malley",
                        "color": "taupe",
                        "power": 42
                    }
                ],
                "mode": "ponens"
            }
            "#};
            let page = serde_json::from_str::<Page<Widget>>(src).unwrap();
            assert_eq!(
                page,
                Page {
                    items: vec![
                        Widget {
                            name: "Steve".into(),
                            color: "aquamarine".into(),
                            power: 9001,
                        },
                        Widget {
                            name: "Widget O'Malley".into(),
                            color: "taupe".into(),
                            power: 42,
                        },
                    ],
                    total: Some(17),
                    incomplete: None,
                }
            );
        }

        #[test]
        fn from_map_extra_list_field() {
            let src = indoc! {r#"
            {
                "total_count": 17,
                "widgets": [
                    {
                        "name": "Steve",
                        "color": "aquamarine",
                        "power": 9001
                    },
                    {
                        "name": "Widget O'Malley",
                        "color": "taupe",
                        "power": 42
                    }
                ],
                "modes": ["ponens", "tollens"]
            }
            "#};
            assert!(serde_json::from_str::<Page<Widget>>(src).is_err());
        }

        #[test]
        fn from_map_extra_no_list_field() {
            let src = indoc! {r#"
            {
                "total_count": 0
            }
            "#};
            assert!(serde_json::from_str::<Page<Widget>>(src).is_err());
        }

        #[test]
        fn from_search_results() {
            let src = indoc! {r#"
            {
                "total_count": 100,
                "incomplete_results": true,
                "items": [
                    {
                        "name": "Steve",
                        "color": "aquamarine",
                        "power": 9001
                    },
                    {
                        "name": "Widget O'Malley",
                        "color": "taupe",
                        "power": 42
                    }
                ]
            }
            "#};
            let page = serde_json::from_str::<Page<Widget>>(src).unwrap();
            assert_eq!(
                page,
                Page {
                    items: vec![
                        Widget {
                            name: "Steve".into(),
                            color: "aquamarine".into(),
                            power: 9001,
                        },
                        Widget {
                            name: "Widget O'Malley".into(),
                            color: "taupe".into(),
                            power: 42,
                        },
                    ],
                    total: Some(100),
                    incomplete: Some(true),
                }
            );
        }
    }
}
