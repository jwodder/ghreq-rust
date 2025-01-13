use crate::HttpUrl;

pub(crate) fn get_page_number(url: &HttpUrl) -> Option<u64> {
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
