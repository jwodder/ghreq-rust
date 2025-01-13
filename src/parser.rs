use crate::{CommonError, HeaderMapExt, ParseResponseError, Response, ResponseParts};
use bstr::ByteVec;
use serde::de::DeserializeOwned;
use std::io::Write;
use std::marker::PhantomData;
use tokio::io::AsyncReadExt;

pub const READ_BLOCK_SIZE: usize = 2048;

pub trait ResponseParser: Sized {
    type Output;
    type Error: From<std::io::Error>;

    fn handle_parts(&mut self, parts: &ResponseParts);
    fn handle_bytes(&mut self, buf: &[u8]);
    // This method may panic if handle_parts() was never called
    fn end(self) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ignore;

impl Ignore {
    pub fn new() -> Ignore {
        Ignore
    }
}

impl ResponseParser for Ignore {
    type Output = ();
    type Error = CommonError;

    fn handle_parts(&mut self, _parts: &ResponseParts) {}

    fn handle_bytes(&mut self, _buf: &[u8]) {}

    fn end(self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

impl ResponseParser for Vec<u8> {
    type Output = Vec<u8>;
    type Error = CommonError;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        if let Some(size) = parts
            .headers()
            .content_length()
            .and_then(|sz| usize::try_from(sz).ok())
        {
            self.reserve(size);
        }
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.extend_from_slice(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Utf8Text(Vec<u8>);

impl Utf8Text {
    pub fn new() -> Self {
        Utf8Text(Vec::new())
    }
}

impl ResponseParser for Utf8Text {
    type Output = String;
    type Error = CommonError;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        self.0.handle_parts(parts);
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.0.handle_bytes(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        String::from_utf8(self.0).map_err(|e| e.utf8_error().into())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LossyUtf8Text(Vec<u8>);

impl LossyUtf8Text {
    pub fn new() -> Self {
        LossyUtf8Text(Vec::new())
    }
}

impl ResponseParser for LossyUtf8Text {
    type Output = String;
    type Error = CommonError;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        self.0.handle_parts(parts);
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.0.handle_bytes(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        Ok(self.0.into_string_lossy())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JsonResponse<T> {
    buf: Vec<u8>,
    _output: PhantomData<T>,
}

impl<T> JsonResponse<T> {
    pub fn new() -> JsonResponse<T> {
        JsonResponse {
            buf: Vec::new(),
            _output: PhantomData,
        }
    }
}

impl<T: DeserializeOwned> ResponseParser for JsonResponse<T> {
    type Output = T;
    type Error = CommonError;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        self.buf.handle_parts(parts);
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.buf.handle_bytes(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        serde_json::from_slice(&self.buf).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WithParts<T> {
    parts: Option<ResponseParts>,
    inner: T,
}

impl<T> WithParts<T> {
    pub fn new(inner: T) -> WithParts<T> {
        WithParts { parts: None, inner }
    }
}

impl<T: ResponseParser> ResponseParser for WithParts<T> {
    type Output = Response<T::Output>;
    type Error = T::Error;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        self.inner.handle_parts(parts);
        self.parts = Some(parts.clone());
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.inner.handle_bytes(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        let parts = self.parts.expect("handle_parts() should have been called");
        let body = self.inner.end()?;
        Ok(Response::from_parts(parts, body))
    }
}

#[derive(Debug, Default)]
pub struct ToWriter<W> {
    writer: W,
    err: Option<std::io::Error>,
}

impl<W> ToWriter<W> {
    pub fn new(writer: W) -> ToWriter<W> {
        ToWriter { writer, err: None }
    }
}

impl<W: Write> ResponseParser for ToWriter<W> {
    type Output = ();
    type Error = CommonError;

    fn handle_parts(&mut self, _parts: &ResponseParts) {}

    fn handle_bytes(&mut self, buf: &[u8]) {
        if self.err.is_none() {
            if let Err(e) = self.writer.write_all(buf) {
                self.err = Some(e);
            }
        }
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        if let Some(e) = self.err {
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

pub trait ResponseParserExt: ResponseParser {
    fn parse_response<R: std::io::Read>(
        mut self,
        resp: Response<R>,
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        let (parts, mut body) = resp.into_parts();
        self.handle_parts(&parts);
        let mut buf = vec![0u8; READ_BLOCK_SIZE];
        loop {
            match body.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => self.handle_bytes(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(ParseResponseError::Read(e)),
            }
        }
        self.end().map_err(ParseResponseError::Parse)
    }

    #[allow(async_fn_in_trait)]
    async fn parse_async_response<R: tokio::io::AsyncRead + Send + 'static>(
        mut self,
        resp: Response<R>,
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        let (parts, body) = resp.into_parts();
        self.handle_parts(&parts);
        let mut buf = vec![0u8; READ_BLOCK_SIZE];
        tokio::pin!(body);
        loop {
            match body.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => self.handle_bytes(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(ParseResponseError::Read(e)),
            }
        }
        self.end().map_err(ParseResponseError::Parse)
    }

    // TODO: map(), try_map()
}

impl<R: ResponseParser> ResponseParserExt for R {}
