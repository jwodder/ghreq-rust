use crate::{AsyncBackend, AsyncBackendResponse, HttpUrl, RequestParts};
use futures_util::TryStreamExt;
use tokio_util::io::{ReaderStream, StreamReader};

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

    async fn send<R: tokio::io::AsyncRead + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        r.body(reqwest::Body::wrap_stream(ReaderStream::new(body)))
            .send()
            .await
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
