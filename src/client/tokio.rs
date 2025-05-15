use super::{ClientConfig, RequestParts};
use crate::{
    errors::{Error, ErrorPayload, ErrorResponseParser},
    pagination::{PaginationRequest, PaginationStream},
    parser::ResponseParserExt,
    request::{AsyncRequestBody, Request},
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

impl<B: AsyncBackend + Sync> AsyncClient<B> {
    pub async fn request<R>(&self, req: R) -> Result<R::Output, Error<B::Error, R::Error>>
    where
        R: Request<Body: AsyncRequestBody<Error: Into<R::Error>>> + Send,
    {
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
                ErrorPayload::Status(Box::new(err_resp)),
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

impl<B: AsyncBackend + Clone + Sync> AsyncClient<B> {
    pub fn paginate<R: PaginationRequest>(&self, req: R) -> PaginationStream<B, R> {
        PaginationStream::new(self.clone(), req)
    }
}

pub trait AsyncBackend {
    type Request;
    type Response: AsyncBackendResponse;
    type Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request;

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

impl<T: AsyncBackend + Sync + ?Sized> AsyncBackend for &T {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (*self).prepare_request(r)
    }

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static {
        (*self).send(r, body)
    }
}

impl<T: AsyncBackend + ?Sized> AsyncBackend for &mut T {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (**self).prepare_request(r)
    }

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static {
        (**self).send(r, body)
    }
}

impl<T: AsyncBackend + ?Sized> AsyncBackend for std::sync::Arc<T> {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (**self).prepare_request(r)
    }

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static {
        (**self).send(r, body)
    }
}

impl<T: AsyncBackend + ?Sized> AsyncBackend for Box<T> {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;

    fn prepare_request(&self, r: RequestParts) -> Self::Request {
        (**self).prepare_request(r)
    }

    fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static {
        (**self).send(r, body)
    }
}

pub trait AsyncBackendResponse: Send {
    fn url(&self) -> HttpUrl;
    fn status(&self) -> http::status::StatusCode;
    fn headers(&self) -> http::header::HeaderMap;
    fn body_reader(self) -> impl tokio::io::AsyncRead + Send + 'static;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_send() {
        #[allow(dead_code)]
        fn require_send<T: Send>(_t: T) {}

        #[allow(dead_code)]
        fn check<B, R>(client: AsyncClient<B>, req: R)
        where
            B: AsyncBackend + Sync,
            R: Request<Body: AsyncRequestBody<Error: Into<R::Error>>> + Send,
        {
            require_send(client.request(req));
        }
    }
}
