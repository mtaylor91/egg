use clap::Parser;
use futures::StreamExt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

use crate::client::Client;
use crate::command::{Cli, Command, Create, CreateCommand, Error};
use crate::plans::{CreatePlan, Plan};
use crate::process::Output;
use crate::tasks::{Task, TaskSpec, TaskState};


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
        Command::Tail { id, server } => {
            tail_task(id, server, args.verbose).await?;
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
        let client = Client::new(server.clone());
        let task = client.get_task(id).await?;

        if verbose {
            println!("Tailing task: {:?}", task);
        }

        match task.spec {
            TaskSpec::Command { .. } => {
                tail_command(id, server).await?;
            }
            TaskSpec::TaskGroup { parallel } => {
                tail_parallel(parallel, server, verbose).await?;
            }
            TaskSpec::TaskList { serial } => {
                tail_serial(serial, server, verbose).await?;
            }
        }

        Ok(())
    })
}


async fn tail_command(
    id: Uuid,
    server: String,
) -> Result<(), Error> {
    let client = Client::new(server);
    let stream = client.tail_task(id).await?;

    let _ = stream.map(|line| {
        match line {
            Ok(line) => {
                match line {
                    Output::Stdout(line) => {
                        println!("{}", line);
                    }
                    Output::Stderr(line) => {
                        eprintln!("{}", line);
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }).collect::<()>().await;

    Ok(())
}


async fn tail_parallel(
    parallel: Vec<Uuid>,
    server: String,
    verbose: bool
) -> Result<(), Error> {
    // dispatch each task to a separate thread
    let mut handles = vec![];

    for id in parallel {
        let server = server.clone();
        let handle = tokio::spawn(async move {
            tail_task(id, server, verbose).await
        });

        handles.push(handle);
    }

    // wait for all tasks to finish
    for handle in handles {
        match handle.await {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }

    Ok(())
}


async fn tail_serial(
    serial: Vec<Uuid>,
    server: String,
    verbose: bool
) -> Result<(), Error> {
    for id in serial {
        tail_task(id, server.clone(), verbose).await?;
    }

    Ok(())
}
