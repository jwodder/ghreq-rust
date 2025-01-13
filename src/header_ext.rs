use crate::HttpUrl;
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

fn get_page_number(url: &HttpUrl) -> Option<u64> {
    // Experimentation on 2025-01-13 indicates that, when making a paginated
    // request to GitHub with one or more "page" query parameters, the server
    // honors only the last such parameter, and if it's not a number, it's
    // discarded.
    url.as_url()
        .query_pairs()
        .filter_map(|(k, v)| (k == "page").then_some(v))
        .last()
        .and_then(|v| v.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("https://api.github.com/users/jwodder/repos", None)]
    #[case("https://api.github.com/users/jwodder/repos?per_page=100", None)]
    #[case("https://api.github.com/users/jwodder/repos?page=3", Some(3))]
    #[case(
        "https://api.github.com/users/jwodder/repos?per_page=100&page=3&flavor=vanilla",
        Some(3)
    )]
    #[case("https://api.github.com/users/jwodder/repos?page=3&page=4", Some(4))]
    #[case("https://api.github.com/users/jwodder/repos?page=three", None)]
    #[case(
        "https://api.github.com/users/jwodder/repos?page=three&page=4",
        Some(4)
    )]
    #[case("https://api.github.com/users/jwodder/repos?page=3&page=four", None)]
    fn test_get_page_number(#[case] url: HttpUrl, #[case] num: Option<u64>) {
        assert_eq!(get_page_number(&url), num);
    }
}
