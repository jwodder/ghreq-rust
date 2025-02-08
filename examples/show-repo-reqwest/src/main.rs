use clap::Parser;
use ghrepo::GHRepo;
use ghreq::{
    client::ClientConfig,
    errors::CommonError,
    parser::{JsonResponse, ResponseParser},
    request::Request,
    reqwest::ReqwestError,
    Endpoint, HttpUrl, Method,
};
use serde::{Deserialize, Serialize};
use std::process::ExitCode;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShowRepository {
    spec: GHRepo,
}

impl Request for ShowRepository {
    type Output = Repository;
    type Error = CommonError;
    type Body = ();

    fn endpoint(&self) -> Endpoint {
        Endpoint::from_iter(["repos", self.spec.owner(), self.spec.name()])
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

    #[arg(value_name = "OWNER/NAME")]
    spec: GHRepo,
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
    let req = ShowRepository { spec: args.spec };
    match client.request(req).await {
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
            // Use anyhow to display the error chain
            let e = anyhow::Error::new(e);
            eprintln!("{e:?}");
            if let Some(body) = e
                .downcast_ref::<ReqwestError>()
                .and_then(ReqwestError::pretty_text)
            {
                eprintln!("\n{body}");
            }
            ExitCode::FAILURE
        }
    }
}
