//! Run with:
//!
//! ```
//! cargo run --example show-repo-ureq --features examples,ureq -- <args>
//! ```

use clap::Parser;
use ghreq::{
    client::ClientConfig,
    errors::CommonError,
    parser::{JsonResponse, ResponseParser},
    request::Request,
    ureq::UreqError,
    Endpoint, HttpUrl, Method,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::process::ExitCode;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShowRepository {
    owner: String,
    name: String,
}

impl Request for ShowRepository {
    type Output = Repository;
    type Error = CommonError;
    type Body = ();

    fn endpoint(&self) -> Endpoint {
        Endpoint::from_iter(["repos", &self.owner, &self.name])
    }

    fn method(&self) -> Method {
        Method::Get
    }

    fn body(&self) {}

    fn parser(
        &self,
    ) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> + Send {
        JsonResponse::new()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Repository {
    full_name: String,
    description: Option<String>,
    topics: Vec<String>,
    html_url: HttpUrl,
    stargazers_count: u64,
    forks_count: u64,
    homepage: Option<String>,
    language: Option<String>,
}

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    #[arg(short = 'J', long)]
    json: bool,

    owner: String,
    name: String,
}

fn main() -> ExitCode {
    let args = Arguments::parse();
    let mut cfg = ClientConfig::new();
    if let Ok(token) = gh_token::get() {
        cfg = match cfg.with_auth_token(&token) {
            Ok(cfg2) => cfg2,
            Err(cfg2) => {
                eprintln!("Warning: invalid GitHub API auth token");
                cfg2
            }
        }
    }
    let client = cfg.with_ureq();
    let req = ShowRepository {
        owner: args.owner,
        name: args.name,
    };
    match client.request(req) {
        Ok(repo) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&repo)
                        .expect("serializing Repository should not fail")
                );
            } else {
                println!("Repository: {}", repo.full_name);

                println!("URL: {}", repo.html_url);

                println!(
                    "Description: {}",
                    repo.description.as_deref().unwrap_or("-")
                );

                println!("Language: {}", repo.language.as_deref().unwrap_or("-"));

                print!("Homepage: ");
                if let Some(hp) = repo.homepage.as_ref().filter(|hp| !hp.is_empty()) {
                    println!("{hp}");
                } else {
                    println!("-");
                }

                print!("Topics: ");
                if repo.topics.is_empty() {
                    println!("-");
                } else {
                    println!("{}", repo.topics.join(", "));
                }

                println!("Stars: {}", repo.stargazers_count);
                println!("Forks: {}", repo.forks_count);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            // Use anyhow to get the error chain displayed
            let e = anyhow::Error::new(e);
            eprintln!("{e:?}");
            if let Some(body) = e
                .downcast::<UreqError>()
                .ok()
                .and_then(|e| e.pretty_text().map(Cow::into_owned))
            {
                eprintln!("\n{body}");
            }
            ExitCode::FAILURE
        }
    }
}
