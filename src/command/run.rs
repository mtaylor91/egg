use clap::Parser;
use std::sync::Arc;
use uuid::Uuid;

use crate::command::{Cli, Command, Create, CreateCommand, Error};
use crate::plans::{CreatePlan, Plan};
use crate::tasks::{Task, TaskState};


pub async fn run() -> Result<(), Error> {
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
    let server = Arc::new(crate::server::Server::new(verbose));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    crate::server::serve(server, listener).await?;
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
