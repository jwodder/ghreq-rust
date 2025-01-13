use crate::{
    client::{Backend, BackendResponse, Client, RequestParts},
    HttpUrl,
};

pub type ReqwestBlockingClient = Client<reqwest::blocking::Client>;

impl Backend for reqwest::blocking::Client {
    type Request = reqwest::blocking::RequestBuilder;
    type Response = reqwest::blocking::Response;
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

    fn send<R: std::io::Read + Send + 'static>(
        &self,
        r: Self::Request,
        body: R,
    ) -> Result<Self::Response, Self::Error> {
        r.body(reqwest::blocking::Body::new(body)).send()
    }
}

impl BackendResponse for reqwest::blocking::Response {
    fn url(&self) -> HttpUrl {
        HttpUrl::try_from(self.url().clone()).expect("response URL should be a valid HTTP URL")
    }

    fn status(&self) -> http::status::StatusCode {
        self.status()
    }

    fn headers(&self) -> http::header::HeaderMap {
        self.headers().clone()
    }

    fn body_reader(self) -> impl std::io::Read {
        std::io::Cursor::new(self.bytes().unwrap())
    }
}
