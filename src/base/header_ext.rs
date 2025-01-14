use crate::{util::get_page_number, HttpUrl};
use mime::{Mime, JSON};

pub trait HeaderMapExt {
    fn content_type_is_json(&self) -> bool;
    fn content_length(&self) -> Option<u64>;
    fn set_content_length(&mut self, length: u64);
    fn pagination_links(&self) -> PaginationLinks;
}

impl HeaderMapExt for http::header::HeaderMap {
    fn content_type_is_json(&self) -> bool {
        self.get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<Mime>().ok())
            .is_some_and(|ct| {
                ct.type_() == "application" && (ct.subtype() == "json" || ct.suffix() == Some(JSON))
            })
    }

    fn content_length(&self) -> Option<u64> {
        self.get(http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
    }

    fn set_content_length(&mut self, length: u64) {
        self.insert(
            http::header::CONTENT_LENGTH,
            length
                .to_string()
                .parse()
                .expect("integer string should be a valid HeaderValue"),
        );
    }

    fn pagination_links(&self) -> PaginationLinks {
        let Some(mut links) = self
            .get(http::header::LINK)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| parse_link_header::parse_with_rel(v).ok())
        else {
            return PaginationLinks::default();
        };
        PaginationLinks {
            first: links
                .remove("first")
                .and_then(|lnk| HttpUrl::try_from(lnk.uri).ok()),
            prev: links
                .remove("prev")
                .and_then(|lnk| HttpUrl::try_from(lnk.uri).ok()),
            next: links
                .remove("next")
                .and_then(|lnk| HttpUrl::try_from(lnk.uri).ok()),
            last: links
                .remove("last")
                .and_then(|lnk| HttpUrl::try_from(lnk.uri).ok()),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PaginationLinks {
    pub first: Option<HttpUrl>,
    pub prev: Option<HttpUrl>,
    pub next: Option<HttpUrl>,
    pub last: Option<HttpUrl>,
}

impl PaginationLinks {
    pub fn first_page_number(&self) -> Option<u64> {
        self.first.as_ref().and_then(get_page_number)
    }

    pub fn prev_page_number(&self) -> Option<u64> {
        self.prev.as_ref().and_then(get_page_number)
    }

    pub fn next_page_number(&self) -> Option<u64> {
        self.next.as_ref().and_then(get_page_number)
    }

    pub fn last_page_number(&self) -> Option<u64> {
        self.last.as_ref().and_then(get_page_number)
    }
}
