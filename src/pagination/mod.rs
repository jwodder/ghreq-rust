#[cfg(feature = "tokio")]
mod stream;
#[cfg(feature = "tokio")]
pub use stream::*;

use crate::{
    Endpoint, HeaderMapExt, HttpUrl, Method,
    client::{Backend, Client},
    errors::CommonError,
    parser::ResponseParser,
    request::Request,
    response::ResponseParts,
    util::get_page_number,
};
use http::header::HeaderMap;
use serde::{Deserialize, de::DeserializeOwned};
use std::marker::PhantomData;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(bound = "T: DeserializeOwned", try_from = "RawPage<T>")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total_count: Option<u64>,
    pub incomplete_results: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum RawPage<T> {
    Array(Vec<T>),
    Map(std::collections::HashMap<String, MapPageValue<T>>),
}

impl<T: DeserializeOwned> TryFrom<RawPage<T>> for Page<T> {
    type Error = ParsePageError;

    fn try_from(value: RawPage<T>) -> Result<Page<T>, ParsePageError> {
        match value {
            RawPage::Array(items) => Ok(Page {
                items,
                total_count: None,
                incomplete_results: None,
            }),
            RawPage::Map(map) => {
                let total_count = map.get("total_count").and_then(MapPageValue::as_u64);
                let incomplete_results = map
                    .get("incomplete_results")
                    .and_then(MapPageValue::as_bool);
                let mut lists = map
                    .into_values()
                    .filter_map(MapPageValue::into_list)
                    .collect::<Vec<_>>();
                if lists.len() == 1 {
                    let Some(items) = lists.pop() else {
                        unreachable!("Vec with 1 item should have something to pop");
                    };
                    Ok(Page {
                        items,
                        total_count,
                        incomplete_results,
                    })
                } else {
                    Err(ParsePageError::ListQty(lists.len()))
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum MapPageValue<T> {
    Count(u64),
    Bool(bool),
    List(Vec<T>),
    Other(serde::de::IgnoredAny),
}

impl<T> MapPageValue<T> {
    fn as_u64(&self) -> Option<u64> {
        if let MapPageValue::Count(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    fn as_bool(&self) -> Option<bool> {
        if let MapPageValue::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    fn into_list(self) -> Option<Vec<T>> {
        if let MapPageValue::List(lst) = self {
            Some(lst)
        } else {
            None
        }
    }
}

#[derive(Debug, Error)]
enum ParsePageError {
    #[error("expected exactly one array of items in map page response, got {0}")]
    ListQty(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationInfo {
    // When this is None, you're either on the first page (for most endpoints)
    // or the pagination doesn't use page numbers (e.g., /repositories).
    pub current_page: Option<u64>,
    pub last_page: Option<u64>,
    pub total_count: Option<u64>,
    pub incomplete_results: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageResponse<T> {
    pub next_url: Option<HttpUrl>,
    pub items: Vec<T>,
    pub info: PaginationInfo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageParser<T> {
    next_url: Option<HttpUrl>,
    info: Option<PaginationInfo>,
    buf: Vec<u8>,
    _items: PhantomData<T>,
}

impl<T> PageParser<T> {
    pub fn new() -> PageParser<T> {
        PageParser {
            next_url: None,
            info: None,
            buf: Vec::new(),
            _items: PhantomData,
        }
    }
}

impl<T> Default for PageParser<T> {
    fn default() -> PageParser<T> {
        PageParser::new()
    }
}

impl<T: DeserializeOwned> ResponseParser for PageParser<T> {
    type Output = PageResponse<T>;
    type Error = CommonError;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        let links = parts.headers().pagination_links();
        let current_page = get_page_number(parts.url());
        let last_page = links.last_page_number();
        self.info = Some(PaginationInfo {
            current_page,
            last_page,
            total_count: None,
            incomplete_results: None,
        });
        self.next_url = links.next;
        self.buf.handle_parts(parts);
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.buf.handle_bytes(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        let page = serde_json::from_slice::<Page<T>>(&self.buf)?;
        let mut info = self.info.expect("handle_parts() should have been called");
        info.total_count = page.total_count;
        info.incomplete_results = page.incomplete_results;
        Ok(PageResponse {
            next_url: self.next_url,
            info,
            items: page.items,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRequest<T> {
    endpoint: Endpoint,
    params: Vec<(String, String)>,
    headers: HeaderMap,
    timeout: Option<Duration>,
    _items: PhantomData<T>,
}

impl<T> PageRequest<T> {
    pub fn new(endpoint: Endpoint) -> PageRequest<T> {
        PageRequest {
            endpoint,
            params: Vec::new(),
            headers: HeaderMap::new(),
            timeout: None,
            _items: PhantomData,
        }
    }

    pub fn with_params(mut self, params: Vec<(String, String)>) -> Self {
        self.params = params;
        self
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_page_number(mut self, page: u64) -> Self {
        self.params.push(("page".into(), page.to_string()));
        self
    }
}

impl<T: DeserializeOwned + Send> Request for PageRequest<T> {
    type Output = PageResponse<T>;
    type Error = CommonError;
    type Body = ();

    fn endpoint(&self) -> Endpoint {
        self.endpoint.clone()
    }

    fn method(&self) -> Method {
        Method::Get
    }

    fn headers(&self) -> HeaderMap {
        self.headers.clone()
    }

    fn params(&self) -> Vec<(String, String)> {
        self.params.clone()
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    fn body(&self) -> Self::Body {}

    fn parser(
        &self,
    ) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send {
        PageParser::new()
    }
}

pub trait PaginationRequest {
    type Item: DeserializeOwned + Send;

    fn endpoint(&self) -> Endpoint;

    fn params(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    fn headers(&self) -> HeaderMap {
        HeaderMap::new()
    }

    // Timeout for each request, not for the whole pagination session
    fn timeout(&self) -> Option<Duration> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct PaginationIter<'a, B, R: PaginationRequest> {
    client: &'a Client<B>,
    req: R,
    next_url: Option<Endpoint>,
    info: Option<PaginationInfo>,
    items: Option<std::vec::IntoIter<R::Item>>,
    state: PaginationState,
}

impl<'a, B, R: PaginationRequest> PaginationIter<'a, B, R> {
    pub fn new(client: &'a Client<B>, req: R) -> Self {
        let next_url = Some(req.endpoint());
        PaginationIter {
            client,
            req,
            next_url,
            info: None,
            items: None,
            state: PaginationState::NotStarted,
        }
    }

    pub fn info(&self) -> Option<PaginationInfo> {
        self.info
    }

    pub fn state(&self) -> PaginationState {
        self.state
    }
}

impl<B, R> Iterator for PaginationIter<'_, B, R>
where
    B: Backend,
    R: PaginationRequest<Item: DeserializeOwned + Send>,
{
    type Item = Result<R::Item, crate::errors::Error<B::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.items.as_mut().and_then(Iterator::next) {
                return Some(Ok(item));
            }
            if let Some(url) = self.next_url.as_ref() {
                let mut req = PageRequest::new(url.clone())
                    .with_headers(self.req.headers())
                    .with_timeout(self.req.timeout());
                if self.state == PaginationState::NotStarted {
                    req = req.with_params(self.req.params());
                }
                let page_resp = match self.client.request(req) {
                    Ok(r) => r,
                    Err(e) => {
                        self.next_url = None;
                        self.state = PaginationState::Ended;
                        self.items = None;
                        self.info = None;
                        return Some(Err(e));
                    }
                };
                self.state = PaginationState::Paging;
                self.next_url = page_resp.next_url.map(Into::into);
                self.items = Some(page_resp.items.into_iter());
                self.info = Some(page_resp.info);
            } else {
                self.state = PaginationState::Ended;
                self.items = None;
                self.info = None;
                return None;
            }
        }
    }
}

impl<B, R> std::iter::FusedIterator for PaginationIter<'_, B, R>
where
    B: Backend,
    R: PaginationRequest<Item: DeserializeOwned + Send>,
{
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PaginationState {
    NotStarted,
    Paging,
    Ended,
}

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
                    total_count: None,
                    incomplete_results: None,
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
                    total_count: Some(17),
                    incomplete_results: None,
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
                    total_count: None,
                    incomplete_results: None,
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
                    total_count: Some(17),
                    incomplete_results: None,
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
                    total_count: Some(17),
                    incomplete_results: None,
                }
            );
        }

        #[test]
        fn from_map_extra_item_list_field() {
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
                "more_widgets": [
                    {
                        "name": "Gidget",
                        "color": "chartreuse",
                        "power": 23
                    }
                ],
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
                    total_count: Some(100),
                    incomplete_results: Some(true),
                }
            );
        }
    }
}
