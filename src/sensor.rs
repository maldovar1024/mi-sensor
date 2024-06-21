use anyhow::{bail, Result};
use btleplug::{
    api::{
        Central as _, CentralEvent, CharPropFlags, Characteristic, Manager as _, Peripheral as _,
        ScanFilter,
    },
    platform::{Adapter, Manager, Peripheral},
};
use futures::StreamExt;
use uuid::Uuid;

const SENSOR_NAME: &str = "LYWSD02";
const DATA_COUNT_CHAR_UUID: Uuid = Uuid::from_u128(0xebe0ccb9_7a0a_4b0c_8a1a_6ff2997da3a6);
const DATA_CHAR_UUID: Uuid = Uuid::from_u128(0xebe0ccbc_7a0a_4b0c_8a1a_6ff2997da3a6);

const BYTES_PER_DATUM: usize = 10;

pub async fn read_sensor_data() -> Result<Vec<u8>> {
    let manager = Manager::new().await?;
    let Some(adapter) = manager.adapters().await?.into_iter().next() else {
        bail!("Can't find adapter")
    };

    let sensor = find_sensor(&adapter).await?;
    if !sensor.is_connected().await? {
        sensor.connect().await?;
    }
    println!("Connected");

    let (data_count_char, data_char) = find_characteristics(&sensor)?;

    let data_count = {
        let data_count = sensor.read(&data_count_char).await?;
        assert!(data_count.len() >= 8);
        let mut buffer = [0u8; 4];
        buffer.copy_from_slice(&data_count[4..8]);
        u32::from_le_bytes(buffer) as usize
    };
    println!("count {data_count}");

    println!("Subscribing to characteristic");
    sensor.subscribe(&data_char).await?;

    let mut notification_stream = sensor.notifications().await?.take(data_count);
    let mut buffer = Vec::with_capacity(data_count * BYTES_PER_DATUM);
    while let Some(data) = notification_stream.next().await {
        buffer.extend_from_slice(&data.value[4..]);
    }
    sensor.disconnect().await?;

    Ok(buffer)
}

async fn find_sensor(adapter: &Adapter) -> Result<Peripheral> {
    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;

    while let Some(event) = events.next().await {
        if let CentralEvent::DeviceDiscovered(id) = event {
            println!("DeviceDiscovered: {id}");
            let p = adapter.peripheral(&id).await?;
            if p.properties().await?.is_some_and(|props| {
                props
                    .local_name
                    .is_some_and(|name| name.contains(SENSOR_NAME))
            }) {
                println!("Found");
                adapter.stop_scan().await?;
                return Ok(p);
            }
        }
    }

    bail!("Can't find sensor");
}

fn find_characteristics(sensor: &Peripheral) -> Result<(Characteristic, Characteristic)> {
    let mut data_count_char = None;
    let mut data_char = None;

    for char in sensor.characteristics() {
        if char.uuid == DATA_CHAR_UUID && char.properties.contains(CharPropFlags::NOTIFY) {
            data_char = Some(char);
        } else if char.uuid == DATA_COUNT_CHAR_UUID && char.properties.contains(CharPropFlags::READ)
        {
            data_count_char = Some(char);
        }
    }

    let Some(data_count_char) = data_count_char else {
        bail!("Can't find `data count` characteristic")
    };
    let Some(data_char) = data_char else {
        bail!("Can't find `data` characteristic")
    };

    Ok((data_count_char, data_char))
}
