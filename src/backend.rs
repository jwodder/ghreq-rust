use crate::Method;
use std::future::Future;
use std::time::Duration;
use url::Url;

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
    fn url(&self) -> &Url;
    fn status(&self) -> http::status::StatusCode;
    fn headers(&self) -> &http::header::HeaderMap;
    fn body_reader(self) -> impl std::io::Read;
}

pub trait AsyncBackend {
    type Request;
    type Response: BackendResponse;
    type Error;

    // TODO: Should this be fallible?
    fn prepare_request(&self, r: RequestParts) -> Self::Request;

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>>;
}

pub trait AsyncBackendResponse {
    fn url(&self) -> &Url;
    fn status(&self) -> http::status::StatusCode;
    fn headers(&self) -> &http::header::HeaderMap;
    fn body_reader(self) -> impl tokio::io::AsyncRead;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedRequest<T> {
    parts: RequestParts,
    body: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestParts {
    pub url: Url,
    pub method: Method,
    pub headers: http::header::HeaderMap,
    pub timeout: Option<Duration>,
}
