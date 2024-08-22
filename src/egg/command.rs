use clap::{Args, Parser, Subcommand};
use uuid::Uuid;

mod run;

pub use run::run;


#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
    #[clap(short, long)]
    pub verbose: bool,
}


#[derive(Subcommand)]
pub enum Command {
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
    #[clap(name = "run")]
    Run {
        id: Uuid,
        #[clap(short, long, default_value = "http://127.0.0.1:3000")]
        server: String,
    },
    #[clap(name = "tail")]
    Tail {
        id: Uuid,
        #[clap(short, long, default_value = "http://127.0.0.1:3000")]
        server: String,
    },
}


#[derive(Args)]
pub struct Create {
    #[clap(subcommand)]
    pub command: CreateCommand,
}


#[derive(Subcommand)]
pub enum CreateCommand {
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
