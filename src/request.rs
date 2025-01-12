use crate::{CommonError, Method, ResponseParser};
use serde::Serialize;
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

pub trait Request {
    type Output;
    type Error;

    fn endpoint(&self) -> String; // TODO: Adjust return type

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

    fn body(&self) -> impl RequestBody<Error: Into<Self::Error>>;

    fn parser(&self) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>>;
}

pub trait RequestBody {
    type Error;

    fn headers(&self) -> http::header::HeaderMap {
        http::header::HeaderMap::new()
    }

    fn into_read(self) -> Result<impl std::io::Read, Self::Error>;

    // TODO: Should this method be async?
    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error>;
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

    fn into_read(self) -> Result<impl std::io::Read, Self::Error> {
        Ok(std::io::empty())
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error> {
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

    fn into_read(self) -> Result<impl std::io::Read, Self::Error> {
        Ok(Cursor::new(self))
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error> {
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

    fn into_read(self) -> Result<impl std::io::Read, Self::Error> {
        Ok(Cursor::new(self.into_bytes()))
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error> {
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

    fn into_read(self) -> Result<impl std::io::Read, Self::Error> {
        Ok(Cursor::new(serde_json::to_vec(&self.0)?))
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error> {
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

    fn into_read(self) -> Result<impl std::io::Read, Self::Error> {
        File::open(self.0).map_err(Into::into)
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error> {
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

    fn into_read(self) -> Result<impl std::io::Read, Self::Error> {
        Ok(self)
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead, Self::Error> {
        Ok(tokio::fs::File::from_std(self))
    }
}
