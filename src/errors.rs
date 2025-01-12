use crate::{Method, Response, ResponseParts};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum CommonError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum ErrorBody {
    #[default]
    Empty,
    Bytes(Vec<u8>),
    Text(String),
    Json(serde_json::Value),
}

impl ErrorBody {
    pub fn pretty_text(&self) -> Option<Cow<'_, str>> {
        match self {
            ErrorBody::Empty => None,
            ErrorBody::Bytes(_) => None,
            ErrorBody::Text(s) => Some(Cow::from(s)),
            ErrorBody::Json(value) => {
                let Ok(s) = serde_json::to_string_pretty(&value) else {
                    unreachable!("JSONifying a serde_json::Value should not fail");
                };
                Some(Cow::from(s))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("server responded with status {}", self.status())]
pub struct ErrorResponse(Response<ErrorBody>);

impl ErrorResponse {
    pub fn initial_url(&self) -> &Url {
        self.0.initial_url()
    }

    pub fn url(&self) -> &Url {
        self.0.url()
    }

    pub fn method(&self) -> Method {
        self.0.method()
    }

    pub fn status(&self) -> http::status::StatusCode {
        self.0.status()
    }

    pub fn headers(&self) -> &http::header::HeaderMap {
        self.0.headers()
    }

    pub fn body_ref(&self) -> &ErrorBody {
        self.0.body_ref()
    }

    pub fn body_mut(&mut self) -> &mut ErrorBody {
        self.0.body_mut()
    }

    pub fn into_body(self) -> ErrorBody {
        self.0.into_body()
    }

    pub fn into_parts(self) -> (ResponseParts, ErrorBody) {
        self.0.into_parts()
    }

    pub fn pretty_text(&self) -> Option<Cow<'_, str>> {
        self.body_ref().pretty_text()
    }
}

impl From<Response<ErrorBody>> for ErrorResponse {
    fn from(value: Response<ErrorBody>) -> ErrorResponse {
        ErrorResponse(value)
    }
}

impl From<ErrorResponse> for Response<ErrorBody> {
    fn from(value: ErrorResponse) -> Response<ErrorBody> {
        value.0
    }
}

#[derive(Debug)]
pub struct Error<BackendError, E = CommonError> {
    url: Url,
    method: Method,
    payload: ErrorPayload<BackendError, E>,
}

impl<BackendError, E> Error<BackendError, E> {
    pub fn new(url: Url, method: Method, payload: ErrorPayload<BackendError, E>) -> Self {
        Error {
            url,
            method,
            payload,
        }
    }

    pub fn url(&self) -> &Url {
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

    // TODO: Methods to consider adding:
    // - kind(&self) -> PayloadKind // C-style enum with variants matching ErrorPayload
    // - is_send_error(&self) -> bool // etc.
    // - into_send_error(self) -> Option<ClientError> // etc.
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
