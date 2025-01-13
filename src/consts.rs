pub static DEFAULT_ACCEPT: &str = "application/vnd.github+json";

pub static DEFAULT_API_URL: &str = "https://api.github.com";

/// The name of the HTTP header used by the GitHub REST API to communicate the
/// API version
pub static API_VERSION_HEADER: &str = "X-GitHub-Api-Version";

pub static DEFAULT_API_VERSION: &str = "2022-11-28";

pub static DEFAULT_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")",
);

pub const READ_BLOCK_SIZE: usize = 2048;
