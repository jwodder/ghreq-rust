use clap::Parser;
use futures_util::StreamExt;
use ghreq::{
    client::ClientConfig, pagination::PaginationRequest, reqwest::ReqwestError, Endpoint, HttpUrl,
};
use serde::{Deserialize, Serialize};
use std::process::ExitCode;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListRepositories {
    owner: String,
}

impl PaginationRequest for ListRepositories {
    type Item = Repository;

    fn endpoint(&self) -> Endpoint {
        Endpoint::from_iter(["users", &*self.owner, "repos"])
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
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
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
    let client = cfg.with_reqwest();
    let req = ListRepositories { owner: args.owner };
    let mut first = true;
    let mut stream = client.paginate(req);
    while let Some(r) = stream.next().await {
        match r {
            Ok(repo) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&repo)
                            .expect("serializing Repository should not fail")
                    );
                } else {
                    if !std::mem::replace(&mut first, false) {
                        println!();
                    }
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
            }
            Err(e) => {
                // Use anyhow to display the error chain
                let e = anyhow::Error::new(e);
                eprintln!("{e:?}");
                if let Some(body) = e
                    .downcast_ref::<ReqwestError>()
                    .and_then(ReqwestError::pretty_text)
                {
                    eprintln!("\n{body}");
                }
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
