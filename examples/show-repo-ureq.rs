use clap::Parser;
use ghreq::{ClientConfig, CommonError, Endpoint, JsonResponse, Method, Request, ResponseParser};
use serde::{Deserialize, Serialize};
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

    fn parser(&self) -> impl ResponseParser<Output = Self::Output, Error: Into<Self::Error>> {
        JsonResponse::new()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Repository {
    full_name: String,
    description: Option<String>,
    topics: Vec<String>,
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
    let client = ClientConfig::new().with_ureq();
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
                println!("Full name: {}", repo.full_name);
                println!(
                    "Description: {}",
                    repo.description.as_deref().unwrap_or("-")
                );
                print!("Topics: ");
                if repo.topics.is_empty() {
                    println!("-");
                } else {
                    println!("{}", repo.topics.join(", "));
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            if let Some(body) = e.pretty_text() {
                eprintln!("{body}");
            }
            ExitCode::FAILURE
        }
    }
}
