use crate::{CommonError, HttpUrl, Method, ResponseParser};
use serde::Serialize;
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

pub trait Request {
    type Output;
    type Error: From<CommonError>;
    type Body: RequestBody<Error: Into<Self::Error>>;

    fn endpoint(&self) -> Endpoint;

    fn method(&self) -> Method;

    fn headers(&self) -> http::header::HeaderMap {
        http::header::HeaderMap::new()
    }

    fn params(&self) -> Vec<(String, String)> {
        // TODO: Rethink return type
        Vec::new()
    }

    fn timeout(&self) -> Option<Duration> {
        None
    }

    fn body(&self) -> Self::Body;

    fn parser(&self) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Url(HttpUrl),
    Path(Vec<String>),
}

impl From<HttpUrl> for Endpoint {
    fn from(value: HttpUrl) -> Endpoint {
        Endpoint::Url(value)
    }
}

impl<S: Into<String>> FromIterator<S> for Endpoint {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        Endpoint::Path(iter.into_iter().map(Into::into).collect())
    }
}

pub trait RequestBody {
    type Error;

    fn headers(&self) -> http::header::HeaderMap {
        http::header::HeaderMap::new()
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error>;

    // TODO: Should this method be async?
    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error>;
}

impl RequestBody for () {
    type Error = CommonError;

    fn headers(&self) -> http::header::HeaderMap {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_LENGTH,
            "0".parse().expect(r#""0" should be a valid HeaderValue"#),
        );
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(std::io::empty())
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(tokio::io::empty())
    }
}

impl RequestBody for Vec<u8> {
    type Error = CommonError;

    fn headers(&self) -> http::header::HeaderMap {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_LENGTH,
            self.len()
                .to_string()
                .parse()
                .expect("integer string should be a valid HeaderValue"),
        );
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(Cursor::new(self))
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(Cursor::new(self))
    }
}

impl RequestBody for String {
    type Error = CommonError;

    fn headers(&self) -> http::header::HeaderMap {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_LENGTH,
            self.len()
                .to_string()
                .parse()
                .expect("integer string should be a valid HeaderValue"),
        );
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(Cursor::new(self.into_bytes()))
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(Cursor::new(self.into_bytes()))
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct JsonBody<T>(T);

impl<T> JsonBody<T> {
    pub fn new(value: T) -> JsonBody<T> {
        JsonBody(value)
    }
}

impl<T: Serialize> RequestBody for JsonBody<T> {
    type Error = CommonError;

    fn headers(&self) -> http::header::HeaderMap {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json"
                .parse()
                .expect(r#""application/json" should be a valid HeaderValue"#),
        );
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(Cursor::new(serde_json::to_vec(&self.0)?))
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(Cursor::new(serde_json::to_vec(&self.0)?))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FilePathBody(PathBuf); // TODO: Rethink name

impl FilePathBody {
    pub fn new<S: Into<PathBuf>>(path: S) -> FilePathBody {
        FilePathBody(path.into())
    }
}

impl RequestBody for FilePathBody {
    type Error = CommonError;

    fn headers(&self) -> http::header::HeaderMap {
        let mut headers = http::header::HeaderMap::new();
        if let Ok(md) = std::fs::metadata(&self.0) {
            headers.insert(
                http::header::CONTENT_LENGTH,
                md.len()
                    .to_string()
                    .parse()
                    .expect("integer string should be a valid HeaderValue"),
            );
        }
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        File::open(self.0).map_err(Into::into)
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        // ASYNC: tokio::fs::File::open(self.0).await.map_err(Into::into)
        let fp = File::open(self.0)?;
        Ok(tokio::fs::File::from_std(fp))
    }
}

impl RequestBody for File {
    type Error = CommonError;

    fn headers(&self) -> http::header::HeaderMap {
        let mut headers = http::header::HeaderMap::new();
        if let Ok(md) = self.metadata() {
            headers.insert(
                http::header::CONTENT_LENGTH,
                md.len()
                    .to_string()
                    .parse()
                    .expect("integer string should be a valid HeaderValue"),
            );
        }
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(self)
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(tokio::fs::File::from_std(self))
    }
}
