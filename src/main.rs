use clap::{Args, Parser, Subcommand};
use std::sync::Arc;
use uuid::Uuid;

use egg::{CreatePlan, Plan, Task, TaskState};


#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
    #[clap(short, long)]
    verbose: bool,
}


#[derive(Subcommand)]
enum Command {
    #[clap(name = "create")]
    Create(Create),
    #[clap(name = "plan")]
    Plan {
        id: Uuid,
        #[clap(short, long, default_value = "http://127.0.0.1:3000")]
        server: String,
    },
    #[clap(name = "serve")]
    Serve {
        #[clap(short, long, default_value = "127.0.0.1")]
        bind: String,
        #[clap(short, long, default_value = "3000")]
        port: u16,
    },
    #[clap(name = "start")]
    Start {
        id: Uuid,
        #[clap(short, long, default_value = "http://127.0.0.1:3000")]
        server: String,
    },
}


#[derive(Args)]
struct Create {
    #[clap(subcommand)]
    command: CreateCommand,
}


#[derive(Subcommand)]
enum CreateCommand {
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
        Command::Create(Create { command }) => match command {
            CreateCommand::Plan { filename, server } => {
                create_plan(filename, server, args.verbose).await?;
            }
        }
        Command::Plan { id, server } => {
            plan(id, server, args.verbose).await?;
        }
        Command::Serve { bind, port } => {
            serve(bind, port, args.verbose).await?;
        }
        Command::Start { id, server } => {
            start(id, server, args.verbose).await?;
        }
    }
    Ok(())
}


async fn create_plan(
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


async fn plan(id: Uuid, server: String, verbose: bool) -> Result<(), Error> {
    let response = reqwest::Client::new()
        .post(&format!("{}/plan/{}", server, id))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
    }

    let task: Task = response.json().await?;
    if verbose {
        println!("{:?}", task);
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


async fn start(id: Uuid, server: String, verbose: bool) -> Result<(), Error> {
    let response = reqwest::Client::new()
        .post(&format!("{}/tasks/{}/start", server, id))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
    }

    let task: TaskState = response.json().await?;
    if verbose {
        println!("{:?}", task);
    }

    Ok(())
}
