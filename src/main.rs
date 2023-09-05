use anyhow::Result;
use embedded_hal::delay::DelayUs;
use esp_idf_hal::{delay::FreeRtos, prelude::Peripherals};

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod mqtt;
mod sensor;
mod wifi;
use crate::{
    mqtt::{new_mqqt_client, SimpleMqttClient},
    sensor::new_bme280,
};

fn main() -> Result<()> {
    println!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let _wifi = wifi::connect(peripherals.modem)?;

    let mut mqqt = new_mqqt_client()?;
    // let mut bme280 = new_bme280(peripherals)?;
    // http_server::start_server()?;

    loop {
        // 5. This loop initiates measurements, reads values and prints humidity in % and Temperature in °C.
        FreeRtos.delay_ms(100u32);

        mqqt.message("test".into())?;

        // if let Some(humidity) = bme280.read_humidity()? {
        //     println!("Humidity: {:.2} %", humidity);
        // } else {
        //     println!("Humidity reading was disabled");
        // }
        // if let Some(temperature) = bme280.read_temperature()? {
        //     println!("Temperature: {} C", temperature);
        // } else {
        //     println!("Temperature reading was disabled");
        // }
        // if let Some(pressure) = bme280.read_pressure()? {
        //     println!("Pressure: {:.2} Pa", pressure);
        // } else {
        //     println!("Pressure reading was disabled");
        // }
        println!("Waiting 5 seconds");
        FreeRtos.delay_ms(5000u32);
    }
}
