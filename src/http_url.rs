use crate::Endpoint;
use serde::{
    de::{Deserializer, Error},
    Deserialize,
};
use std::fmt;
use thiserror::Error;
use url::Url;

/// A wrapper around [`url::Url`] that enforces a scheme of "http" or "https"
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HttpUrl(Url);

impl HttpUrl {
    /// Return the URL as a string
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Return a reference to the underlying [`url::Url`]
    pub fn as_url(&self) -> &Url {
        &self.0
    }

    /// Append the given path segment to this URL's path component.
    ///
    /// If the URL does not end with a forward slash, one will be appended, and
    /// then the segment will be added after that.
    pub fn push<S: AsRef<str>>(&mut self, segment: S) -> &mut Self {
        {
            let Ok(mut ps) = self.0.path_segments_mut() else {
                unreachable!("HTTP(S) URLs should always be able to be a base");
            };
            ps.pop_if_empty().push(segment.as_ref());
        }
        self
    }

    /// Append the given path segments to this URL's path component.
    ///
    /// If the URL does not end with a forward slash, one will be appended, and
    /// then the segments will be added after that.
    pub fn extend<I>(&mut self, segments: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        {
            let Ok(mut ps) = self.0.path_segments_mut() else {
                unreachable!("HTTP(S) URLs should always be able to be a base");
            };
            ps.pop_if_empty().extend(segments);
        }
        self
    }

    /// Append a trailing forward slash to the URL if it does not already end
    /// with one
    pub fn ensure_dirpath(&mut self) -> &mut Self {
        {
            let Ok(mut ps) = self.0.path_segments_mut() else {
                unreachable!("HTTP(S) URLs should always be able to be a base");
            };
            ps.pop_if_empty().push("");
        }
        self
    }

    pub fn join_endpoint(&self, endpoint: Endpoint) -> HttpUrl {
        match endpoint {
            Endpoint::Url(url) => url,
            Endpoint::Path(path) => {
                let mut url = self.clone();
                url.extend(path);
                url
            }
        }
    }

    /// Append `"{key}={value}"` (after percent-encoding) to the URL's query
    /// parameters
    pub fn append_query_param(&mut self, key: &str, value: &str) -> &mut Self {
        self.0.query_pairs_mut().append_pair(key, value);
        self
    }
}

impl From<HttpUrl> for Url {
    fn from(value: HttpUrl) -> Url {
        value.0
    }
}

impl TryFrom<Url> for HttpUrl {
    type Error = NotHttpError;

    fn try_from(value: Url) -> Result<HttpUrl, NotHttpError> {
        if matches!(value.scheme(), "http" | "https") {
            Ok(HttpUrl(value))
        } else {
            Err(NotHttpError)
        }
    }
}

impl fmt::Display for HttpUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for HttpUrl {
    type Err = ParseHttpUrlError;

    fn from_str(s: &str) -> Result<HttpUrl, ParseHttpUrlError> {
        let url = s.parse::<Url>()?;
        if matches!(url.scheme(), "http" | "https") {
            Ok(HttpUrl(url))
        } else {
            Err(ParseHttpUrlError::BadScheme)
        }
    }
}

impl<'de> Deserialize<'de> for HttpUrl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let url = Url::deserialize(deserializer)?;
        if matches!(url.scheme(), "http" | "https") {
            Ok(HttpUrl(url))
        } else {
            Err(D::Error::custom("expected URL with HTTP(S) scheme"))
        }
    }
}

/// Error returned by [`HttpUrl`]'s `FromStr` implementation
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ParseHttpUrlError {
    /// The string was a valid URL, but the scheme was neither HTTP nor HTTPS
    #[error(r#"URL scheme must be "http" or "https""#)]
    BadScheme,

    /// The string was not a valid URL
    #[error(transparent)]
    Url(#[from] url::ParseError),
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error(r#"URL scheme must be "http" or "https""#)]
pub struct NotHttpError;

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("foo#bar", "https://api.github.com/base/foo%23bar")]
    #[case("foo%bar", "https://api.github.com/base/foo%25bar")]
    #[case("foo/bar", "https://api.github.com/base/foo%2Fbar")]
    #[case("foo?bar", "https://api.github.com/base/foo%3Fbar")]
    fn push_special_chars(#[case] path: &str, #[case] expected: &str) {
        let mut base = "https://api.github.com/base".parse::<HttpUrl>().unwrap();
        base.push(path);
        assert_eq!(base.as_str(), expected);
    }

    #[rstest]
    #[case(&["foo"], "https://api.github.com/foo")]
    #[case(&["foo", "bar"], "https://api.github.com/foo/bar")]
    fn extend_nopath(
        #[values("https://api.github.com", "https://api.github.com/")] mut base: HttpUrl,
        #[case] segments: &[&str],
        #[case] expected: &str,
    ) {
        base.extend(segments);
        assert_eq!(base.as_str(), expected);
    }

    #[rstest]
    #[case(&["gnusto"], "https://api.github.com/foo/bar/gnusto")]
    #[case(&["gnusto", "cleesh"], "https://api.github.com/foo/bar/gnusto/cleesh")]
    fn extend_path(
        #[values("https://api.github.com/foo/bar", "https://api.github.com/foo/bar/")]
        mut base: HttpUrl,
        #[case] segments: &[&str],
        #[case] expected: &str,
    ) {
        base.extend(segments);
        assert_eq!(base.as_str(), expected);
    }

    #[rstest]
    #[case("https://api.github.com", "https://api.github.com/")]
    #[case("https://api.github.com/", "https://api.github.com/")]
    #[case("https://api.github.com/foo", "https://api.github.com/foo/")]
    #[case("https://api.github.com/foo/", "https://api.github.com/foo/")]
    fn ensure_dirpath(#[case] mut before: HttpUrl, #[case] after: &str) {
        before.ensure_dirpath();
        assert_eq!(before.as_str(), after);
    }

    #[test]
    fn append_query_param() {
        let mut url = "https://api.github.com/foo".parse::<HttpUrl>().unwrap();
        assert_eq!(url.as_str(), "https://api.github.com/foo");
        url.append_query_param("bar", "baz");
        assert_eq!(url.as_str(), "https://api.github.com/foo?bar=baz");
        url.append_query_param("quux", "with space");
        assert_eq!(
            url.as_str(),
            "https://api.github.com/foo?bar=baz&quux=with+space"
        );
        url.append_query_param("bar", "rod");
        assert_eq!(
            url.as_str(),
            "https://api.github.com/foo?bar=baz&quux=with+space&bar=rod"
        );
    }
}
