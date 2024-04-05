use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    todo_backend::run().await
}
