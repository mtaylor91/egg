use egg::command::{Error, run};


#[tokio::main]
async fn main() -> Result<(), Error> {
    run().await
}
