use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspBackgroundEventLoop;
use log::{error, info};
use std::result::Result::Ok;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod mqtt;
mod sensor;
mod wifi;
use crate::{
    mqtt::{new_mqqt_client, MqttCommand, SimpleMqttClient},
    sensor::{new_bme280, MessageAble, SoilMoisture},
};

fn main() -> Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let mut event_loop = EspBackgroundEventLoop::new(&Default::default())?;

    let _wifi = wifi::connect(peripherals.modem)?;

    let mqqt_loop = event_loop.clone();
    let mut mqqt = new_mqqt_client(move |cmd| {
        let post = mqqt_loop.post(&cmd, None);
        if post.is_err() {
            error!("Error posting to event loop: {:?}", post);
        }
        info!("Added command to the event loop: {:?}", cmd);
    })?;
    info!("Ready to broadcast ...");

    info!("Setup sensors");
    let mut bme280_rs = new_bme280(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c0,
    );

    let mut soil_moisture_rs = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36);

    info!("Setup background event loop");
    // this let variable is necessary so that the Subscription does not get dropped
    let _subscription = event_loop.subscribe(move |message: &MqttCommand| {
        info!("Got message from the event loop: {:?}", message);
        match message {
            MqttCommand::Water(on_off) => info!("Turn on water: {on_off}"),
            MqttCommand::Lamp(percent) => info!("Set lamp dim to: {percent}"),
            MqttCommand::ReadSoilMoisture => match soil_moisture_rs.as_mut() {
                Ok(moisture) => {
                    if let Some(msg) = moisture.to_json() {
                        mqqt.safe_message(msg)
                    } else {
                        mqqt.safe_message("Error reading Soil sensor values".to_string());
                    }
                }
                Err(e) => {
                    error!("Error with Soil moisture driver: {:?}", e);
                    mqqt.safe_message("Soil sensor is not connected".to_string());
                }
            },

            MqttCommand::ReadBarometer => match bme280_rs.as_mut() {
                Ok(bme280) => {
                    if let Some(msg) = bme280.to_json() {
                        mqqt.safe_message(msg)
                    } else {
                        mqqt.safe_message("Error reading Bme280 sensor values".to_string());
                    }
                }
                Err(e) => {
                    error!("Error with bme280 driver: {:?}", e);
                    mqqt.safe_message("Bme280 sensor is not connected".to_string());
                }
            },
        }
    });

    info!("Ready for action!");
    event_loop.spin(None)?;

    Ok(())
}
