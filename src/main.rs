use anyhow::{Ok, Result};
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

    let mut mqqt = new_mqqt_client(|msg| {
        println!("Command: {msg}");
    })?;
    let mut bme280 = new_bme280(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c0,
    )?;

    println!("Ready to broadcast ...");

    for _ in 1..5 {
        // 5. This loop initiates measurements, reads values and prints humidity in % and Temperature in °C.
        FreeRtos.delay_ms(100u32);
        {
            use std::result::Result::Ok;
            if let Ok(Some(pressure)) = bme280.read_pressure() {
                {
                    mqqt.message(format!("Pressure: {:.2} Pa", pressure))?;
                }
            }
            if let Ok(Some(temperature)) = bme280.read_temperature() {
                {
                    mqqt.message(format!("Temperature: {:.2} °C", temperature))?;
                }
            }
            if let Ok(Some(humidity)) = bme280.read_humidity() {
                {
                    mqqt.message(format!("Humidity: {:.2} %", humidity))?;
                }
            } else {
                println!("Pressure reading was disabled");
                println!("BME280 sensore not connected");
                mqqt.message("BME280 sensor not connected".to_string())?;
            }
        }

        println!("Waiting 5 seconds");
        FreeRtos.delay_ms(5000u32);
    }
    Ok(())
}
