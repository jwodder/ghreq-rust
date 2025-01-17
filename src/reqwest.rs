use crate::{
    client::{
        tokio::{AsyncBackend, AsyncBackendResponse, AsyncClient},
        RequestParts,
    },
    errors::{CommonError, Error, ErrorPayload},
    HttpUrl,
};
use futures_util::TryStreamExt;
use std::future::Future;
use tokio_util::io::{ReaderStream, StreamReader};

/// An asynchronous client backed by [`reqwest`]
pub type ReqwestClient = AsyncClient<reqwest::Client>;

impl AsyncBackend for reqwest::Client {
    type Request = reqwest::RequestBuilder;
    type Response = reqwest::Response;
    type Error = reqwest::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        let mut req = self
            .request(r.method.into(), r.url.as_str())
            .headers(r.headers);
        if let Some(d) = r.timeout {
            req = req.timeout(d);
        }
        req
    }

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static {
        r.body(reqwest::Body::wrap_stream(ReaderStream::new(body)))
            .send()
    }
}

impl AsyncBackendResponse for reqwest::Response {
    fn url(&self) -> HttpUrl {
        HttpUrl::try_from(self.url().clone()).expect("response URL should be a valid HTTP URL")
    }

    fn status(&self) -> http::status::StatusCode {
        self.status()
    }

    fn headers(&self) -> http::header::HeaderMap {
        self.headers().clone()
    }

    fn body_reader(self) -> impl tokio::io::AsyncRead + Send + 'static {
        StreamReader::new(self.bytes_stream().map_err(std::io::Error::other))
    }
}

/// Error type returned by [`ReqwestClient`] methods.
///
/// The `E` parameter is the `Error` type of the input
/// [`Request`][crate::request::Request] provided to a method.
pub type ReqwestError<E = CommonError> = Error<reqwest::Error, E>;

/// Payload of errors returned by [`ReqwestClient`] methods.
///
/// The `E` parameter is the `Error` type of the input
/// [`Request`][crate::request::Request] provided to a method.
pub type ReqwestErrorPayload<E = CommonError> = ErrorPayload<reqwest::Error, E>;
