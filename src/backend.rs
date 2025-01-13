use crate::{HttpUrl, Method};
use std::future::Future;
use std::time::Duration;

pub trait Backend {
    type Request;
    type Response: BackendResponse;
    type Error;

    // TODO: Should this be fallible?
    fn prepare_request(&self, r: RequestParts) -> Self::Request;

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error>;
}

pub trait BackendResponse {
    fn url(&self) -> HttpUrl;
    fn status(&self) -> http::status::StatusCode;
    fn headers(&self) -> http::header::HeaderMap;
    fn body_reader(self) -> impl std::io::Read;
}

pub trait AsyncBackend {
    type Request;
    type Response: AsyncBackendResponse;
    type Error;

    // TODO: Should this be fallible?
    fn prepare_request(&self, r: RequestParts) -> Self::Request;

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>>;
}

pub trait AsyncBackendResponse {
    fn url(&self) -> HttpUrl;
    fn status(&self) -> http::status::StatusCode;
    fn headers(&self) -> http::header::HeaderMap;
    fn body_reader(self) -> impl tokio::io::AsyncRead + Send + 'static;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedRequest<T> {
    parts: RequestParts,
    body: T,
}

impl<T> PreparedRequest<T> {
    pub fn url(&self) -> &HttpUrl {
        &self.parts.url
    }

    pub fn method(&self) -> Method {
        self.parts.method
    }

    pub fn headers(&self) -> &http::header::HeaderMap {
        &self.parts.headers
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

    pub fn into_parts(self) -> (RequestParts, T) {
        (self.parts, self.body)
    }

    pub fn from_parts(parts: RequestParts, body: T) -> PreparedRequest<T> {
        PreparedRequest { parts, body }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestParts {
    pub url: HttpUrl,
    pub method: Method,
    pub headers: http::header::HeaderMap,
    pub timeout: Option<Duration>,
}
