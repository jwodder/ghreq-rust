use super::{ClientConfig, RequestParts};
use crate::{
    errors::{Error, ErrorPayload, ErrorResponseParser},
    parser::ResponseParserExt,
    request::Request,
    response::{Response, ResponseParts},
    HttpUrl,
};
use std::future::Future;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AsyncClient<B> {
    pub(super) config: ClientConfig,
    pub(super) backend: B,
}

impl<B> AsyncClient<B> {
    pub fn new(config: ClientConfig, backend: B) -> AsyncClient<B> {
        AsyncClient { config, backend }
    }

    pub fn backend_ref(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: AsyncBackend> AsyncClient<B> {
    #[allow(clippy::future_not_send)]
    pub async fn request<R: Request>(
        &self,
        req: R,
    ) -> Result<R::Output, Error<B::Error, R::Error>> {
        // TODO: Mutation delay
        // TODO: Retrying
        let (reqparts, reqbody) = self.config.prepare_async_request(&req)?.into_parts();
        let initial_url = reqparts.url.clone();
        let method = reqparts.method;
        let backreq = self.backend.prepare_request(reqparts);
        let resp = match self.backend.send(backreq, reqbody).await {
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
            let err_resp = parser.parse_async_response(response).await.map_err(|e| {
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
            parser.parse_async_response(response).await.map_err(|e| {
                Error::new(
                    initial_url,
                    method,
                    ErrorPayload::ParseResponse(e.convert_parse_error()),
                )
            })
        }
    }
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
