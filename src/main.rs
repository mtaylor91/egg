use clap::{Parser, Subcommand};
use std::sync::Arc;

use egg::{CreatePlan, Plan};


#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
    #[clap(short, long)]
    verbose: bool,
}


#[derive(Subcommand)]
enum Command {
    #[clap(name = "serve")]
    Serve {
        #[clap(short, long, default_value = "127.0.0.1")]
        bind: String,
        #[clap(short, long, default_value = "3000")]
        port: u16,
    },
    #[clap(name = "plan")]
    Plan {
        filename: String,
        #[clap(short, long, default_value = "http://127.0.0.1:3000")]
        server: String,
    },
}


#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Reqwest(reqwest::Error),
    Serde(serde_yaml::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::Serde(err)
    }
}


#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Cli::parse();
    match args.command {
        Command::Serve { bind, port } => {
            serve(bind, port, args.verbose).await?;
        }
        Command::Plan { filename, server } => {
            plan(filename, server, args.verbose).await?;
        }
    }
    Ok(())
}


async fn plan(
    filename: String,
    server: String,
    verbose: bool
) -> Result<(), Error> {
    let plan: CreatePlan = serde_yaml::from_str(&std::fs::read_to_string(filename)?)?;
    let response = reqwest::Client::new()
        .post(&format!("{}/plans", server))
        .json(&plan)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
    }

    let plan: Plan = response.json().await?;
    if verbose {
        println!("{:?}", plan);
    }

    Ok(())
}


async fn serve(bind: String, port: u16, verbose: bool) -> Result<(), std::io::Error> {
    let addr = format!("{}:{}", bind, port);
    let server = Arc::new(egg::Server::new(verbose));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    egg::serve(server, listener).await?;
    Ok(())
}
