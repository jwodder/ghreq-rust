use crate::{CommonError, HeaderMapExt, HttpUrl, Method, Response, ResponseParser, ResponseParts};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("server responded with status {}", self.status())]
pub struct ErrorResponse(Response<ErrorBody>);

impl ErrorResponse {
    pub fn initial_url(&self) -> &HttpUrl {
        self.0.initial_url()
    }

    pub fn url(&self) -> &HttpUrl {
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ErrorResponseParser {
    parts: Option<ResponseParts>,
    body: Vec<u8>,
}

impl ErrorResponseParser {
    pub fn new() -> ErrorResponseParser {
        ErrorResponseParser::default()
    }
}

impl ResponseParser for ErrorResponseParser {
    type Output = ErrorResponse;
    type Error = CommonError;

    fn handle_parts(&mut self, parts: &ResponseParts) {
        self.body.handle_parts(parts);
        self.parts = Some(parts.clone());
    }

    fn handle_bytes(&mut self, buf: &[u8]) {
        self.body.handle_bytes(buf);
    }

    fn end(self) -> Result<Self::Output, Self::Error> {
        let parts = self.parts.expect("handle_parts() should have been called");
        let body = if parts.headers().content_type_is_json() {
            match serde_json::from_slice::<serde_json::Value>(&self.body) {
                Ok(value) => ErrorBody::Json(value),
                Err(e) => return Err(e.into()),
            }
        } else {
            match String::from_utf8(self.body) {
                Ok(s) => {
                    if s.chars().all(char::is_whitespace) {
                        ErrorBody::Empty
                    } else {
                        ErrorBody::Text(s)
                    }
                }
                Err(e) => ErrorBody::Bytes(e.into_bytes()),
            }
        };
        Ok(ErrorResponse(Response::from_parts(parts, body)))
    }
}
