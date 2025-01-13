#[cfg(feature = "tokio")]
pub mod tokio;

use crate::{
    consts::{
        API_VERSION_HEADER, DEFAULT_ACCEPT, DEFAULT_API_URL, DEFAULT_API_VERSION,
        DEFAULT_USER_AGENT,
    },
    errors::{Error, ErrorPayload, ErrorResponseParser},
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientConfig {
    base_url: HttpUrl,
    headers: HeaderMap,
    timeout: Option<Duration>,
    // TODO: mutation delay and retry config
}

impl ClientConfig {
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

    pub fn with_base_url(mut self, url: HttpUrl) -> Self {
        self.base_url = url;
        self
    }

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

    pub fn with_user_agent(mut self, value: HeaderValue) -> Self {
        self.headers.insert(http::header::USER_AGENT, value);
        self
    }

    pub fn with_accept(mut self, value: HeaderValue) -> Self {
        self.headers.insert(http::header::ACCEPT, value);
        self
    }

    pub fn with_api_version(mut self, value: HeaderValue) -> Self {
        self.headers.insert(API_VERSION_HEADER, value);
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_backend<B>(self, backend: B) -> Client<B> {
        Client {
            config: self,
            backend,
        }
    }

    #[cfg(feature = "tokio")]
    pub fn with_async_backend<B>(self, backend: B) -> AsyncClient<B> {
        AsyncClient {
            config: self,
            backend,
        }
    }

    #[cfg(feature = "ureq")]
    pub fn with_ureq(self) -> crate::ureq::UreqClient {
        self.with_backend(ureq::AgentBuilder::new().build())
    }

    #[cfg(feature = "reqwest")]
    pub fn with_reqwest(self) -> crate::reqwest::ReqwestClient {
        self.with_async_backend(reqwest::Client::default())
    }

    // PRIVATE
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

    // PRIVATE
    #[cfg(feature = "tokio")]
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
        // TODO: Mutation delay
        // TODO: Retrying
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
}
