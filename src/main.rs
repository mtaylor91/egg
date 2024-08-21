use clap::{Parser, Subcommand};
use std::sync::Arc;


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
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let args = Cli::parse();
    match args.command {
        Command::Serve { bind, port } => {
            serve(bind, port, args.verbose).await?;
        }
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
