use crate::{errors::CommonError, parser::ResponseParser, Endpoint, HeaderMapExt, Method};
use http::header::HeaderMap;
use serde::Serialize;
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

pub trait Request {
    type Output;
    type Error: From<CommonError>;
    // The rest of the library requires Body to implement either RequestBody or
    // AsyncRequestBody, and the Error type must impl Into<Request::Error>.
    type Body;

    fn endpoint(&self) -> Endpoint;

    fn method(&self) -> Method;

    fn headers(&self) -> HeaderMap {
        HeaderMap::new()
    }

    fn params(&self) -> Vec<(String, String)> {
        // TODO: Rethink return type
        Vec::new()
    }

    fn timeout(&self) -> Option<Duration> {
        None
    }

    fn body(&self) -> Self::Body;

    fn parser(&self)
        -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send;
}

impl<T: Request + ?Sized> Request for &T {
    type Output = T::Output;
    type Error = T::Error;
    type Body = T::Body;

    fn endpoint(&self) -> Endpoint {
        (*self).endpoint()
    }

    fn method(&self) -> Method {
        (*self).method()
    }

    fn headers(&self) -> HeaderMap {
        (*self).headers()
    }

    fn params(&self) -> Vec<(String, String)> {
        (*self).params()
    }

    fn timeout(&self) -> Option<Duration> {
        (*self).timeout()
    }

    fn body(&self) -> Self::Body {
        (*self).body()
    }

    fn parser(
        &self,
    ) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send {
        (*self).parser()
    }
}

impl<T: Request + ?Sized> Request for &mut T {
    type Output = T::Output;
    type Error = T::Error;
    type Body = T::Body;

    fn endpoint(&self) -> Endpoint {
        (**self).endpoint()
    }

    fn method(&self) -> Method {
        (**self).method()
    }

    fn headers(&self) -> HeaderMap {
        (**self).headers()
    }

    fn params(&self) -> Vec<(String, String)> {
        (**self).params()
    }

    fn timeout(&self) -> Option<Duration> {
        (**self).timeout()
    }

    fn body(&self) -> Self::Body {
        (**self).body()
    }

    fn parser(
        &self,
    ) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send {
        (**self).parser()
    }
}

impl<T: Request + ?Sized> Request for std::sync::Arc<T> {
    type Output = T::Output;
    type Error = T::Error;
    type Body = T::Body;

    fn endpoint(&self) -> Endpoint {
        (**self).endpoint()
    }

    fn method(&self) -> Method {
        (**self).method()
    }

    fn headers(&self) -> HeaderMap {
        (**self).headers()
    }

    fn params(&self) -> Vec<(String, String)> {
        (**self).params()
    }

    fn timeout(&self) -> Option<Duration> {
        (**self).timeout()
    }

    fn body(&self) -> Self::Body {
        (**self).body()
    }

    fn parser(
        &self,
    ) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send {
        (**self).parser()
    }
}

impl<T: Request + ?Sized> Request for Box<T> {
    type Output = T::Output;
    type Error = T::Error;
    type Body = T::Body;

    fn endpoint(&self) -> Endpoint {
        (**self).endpoint()
    }

    fn method(&self) -> Method {
        (**self).method()
    }

    fn headers(&self) -> HeaderMap {
        (**self).headers()
    }

    fn params(&self) -> Vec<(String, String)> {
        (**self).params()
    }

    fn timeout(&self) -> Option<Duration> {
        (**self).timeout()
    }

    fn body(&self) -> Self::Body {
        (**self).body()
    }

    fn parser(
        &self,
    ) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send {
        (**self).parser()
    }
}

pub trait RequestBody {
    type Error;

    fn headers(&self) -> HeaderMap {
        HeaderMap::new()
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error>;
}

#[cfg(feature = "tokio")]
pub trait AsyncRequestBody {
    type Error;

    fn headers(&self) -> HeaderMap {
        HeaderMap::new()
    }

    // TODO: Should this method be async?
    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error>;
}

impl RequestBody for () {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.set_content_length(0);
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(std::io::empty())
    }
}

#[cfg(feature = "tokio")]
impl AsyncRequestBody for () {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.set_content_length(0);
        headers
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(tokio::io::empty())
    }
}

impl RequestBody for Vec<u8> {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(sz) = self.len().try_into() {
            headers.set_content_length(sz);
        }
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(Cursor::new(self))
    }
}

#[cfg(feature = "tokio")]
impl AsyncRequestBody for Vec<u8> {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(sz) = self.len().try_into() {
            headers.set_content_length(sz);
        }
        headers
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(Cursor::new(self))
    }
}

impl RequestBody for String {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(sz) = self.len().try_into() {
            headers.set_content_length(sz);
        }
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(Cursor::new(self.into_bytes()))
    }
}

#[cfg(feature = "tokio")]
impl AsyncRequestBody for String {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(sz) = self.len().try_into() {
            headers.set_content_length(sz);
        }
        headers
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

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
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
}

#[cfg(feature = "tokio")]
impl<T: Serialize> AsyncRequestBody for JsonBody<T> {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json"
                .parse()
                .expect(r#""application/json" should be a valid HeaderValue"#),
        );
        headers
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(Cursor::new(serde_json::to_vec(&self.0)?))
    }
}

impl RequestBody for PathBuf {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(md) = std::fs::metadata(self) {
            headers.set_content_length(md.len());
        }
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        File::open(self).map_err(Into::into)
    }
}

#[cfg(feature = "tokio")]
impl AsyncRequestBody for PathBuf {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(md) = std::fs::metadata(self) {
            headers.set_content_length(md.len());
        }
        headers
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        // ASYNC: tokio::fs::File::open(self.0).await.map_err(Into::into)
        let fp = File::open(self)?;
        Ok(tokio::fs::File::from_std(fp))
    }
}

impl RequestBody for File {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(md) = self.metadata() {
            headers.set_content_length(md.len());
        }
        headers
    }

    fn into_read(self) -> Result<impl std::io::Read + 'static, Self::Error> {
        Ok(self)
    }
}

#[cfg(feature = "tokio")]
impl AsyncRequestBody for File {
    type Error = CommonError;

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(md) = self.metadata() {
            headers.set_content_length(md.len());
        }
        headers
    }

    fn into_async_read(self) -> Result<impl tokio::io::AsyncRead + Send + 'static, Self::Error> {
        Ok(tokio::fs::File::from_std(self))
    }
}
