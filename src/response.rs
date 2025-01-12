use crate::Method;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseParts {
    initial_url: Url,
    url: Url,
    method: Method,
    status: http::status::StatusCode,
    headers: http::header::HeaderMap,
}

impl ResponseParts {
    pub fn initial_url(&self) -> &Url {
        &self.initial_url
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn status(&self) -> http::status::StatusCode {
        self.status
    }

    pub fn headers(&self) -> &http::header::HeaderMap {
        &self.headers
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response<T> {
    parts: ResponseParts,
    body: T,
}

impl<T> Response<T> {
    pub fn initial_url(&self) -> &Url {
        self.parts.initial_url()
    }

    pub fn url(&self) -> &Url {
        self.parts.url()
    }

    pub fn method(&self) -> Method {
        self.parts.method()
    }

    pub fn status(&self) -> http::status::StatusCode {
        self.parts.status()
    }

    pub fn headers(&self) -> &http::header::HeaderMap {
        self.parts.headers()
    }

    pub fn body_ref(&self) -> &T {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    pub fn into_body(self) -> T {
        self.body
    }

    pub fn into_parts(self) -> (ResponseParts, T) {
        (self.parts, self.body)
    }

    pub fn from_parts(parts: ResponseParts, body: T) -> Response<T> {
        Response { parts, body }
    }
}
