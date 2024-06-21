mod sensor;
use anyhow::Result;
use sensor::read_sensor_data;

#[tokio::main]
async fn main() -> Result<()> {
    let _data = read_sensor_data().await?;
    Ok(())
}
