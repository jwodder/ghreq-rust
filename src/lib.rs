#![cfg_attr(docsrs, feature(doc_cfg))]
mod base;
pub mod client;
pub mod consts;
pub mod errors;
pub mod pagination;
pub mod parser;
pub mod request;
pub mod response;
mod util;
pub use crate::base::*;

/// Re-export of [`http::header`]
pub use http::header;

/// Re-export of [`http::status`]
pub use http::status;

#[cfg(feature = "reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
pub mod reqwest;

#[cfg(feature = "ureq")]
#[cfg_attr(docsrs, doc(cfg(feature = "ureq")))]
pub mod ureq;
