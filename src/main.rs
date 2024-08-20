use std::sync::Arc;


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let server = Arc::new(egg::Server::new());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    egg::serve(server, listener).await?;
    Ok(())
}
