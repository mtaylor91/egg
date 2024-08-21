use clap::Parser;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

use crate::client::Client;
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
        Command::Run { id, server } => {
            run_task(id, server, args.verbose).await?;
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
    let plan: Plan = Client::new(server).create_plan(&plan).await?;
    if verbose {
        println!("{:?}", plan);
    }

    Ok(())
}


async fn plan(id: Uuid, server: String, verbose: bool) -> Result<(), Error> {
    let task: Task = Client::new(server).plan(id).await?;
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
    let task: TaskState = Client::new(server).start_task(id).await?;
    if verbose {
        println!("{:?}", task);
    }

    Ok(())
}


async fn run_task(id: Uuid, server: String, verbose: bool) -> Result<(), Error> {
    start(id, server.clone(), verbose).await?;
    tail_task(id, server, verbose).await
}


fn tail_task(
    id: Uuid,
    server: String,
    verbose: bool
) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> {
    Box::pin(async move {
        let client = Client::new(server);
        let task = client.get_task(id).await?;
        Ok(())
    })
}
