use std::fmt;
use thiserror::Error;

/// An enum of the HTTP methods supported by the GitHub REST API
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }

    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            Method::Post | Method::Put | Method::Patch | Method::Delete
        )
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Method {
    type Err = ParseMethodError;

    fn from_str(s: &str) -> Result<Method, ParseMethodError> {
        match s.to_ascii_uppercase().as_str() {
            "GET" => Ok(Method::Get),
            "HEAD" => Ok(Method::Head),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "PATCH" => Ok(Method::Patch),
            "DELETE" => Ok(Method::Delete),
            _ => Err(ParseMethodError),
        }
    }
}

impl From<Method> for http::Method {
    fn from(value: Method) -> http::Method {
        match value {
            Method::Get => http::Method::GET,
            Method::Head => http::Method::HEAD,
            Method::Post => http::Method::POST,
            Method::Put => http::Method::PUT,
            Method::Patch => http::Method::PATCH,
            Method::Delete => http::Method::DELETE,
        }
    }
}

impl TryFrom<http::Method> for Method {
    type Error = MethodConvertError;

    fn try_from(value: http::Method) -> Result<Method, MethodConvertError> {
        match value {
            http::Method::GET => Ok(Method::Get),
            http::Method::HEAD => Ok(Method::Head),
            http::Method::POST => Ok(Method::Post),
            http::Method::PUT => Ok(Method::Put),
            http::Method::PATCH => Ok(Method::Patch),
            http::Method::DELETE => Ok(Method::Delete),
            other => Err(MethodConvertError(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, Hash, PartialEq)]
#[error("invalid method name")]
pub struct ParseMethodError;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("method {0} is not supported by ghreq")]
pub struct MethodConvertError(pub http::Method);

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Method::Get)]
    #[case(Method::Head)]
    #[case(Method::Post)]
    #[case(Method::Put)]
    #[case(Method::Patch)]
    #[case(Method::Delete)]
    fn parse_display_roundtrip(#[case] m: Method) {
        assert_eq!(m.to_string().parse::<Method>().unwrap(), m);
    }

    #[rstest]
    #[case("get", Method::Get)]
    #[case("Get", Method::Get)]
    #[case("gET", Method::Get)]
    #[case("GeT", Method::Get)]
    #[case("head", Method::Head)]
    #[case("Head", Method::Head)]
    #[case("hEAD", Method::Head)]
    #[case("post", Method::Post)]
    #[case("Post", Method::Post)]
    #[case("pOST", Method::Post)]
    #[case("put", Method::Put)]
    #[case("Put", Method::Put)]
    #[case("pUT", Method::Put)]
    #[case("patch", Method::Patch)]
    #[case("Patch", Method::Patch)]
    #[case("pATCH", Method::Patch)]
    #[case("delete", Method::Delete)]
    #[case("Delete", Method::Delete)]
    #[case("dELETE", Method::Delete)]
    #[case("DeLeTe", Method::Delete)]
    #[case("dElEtE", Method::Delete)]
    fn parse_crazy_casing(#[case] s: &str, #[case] m: Method) {
        assert_eq!(s.parse::<Method>().unwrap(), m);
    }

    #[rstest]
    #[case("CONNECT")]
    #[case("OPTIONS")]
    #[case("TRACE")]
    #[case("PROPFIND")]
    fn parse_unsupported(#[case] s: &str) {
        assert!(s.parse::<Method>().is_err());
    }

    #[rstest]
    #[case(http::Method::CONNECT)]
    #[case(http::Method::OPTIONS)]
    #[case(http::Method::TRACE)]
    fn try_from_unsupported(#[case] m: http::Method) {
        let m2 = m.clone();
        assert_eq!(Method::try_from(m), Err(MethodConvertError(m2)));
    }
}
