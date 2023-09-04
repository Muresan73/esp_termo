use anyhow::Result;
use embedded_hal::delay::DelayUs;
use embedded_svc::mqtt::client::QoS;
use esp_idf_hal::delay::FreeRtos;

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod mqtt;
mod sensor;
use crate::{mqtt::new_mqqt_client, sensor::new_bme280};

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    println!("program started :)");

    // let mut bme280 = new_bme280()?;
    let mut mqqt = new_mqqt_client()?;

    loop {
        // 5. This loop initiates measurements, reads values and prints humidity in % and Temperature in °C.
        FreeRtos.delay_ms(100u32);

        mqqt.publish(
            format!("{}/feeds/temperature", "USERNAME").as_str(),
            QoS::AtMostOnce,
            false,
            format!("{}", 11).as_bytes(),
        )?;
        println!("Published message");

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
        println!();
        FreeRtos.delay_ms(5000u32);
    }
}
