use std::error::Error;

mod discovery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let discovery = discovery::Discovery::new("0.0.0.0:3702").await?;
    Ok(())
}
