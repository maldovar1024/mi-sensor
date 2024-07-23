此代码是针对[小米米家电子温湿度计 Pro](https://www.mi.com/shop/buy/detail?product_id=9542) 的，关于具体的数据格式，参考 https://github.com/JsBergbau/MiTemperature2/issues/1

连接好之后，首先通过 id 为 `ebe0ccb9_7a0a_4b0c_8a1a_6ff2997da3a6` 的 Characteristic 获取数据量，然后通过 id 为 `ebe0ccbc_7a0a_4b0c_8a1a_6ff2997da3a6` 的 Characteristic 读取设备内保存的历史数据，详见 [src/sensor.rs](src/sensor.rs)
