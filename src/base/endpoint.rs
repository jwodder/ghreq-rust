use crate::HttpUrl;

/// A description of an API endpoint to make a request to.
///
/// This can be either a complete URL or a sequence of path components to
/// append to the client's base API URL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Endpoint {
    /// A complete HTTP(S) URL.
    ///
    /// When an `Endpoint` of this type is used to make a request, the given
    /// URL is used as-is for the request.
    Url(HttpUrl),

    /// A sequence of path components.
    ///
    /// When an `Endpoint` of this type is used to make a request, the path
    /// components are appended to the client object's base API URL with
    /// [`url::PathSegmentsMut::extend`].
    Path(Vec<String>),
}

impl From<HttpUrl> for Endpoint {
    fn from(value: HttpUrl) -> Endpoint {
        Endpoint::Url(value)
    }
}

impl<S: Into<String>> FromIterator<S> for Endpoint {
    /// Convert an iterator of path component strings into an `Endpoint`
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        Endpoint::Path(iter.into_iter().map(Into::into).collect())
    }
}
