/// The default `Accept` header sent in requests
pub static DEFAULT_ACCEPT: &str = "application/vnd.github+json";

/// The default base API URL to which to append path endpoints
pub static DEFAULT_API_URL: &str = "https://api.github.com";

/// The name of the HTTP header used by the GitHub REST API to communicate the
/// API version
pub static API_VERSION_HEADER: &str = "X-GitHub-Api-Version";

/// The default `X-GitHub-Api-Version` header sent in requests
pub static DEFAULT_API_VERSION: &str = "2022-11-28";

/// The default `User-Agent` header sent in requests
///
/// This value *will* change for each release of `ghreq`.
pub static DEFAULT_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")",
);

/// The maximum number of bytes read at once from a response body.
///
/// This value may change at any time between releases.
pub const READ_BLOCK_SIZE: usize = 2048;
