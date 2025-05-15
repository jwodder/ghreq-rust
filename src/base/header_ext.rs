use crate::{util::get_page_number, HttpUrl};
use mime::{Mime, JSON};

/// Additional utility methods added to [`http::header::HeaderMap`]
pub trait HeaderMapExt {
    /// Returns true if the headers contain a `Content-Type` header with a
    /// value of "application/json" or "application/{something}+json".
    fn content_type_is_json(&self) -> bool;

    /// Returns the value of the `Content-Length` header as a `u64`.  Returns
    /// `None` if the header is not set or the value could not be parsed into a
    /// `u64`.
    fn content_length(&self) -> Option<u64>;

    /// Set the value of the `Content-Length` header to the given integer value.
    fn set_content_length(&mut self, length: u64);

    /// Parse the value of the `Link` header and return the links with
    /// `rel` types of "first", "prev", "next", and "last".  If there is no
    /// `Link` header or it could not be parsed, all fields in the returned
    /// structure are `None`.
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

    #[allow(clippy::return_and_then)]
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

/// A set of pagination-related URLs parsed from a `Link` header
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PaginationLinks {
    /// The link with `rel` link type "first", if any
    pub first: Option<HttpUrl>,

    /// The link with `rel` link type "prev", if any
    pub prev: Option<HttpUrl>,

    /// The link with `rel` link type "next", if any
    pub next: Option<HttpUrl>,

    /// The link with `rel` link type "last", if any
    pub last: Option<HttpUrl>,
}

impl PaginationLinks {
    /// Extracts the value of the `page` query parameter from
    /// [`PaginationLinks::first`].
    ///
    /// Returns `None` if the field is `None`, if there is no `page` parameter,
    /// or if the value could not be parsed into a `u64`.
    pub fn first_page_number(&self) -> Option<u64> {
        self.first.as_ref().and_then(get_page_number)
    }

    /// Extracts the value of the `page` query parameter from
    /// [`PaginationLinks::prev`].
    ///
    /// Returns `None` if the field is `None`, if there is no `page` parameter,
    /// or if the value could not be parsed into a `u64`.
    pub fn prev_page_number(&self) -> Option<u64> {
        self.prev.as_ref().and_then(get_page_number)
    }

    /// Extracts the value of the `page` query parameter from
    /// [`PaginationLinks::next`].
    ///
    /// Returns `None` if the field is `None`, if there is no `page` parameter,
    /// or if the value could not be parsed into a `u64`.
    pub fn next_page_number(&self) -> Option<u64> {
        self.next.as_ref().and_then(get_page_number)
    }

    /// Extracts the value of the `page` query parameter from
    /// [`PaginationLinks::last`].
    ///
    /// Returns `None` if the field is `None`, if there is no `page` parameter,
    /// or if the value could not be parsed into a `u64`.
    pub fn last_page_number(&self) -> Option<u64> {
        self.last.as_ref().and_then(get_page_number)
    }
}
