use crate::{
    HttpUrl, Method,
    client::{Backend, BackendResponse, Client, RequestParts},
    errors::{CommonError, Error, ErrorPayload},
};
use http::header::HeaderMap;
use ureq::{ResponseExt, SendBody};

/// A synchronous client backed by [`ureq`]
pub type UreqClient = Client<ureq::Agent>;

impl Backend for ureq::Agent {
    type Request = ureq::RequestBuilder<ureq::typestate::WithBody>;
    type Response = http::Response<ureq::Body>;
    type Error = ureq::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        let mut req = match r.method {
            Method::Get => self.get(r.url).force_send_body(),
            Method::Head => self.head(r.url).force_send_body(),
            Method::Post => self.post(r.url),
            Method::Put => self.put(r.url),
            Method::Patch => self.patch(r.url),
            Method::Delete => self.delete(r.url).force_send_body(),
        };
        for (k, v) in &r.headers {
            req = req.header(k, v);
        }
        if let Some(d) = r.timeout {
            req = req.config().timeout_global(Some(d)).build();
        }
        req.config().http_status_as_error(false).build()
    }

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        mut body: R,
    ) -> Result<Self::Response, Self::Error> {
        r.send(SendBody::from_reader(&mut body))
    }
}

impl BackendResponse for http::Response<ureq::Body> {
    fn url(&self) -> HttpUrl {
        self.get_uri()
            .to_string()
            .parse::<HttpUrl>()
            .expect("response URL should be a valid HTTP URL")
    }

    fn status(&self) -> http::status::StatusCode {
        self.status()
    }

    fn headers(&self) -> HeaderMap {
        self.headers().clone()
    }

    fn body_reader(self) -> impl std::io::Read {
        self.into_body().into_reader()
    }
}

/// Error type returned by [`UreqClient`] methods.
///
/// The `E` parameter is the `Error` type of the input
/// [`Request`][crate::request::Request] provided to a method.
pub type UreqError<E = CommonError> = Error<ureq::Error, E>;

/// Payload of errors returned by [`UreqClient`] methods.
///
/// The `E` parameter is the `Error` type of the input
/// [`Request`][crate::request::Request] provided to a method.
pub type UreqErrorPayload<E = CommonError> = ErrorPayload<ureq::Error, E>;
