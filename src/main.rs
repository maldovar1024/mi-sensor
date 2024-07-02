mod sensor;
use anyhow::Result;
use sensor::update_data;

#[tokio::main]
async fn main() -> Result<()> {
    update_data("").await?;
    Ok(())
}
