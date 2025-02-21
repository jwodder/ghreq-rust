#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

use crate::{
    consts::{
        API_VERSION_HEADER, DEFAULT_ACCEPT, DEFAULT_API_URL, DEFAULT_API_VERSION,
        DEFAULT_USER_AGENT,
    },
    errors::{Error, ErrorPayload, ErrorResponseParser},
    pagination::{PaginationIter, PaginationRequest},
    parser::ResponseParserExt,
    request::{Request, RequestBody},
    response::{Response, ResponseParts},
    HttpUrl, Method,
};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

#[cfg(feature = "tokio")]
use self::tokio::AsyncClient;
#[cfg(feature = "tokio")]
use crate::request::AsyncRequestBody;

/// Configuration for a GitHub REST API client
///
/// Create a `ClientConfig` with [`ClientConfig::new()`], chain calls to zero
/// or more of its `with_*` methods to modify the settings of your choice, and
/// then call [`ClientConfig::with_backend()`] or
/// [`ClientConfig::with_async_backend()`] to combine the configuration with a
/// backend and thereby acquire a [`Client`] or [`AsyncClient`].
#[cfg_attr(
    feature = "ureq",
    doc = r#"

# Example

```
# use ghreq::client::ClientConfig;
# use ghreq::header::HeaderValue;
let client = ClientConfig::new()
    .with_auth_token("hunter2")
    .unwrap()
    .with_user_agent(HeaderValue::from_static("my-custom-client/v1.2.3"))
    .with_backend(ureq::Agent::new_with_defaults());
```
"#
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientConfig {
    base_url: HttpUrl,
    headers: HeaderMap,
    timeout: Option<Duration>,
}

impl ClientConfig {
    /// Create a new `ClientConfig` with default values
    pub fn new() -> ClientConfig {
        fn parse_const_value(value: &str, name: &str) -> HeaderValue {
            match value.parse::<HeaderValue>() {
                Ok(v) => v,
                Err(_) => unreachable!("{name} should be a valid header value"),
            }
        }

        let Ok(base_url) = DEFAULT_API_URL.parse::<HttpUrl>() else {
            unreachable!("DEFAULT_API_URL should be a valid URL");
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::ACCEPT,
            parse_const_value(DEFAULT_ACCEPT, "DEFAULT_ACCEPT"),
        );
        headers.insert(
            API_VERSION_HEADER,
            parse_const_value(DEFAULT_API_VERSION, "DEFAULT_API_VERSION"),
        );
        headers.insert(
            http::header::USER_AGENT,
            parse_const_value(DEFAULT_USER_AGENT, "DEFAULT_USER_AGENT"),
        );
        ClientConfig {
            base_url,
            headers,
            timeout: None,
        }
    }

    /// Set the base API URL for making API requests.
    ///
    /// When the resulting client is given a request whose
    /// [`Endpoint`][crate::Endpoint] is a sequence of path components, those
    /// components will be appended to this URL.
    ///
    /// The default base API URL is given by [`DEFAULT_API_URL`].
    pub fn with_base_url(mut self, url: HttpUrl) -> Self {
        self.base_url = url;
        self
    }

    /// Send the given access token in the "Authorization" header of outgoing
    /// requests.
    ///
    /// By default, no access token is sent.
    ///
    /// # Errors
    ///
    /// If the string `"Bearer {token}"` cannot be parsed into a
    /// [`HeaderValue`], then `Err` is returned, containing the unmodified
    /// `ClientConfig`.
    #[allow(clippy::result_large_err)]
    pub fn with_auth_token(mut self, token: &str) -> Result<Self, Self> {
        let value = format!("Bearer {token}");
        match value.parse::<HeaderValue>() {
            Ok(value) => {
                self.headers.insert(http::header::AUTHORIZATION, value);
                Ok(self)
            }
            Err(_) => Err(self),
        }
    }

    /// Set the value to use for the `User-Agent` header in outgoing requests.
    ///
    /// The default setting is given by [`DEFAULT_USER_AGENT`].
    pub fn with_user_agent(mut self, value: HeaderValue) -> Self {
        self.headers.insert(http::header::USER_AGENT, value);
        self
    }

    /// Set the value to use for the `Accept` header in outgoing requests.
    ///
    /// The default setting is given by [`DEFAULT_ACCEPT`].
    pub fn with_accept(mut self, value: HeaderValue) -> Self {
        self.headers.insert(http::header::ACCEPT, value);
        self
    }

    /// Set the value to use for the `X-GitHub-Api-Version` header in outgoing
    /// requests.
    ///
    /// The default setting is given by [`DEFAULT_API_VERSION`].
    pub fn with_api_version(mut self, value: HeaderValue) -> Self {
        self.headers.insert(API_VERSION_HEADER, value);
        self
    }

    /// Add the given HTTP header & value to all outgoing requests.
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set the request timeout (covering the time from the start of the
    /// connection for a request until the end of the response is received) to
    /// the given duration.
    ///
    /// By default, `ghreq` does not set a timeout, resulting in each backend
    /// using its own timeout value.
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Combine the `ClientConfig` with the given synchronous backend (ideally
    /// an implementor of [`Backend`]) to acquire a synchronous [`Client`].
    pub fn with_backend<B>(self, backend: B) -> Client<B> {
        Client {
            config: self,
            backend,
        }
    }

    /// Combine the `ClientConfig` with the given asynchronous backend (ideally
    /// an implementor of [`AsyncBackend`][self::tokio::AsyncBackend]) to
    /// acquire an asynchronous [`AsyncClient`].
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    pub fn with_async_backend<B>(self, backend: B) -> AsyncClient<B> {
        AsyncClient {
            config: self,
            backend,
        }
    }

    /// Combine the `ClientConfig` with a default [`ureq::Agent`] to acquire an
    /// [`UreqClient`][crate::ureq::UreqClient].
    #[cfg(feature = "ureq")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ureq")))]
    pub fn with_ureq(self) -> crate::ureq::UreqClient {
        self.with_backend(ureq::Agent::new_with_defaults())
    }

    /// Combine the `ClientConfig` with a default [`reqwest::Client`] to
    /// acquire a [`ReqwestClient`][crate::reqwest::ReqwestClient].
    #[cfg(feature = "reqwest")]
    #[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
    pub fn with_reqwest(self) -> crate::reqwest::ReqwestClient {
        self.with_async_backend(reqwest::Client::default())
    }

    /// [Private] Convert a [`Request`] instance into a [`PreparedRequest`]
    /// with a [`std::io::Read`] for a body.
    fn prepare_request<R, BE>(
        &self,
        req: &R,
    ) -> Result<PreparedRequest<impl std::io::Read + 'static>, Error<BE, R::Error>>
    where
        R: Request<Body: RequestBody<Error: Into<R::Error>>>,
    {
        let mut url = self.base_url.join_endpoint(req.endpoint());
        for (name, value) in req.params() {
            url.append_query_param(&name, &value);
        }
        let method = req.method();
        let timeout = req.timeout().or(self.timeout);
        let body = req.body();
        // Set the body headers first so that the Request can override them if
        // it wants
        let mut headers = self.headers.clone();
        headers.extend(body.headers());
        headers.extend(req.headers());
        let parts = RequestParts {
            url: url.clone(),
            method,
            headers,
            timeout,
        };
        let body = match body.into_read() {
            Ok(body) => body,
            Err(e) => {
                let payload = ErrorPayload::PrepareRequest(e.into());
                return Err(Error::new(parts.url, parts.method, payload));
            }
        };
        Ok(PreparedRequest::from_parts(parts, body))
    }

    /// [Private] Convert a [`Request`] instance into a [`PreparedRequest`]
    /// with a [`tokio::io::AsyncRead`] for a body.
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    fn prepare_async_request<R, BE>(
        &self,
        req: &R,
    ) -> Result<PreparedRequest<impl ::tokio::io::AsyncRead + Send + 'static>, Error<BE, R::Error>>
    where
        R: Request<Body: AsyncRequestBody<Error: Into<<R as Request>::Error>>>,
    {
        let mut url = self.base_url.join_endpoint(req.endpoint());
        for (name, value) in req.params() {
            url.append_query_param(&name, &value);
        }
        let method = req.method();
        let timeout = req.timeout().or(self.timeout);
        let body = req.body();
        // Set the body headers first so that the Request can override them if
        // it wants
        let mut headers = self.headers.clone();
        headers.extend(body.headers());
        headers.extend(req.headers());
        let parts = RequestParts {
            url: url.clone(),
            method,
            headers,
            timeout,
        };
        let body = match body.into_async_read() {
            Ok(body) => body,
            Err(e) => {
                let payload = ErrorPayload::PrepareRequest(e.into());
                return Err(Error::new(parts.url, parts.method, payload));
            }
        };
        Ok(PreparedRequest::from_parts(parts, body))
    }
}

impl Default for ClientConfig {
    fn default() -> ClientConfig {
        ClientConfig::new()
    }
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

    pub fn headers(&self) -> &HeaderMap {
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
    pub headers: HeaderMap,
    pub timeout: Option<Duration>,
}

pub trait Backend {
    type Request;
    type Response: BackendResponse;
    type Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request;

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error>;
}

impl<T: Backend + ?Sized> Backend for &T {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (*self).prepare_request(r)
    }

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        (*self).send(r, body)
    }
}

impl<T: Backend + ?Sized> Backend for &mut T {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (**self).prepare_request(r)
    }

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        (**self).send(r, body)
    }
}

impl<T: Backend + ?Sized> Backend for std::sync::Arc<T> {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (**self).prepare_request(r)
    }

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        (**self).send(r, body)
    }
}

impl<T: Backend + ?Sized> Backend for Box<T> {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (**self).prepare_request(r)
    }

    fn send<R: std::io::Read>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        (**self).send(r, body)
    }
}

pub trait BackendResponse {
    fn url(&self) -> HttpUrl;
    fn status(&self) -> http::status::StatusCode;
    fn headers(&self) -> HeaderMap;
    fn body_reader(self) -> impl std::io::Read;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Client<B> {
    config: ClientConfig,
    backend: B,
}

impl<B> Client<B> {
    pub fn new(config: ClientConfig, backend: B) -> Client<B> {
        Client { config, backend }
    }

    pub fn backend_ref(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: Backend> Client<B> {
    pub fn request<R>(&self, req: R) -> Result<R::Output, Error<B::Error, R::Error>>
    where
        R: Request<Body: RequestBody<Error: Into<R::Error>>>,
    {
        let (reqparts, reqbody) = self.config.prepare_request(&req)?.into_parts();
        let initial_url = reqparts.url.clone();
        let method = reqparts.method;
        let backreq = self.backend.prepare_request(reqparts);
        let resp = match self.backend.send(backreq, reqbody) {
            Ok(resp) => resp,
            Err(e) => {
                let payload = ErrorPayload::Send(e);
                return Err(Error::new(initial_url, method, payload));
            }
        };
        let parts = ResponseParts {
            initial_url: initial_url.clone(),
            method,
            url: resp.url(),
            status: resp.status(),
            headers: resp.headers(),
        };
        let body = resp.body_reader();
        let response = Response::from_parts(parts, body);
        if response.status().is_client_error() || response.status().is_server_error() {
            let parser = ErrorResponseParser::new();
            let err_resp = parser.parse_response(response).map_err(|e| {
                Error::new(
                    initial_url.clone(),
                    method,
                    ErrorPayload::ParseResponse(e.convert_parse_error::<R::Error>()),
                )
            })?;
            Err(Error::new(
                initial_url,
                method,
                ErrorPayload::Status(err_resp),
            ))
        } else {
            let parser = req.parser();
            parser.parse_response(response).map_err(|e| {
                Error::new(
                    initial_url,
                    method,
                    ErrorPayload::ParseResponse(e.convert_parse_error()),
                )
            })
        }
    }

    pub fn paginate<R: PaginationRequest>(&self, req: R) -> PaginationIter<'_, B, R> {
        PaginationIter::new(self, req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_new_succeeds() {
        let _ = ClientConfig::new();
    }
}
