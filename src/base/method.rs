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
    /// Returns the name of the method as an uppercase ASCII string
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

    /// Returns true if this is a mutating method (i.e., POST, PUT, PATCH, or
    /// DELETE).
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

    /// Parse a method from its name, case insensitive
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
    /// Convert a `Method` to an [`http::Method`]
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

    /// Convert an [`http::Method`] to a `Method`
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input method does not correspond to one of the
    /// variants of `Method`.
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

/// Error returned by [`Method`]'s `FromStr` implementation
#[derive(Clone, Copy, Debug, Eq, Error, Hash, PartialEq)]
#[error("invalid method name")]
pub struct ParseMethodError;

/// Error returned when trying to convert an [`http::Method`] that does not
/// exist in [`Method`] to the latter type
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("method {0} is not supported by ghreq")]
pub struct MethodConvertError(
    /// The input [`http::Method`] that could not be converted
    pub http::Method,
);

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
