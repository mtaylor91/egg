use poultry::egg::command::Error;


#[tokio::main]
async fn main() -> Result<(), Error> {
    poultry::egg::command::run().await
}
