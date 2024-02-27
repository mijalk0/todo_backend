use fullstack_todo;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    fullstack_todo::run().await
}
