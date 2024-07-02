use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
};

use anyhow::{anyhow, bail, Result};
use btleplug::{
    api::{
        Central as _, CentralEvent, CharPropFlags, Characteristic, Manager as _, Peripheral as _,
        ScanFilter,
    },
    platform::{Adapter, Manager, Peripheral},
};
use futures::{future, StreamExt};
use tokio::{select, sync::mpsc};
use uuid::Uuid;

const SENSOR_NAME: &str = "LYWSD02";
const DATA_COUNT_CHAR_UUID: Uuid = Uuid::from_u128(0xebe0ccb9_7a0a_4b0c_8a1a_6ff2997da3a6);
const DATA_CHAR_UUID: Uuid = Uuid::from_u128(0xebe0ccbc_7a0a_4b0c_8a1a_6ff2997da3a6);

pub const BYTES_PER_DATUM: usize = 10;

pub async fn update_data(filename: impl AsRef<Path>) -> Result<()> {
    let backup = filename.as_ref().with_extension("bk");
    fs::copy(filename.as_ref(), backup)?;

    let mut file = File::options().read(true).append(true).open(filename)?;
    file.seek(SeekFrom::End(-(BYTES_PER_DATUM as i64)))?;
    let mut last_item_time = [0u8; 4];
    file.read_exact(&mut last_item_time)?;
    let last_item_time = u32::from_le_bytes(last_item_time);

    let data = read_sensor_data(last_item_time).await?;
    file.seek(SeekFrom::End(0))?;
    file.write_all(&data)?;

    Ok(())
}

async fn read_sensor_data(last_item_time: u32) -> Result<Vec<u8>> {
    let manager = Manager::new().await?;
    let Some(adapter) = manager.adapters().await?.into_iter().next() else {
        bail!("Can't find adapter")
    };

    let sensor = find_sensor(Arc::new(adapter)).await?;
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

    let mut notification_stream =
        sensor
            .notifications()
            .await?
            .take(data_count)
            .skip_while(|item| {
                future::ready(
                    u32::from_le_bytes(item.value[4..8].try_into().unwrap()) <= last_item_time,
                )
            });
    let mut buffer = Vec::with_capacity(data_count * BYTES_PER_DATUM);
    while let Some(data) = notification_stream.next().await {
        buffer.extend_from_slice(&data.value[4..]);
    }
    sensor.disconnect().await?;

    Ok(buffer)
}

async fn find_sensor(adapter: Arc<Adapter>) -> Result<Peripheral> {
    let (tx, mut rx) = mpsc::channel::<Peripheral>(1);

    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;

    let res = loop {
        select! {
            biased;

            Some(p) = rx.recv() => break Ok(p),
            Some(event) = events.next() => {
                if let CentralEvent::DeviceDiscovered(id) = event {
                    println!("DeviceDiscovered: {id}");
                    let adapter = adapter.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let p = adapter.peripheral(&id).await?;
                        if p.properties().await?.is_some_and(|props| {
                            props
                                .local_name
                                .is_some_and(|name| name.contains(SENSOR_NAME))
                        }) {
                            println!("Found");
                            tx.send(p).await?;
                        }
                        Ok(()) as Result<()>
                    });
                }
            }
            else => break Err(anyhow!("Can't find sensor"))
        }
    };

    adapter.stop_scan().await?;

    res
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
