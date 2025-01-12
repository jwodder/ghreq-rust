use mime::{Mime, JSON};
use url::Url;

pub trait HeaderMapExt {
    fn content_type_is_json(&self) -> bool;
    fn content_length(&self) -> Option<u64>;
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

    fn pagination_links(&self) -> PaginationLinks {
        let Some(mut links) = self
            .get(http::header::LINK)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| parse_link_header::parse_with_rel(v).ok())
        else {
            return PaginationLinks::default();
        };
        PaginationLinks {
            first: links.remove("first").map(|lnk| lnk.uri),
            prev: links.remove("prev").map(|lnk| lnk.uri),
            next: links.remove("next").map(|lnk| lnk.uri),
            last: links.remove("last").map(|lnk| lnk.uri),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PaginationLinks {
    pub first: Option<Url>,
    pub prev: Option<Url>,
    pub next: Option<Url>,
    pub last: Option<Url>,
}
