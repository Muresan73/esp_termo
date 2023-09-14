use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspBackgroundEventLoop;
use log::{error, info, warn};
use std::{result::Result::Ok, sync::mpsc::channel, thread};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod relay;
mod sensor;
mod utils;
use relay::{
    discord::discord_webhook,
    mqtt::{new_mqqt_client, MqttCommand, SimplCommandError, SimpleMqttClient},
};
use sensor::{bme280::new_bme280, soil::SoilMoisture, MessageAble};
use utils::wifi;
fn main() -> Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let mut event_loop = EspBackgroundEventLoop::new(&Default::default())?;

    let _wifi = wifi::connect(peripherals.modem)?;

    discord_webhook()?;
    let cmd_loop = event_loop.clone();
    let error_loop = event_loop.clone();
    let mut mqqt = new_mqqt_client(
        move |cmd| {
            let post = cmd_loop.post(&cmd, None);
            if post.is_err() {
                error!("Error posting to event loop: {:?}", post);
            }
            info!("Added command to the event loop: {:?}", cmd);
        },
        move |err| {
            let err: SimplCommandError = err.into();
            let post = error_loop.post(&err, None);
            if post.is_err() {
                error!("Error posting to event loop: {:?}", post);
            }
            info!("Added command to the event loop: {:?}", err);
        },
    )?;
    let (tx, rx) = channel();
    thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            info!("Sending message to mqqt: {:?}", msg);
            mqqt.safe_message(msg);
        }
        warn!("MQTT thread stopped")
    });
    let tx_cmd = tx.clone();
    let tx_err = tx.clone();

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
                        let _ = tx_cmd.send(msg);
                    } else {
                        let _ = tx_cmd.send("Error reading Soil sensor values".to_string());
                    }
                }
                Err(e) => {
                    error!("Error with Soil moisture driver: {:?}", e);
                    let _ = tx_cmd.send("Soil sensor is not connected".to_string());
                }
            },

            MqttCommand::ReadBarometer => match bme280_rs.as_mut() {
                Ok(bme280) => {
                    if let Some(msg) = bme280.to_json() {
                        let _ = tx_cmd.send(msg);
                    } else {
                        let _ = tx_cmd.send("Error reading Bme280 sensor values".to_string());
                    }
                }
                Err(e) => {
                    error!("Error with bme280 driver: {:?}", e);
                    let _ = tx_cmd.send("Bme280 sensor is not connected".to_string());
                }
            },
            MqttCommand::AllSemorData => {
                let s = soil_moisture_rs.as_mut();
                let b = bme280_rs.as_mut();

                if let (Ok(moisture), Ok(bme280)) = (s, b) {
                    if let Some(msg) = moisture.to_json() {
                        let _ = tx_cmd.send(msg);
                    } else {
                        let _ = tx_cmd.send("Error reading Soil sensor values".to_string());
                    }
                    if let Some(msg) = bme280.to_json() {
                        let _ = tx_cmd.send(msg);
                    } else {
                        let _ = tx_cmd.send("Error reading Bme280 sensor values".to_string());
                    }
                } else {
                    let _ = tx_cmd.send("Sensors are not connected".to_string());
                }
            }
        }
    });
    let _error_sub = event_loop.subscribe(move |err: &SimplCommandError| match err {
        SimplCommandError::InvalidValue => {
            let _ = tx_err.send("Missing or wrong value".to_string());
        }
        SimplCommandError::JsonParseError => {
            let _ = tx_err.send("Invalid Json".to_string());
        }
        SimplCommandError::ParseError => {
            let _ = tx_err.send("Invalid encoding ( utf8 parsing failed )".to_string());
        }
        SimplCommandError::WrongCommand => {
            let _ = tx_err.send("Unknown command".to_string());
        }
    });

    info!("Ready for action!");
    event_loop.spin(None)?;

    Ok(())
}
