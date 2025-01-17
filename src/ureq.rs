use crate::{
    client::{Backend, BackendResponse, Client, RequestParts},
    errors::{CommonError, Error, ErrorPayload},
    HttpUrl,
};
use http::header::{HeaderMap, HeaderName, HeaderValue};

/// A synchronous client backed by [`ureq`]
pub type UreqClient = Client<ureq::Agent>;

impl Backend for ureq::Agent {
    type Request = ureq::Request;
    type Response = ureq::Response;
    type Error = ureq::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        let mut req = self.request_url(r.method.as_str(), r.url.as_url());
        for (k, v) in &r.headers {
            if let Ok(s) = v.to_str() {
                req = req.set(k.as_str(), s);
            }
        }
        if let Some(d) = r.timeout {
            req = req.timeout(d);
        }
        req
    }

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        match r.send(body) {
            Ok(resp) => Ok(resp),
            Err(ureq::Error::Status(_, resp)) => Ok(resp),
            Err(e) => Err(e),
        }
    }
}

impl BackendResponse for ureq::Response {
    fn url(&self) -> HttpUrl {
        self.get_url()
            .parse::<HttpUrl>()
            .expect("response URL should be a valid HTTP URL")
    }

    fn status(&self) -> http::status::StatusCode {
        http::status::StatusCode::from_u16(self.status())
            .expect("response status should be in valid range")
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for name in self.headers_names() {
            let Ok(hname) = name.parse::<HeaderName>() else {
                continue;
            };
            for value in self.all(&name) {
                let Ok(hvalue) = value.parse::<HeaderValue>() else {
                    continue;
                };
                headers.append(hname.clone(), hvalue);
            }
        }
        headers
    }

    fn body_reader(self) -> impl std::io::Read {
        self.into_reader()
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
