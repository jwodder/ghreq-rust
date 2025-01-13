mod err_resp;
pub use self::err_resp::*;
use crate::{HttpUrl, Method};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommonError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct Error<BackendError, E = CommonError> {
    url: HttpUrl,
    method: Method,
    payload: ErrorPayload<BackendError, E>,
}

impl<BackendError, E> Error<BackendError, E> {
    pub fn new(url: HttpUrl, method: Method, payload: ErrorPayload<BackendError, E>) -> Self {
        Error {
            url,
            method,
            payload,
        }
    }

    pub fn url(&self) -> &HttpUrl {
        &self.url
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn payload_ref(&self) -> &ErrorPayload<BackendError, E> {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut ErrorPayload<BackendError, E> {
        &mut self.payload
    }

    pub fn into_payload(self) -> ErrorPayload<BackendError, E> {
        self.payload
    }

    pub fn pretty_text(&self) -> Option<Cow<'_, str>> {
        self.payload.pretty_text()
    }
}

impl<BackendError: StdError + 'static, E: StdError + 'static> fmt::Display
    for Error<BackendError, E>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} request to {} failed: {}",
            self.method, self.url, self.payload
        )
    }
}

impl<BackendError: StdError + 'static, E: StdError + 'static> StdError for Error<BackendError, E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.payload)
    }
}

#[derive(Debug, Error)]
pub enum ErrorPayload<BackendError, E = CommonError> {
    #[error("failed to prepare request")]
    PrepareRequest(#[source] E),

    #[error("failed to read request body")]
    ReadRequestBody(#[source] std::io::Error),

    #[error("failed to send request")]
    Send(#[source] BackendError),

    #[error("server responded with status {}", .0.status())]
    Status(#[source] ErrorResponse),

    #[error(transparent)]
    ParseResponse(ParseResponseError<E>),
}

impl<BackendError, E> ErrorPayload<BackendError, E> {
    pub fn pretty_text(&self) -> Option<Cow<'_, str>> {
        if let ErrorPayload::Status(ref r) = self {
            r.pretty_text()
        } else {
            None
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseResponseError<E> {
    #[error("error reading response body")]
    Read(std::io::Error),

    #[error("error parsing response body")]
    Parse(#[source] E),
}

impl<E> ParseResponseError<E> {
    pub(crate) fn convert_parse_error<E2>(self) -> ParseResponseError<E2>
    where
        E: Into<E2>,
    {
        match self {
            ParseResponseError::Read(e) => ParseResponseError::Read(e),
            ParseResponseError::Parse(e) => ParseResponseError::Parse(e.into()),
        }
    }
}
