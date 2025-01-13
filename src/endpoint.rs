use crate::HttpUrl;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Url(HttpUrl),
    Path(Vec<String>),
}

impl From<HttpUrl> for Endpoint {
    fn from(value: HttpUrl) -> Endpoint {
        Endpoint::Url(value)
    }
}

impl<S: Into<String>> FromIterator<S> for Endpoint {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        Endpoint::Path(iter.into_iter().map(Into::into).collect())
    }
}
