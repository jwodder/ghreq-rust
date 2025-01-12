use crate::{
    Backend, BackendResponse, Error, ErrorPayload, HttpUrl, PreparedRequest, Request, RequestBody,
    RequestParts, Response, ResponseParserExt, ResponseParts,
};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

pub static DEFAULT_ACCEPT: &str = "application/vnd.github+json";

/// The name of the HTTP header used by the GitHub REST API to communicate the
/// API version
pub static API_VERSION_HEADER: &str = "X-GitHub-Api-Version";

pub static DEFAULT_API_URL: &str = "https://api.github.com";

pub static DEFAULT_API_VERSION: &str = "2022-11-28";

pub static DEFAULT_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")",
);

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

    pub fn set_base_url(&mut self, url: HttpUrl) {
        self.base_url = url;
    }

    pub fn set_auth_token(&mut self, token: &str) -> Result<(), http::header::InvalidHeaderValue> {
        let value = format!("Bearer {token}");
        let value = value.parse::<HeaderValue>()?;
        self.headers.insert(http::header::AUTHORIZATION, value);
        Ok(())
    }

    pub fn set_user_agent(&mut self, value: HeaderValue) {
        self.headers.insert(http::header::USER_AGENT, value);
    }

    pub fn set_accept(&mut self, value: HeaderValue) {
        self.headers.insert(http::header::ACCEPT, value);
    }

    pub fn set_api_version(&mut self, value: HeaderValue) {
        self.headers.insert(API_VERSION_HEADER, value);
    }

    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }

    pub fn with_backend<B>(self, backend: B) -> Client<B> {
        Client {
            config: self,
            backend,
        }
    }

    /* XXX
    pub fn with_async_backend<B>(self, backend: B) -> Client<B> {
        AsyncClient {
            config: self,
            backend,
        }
    }
    */

    // PRIVATE
    fn prepare_request<R: Request, BE>(
        &self,
        req: &R,
    ) -> Result<PreparedRequest<impl std::io::Read + 'static>, Error<BE, R::Error>> {
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
                return Err(Error::new(url, method, payload));
            }
        };
        Ok(PreparedRequest::from_parts(parts, body))
    }

    // TODO: with_ureq(self), with_reqwest(self) — use default backend values
}

impl Default for ClientConfig {
    fn default() -> ClientConfig {
        ClientConfig::new()
    }
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
    pub fn request<R: Request>(&self, req: R) -> Result<R::Output, Error<B::Error, R::Error>> {
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
            todo!()
        }
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
