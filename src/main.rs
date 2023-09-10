use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspBackgroundEventLoop;
use log::{error, info};
use serde_json::json;
use std::result::Result::Ok;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod mqtt;
mod sensor;
mod wifi;
use crate::{
    mqtt::{new_mqqt_client, MqttCommand, SimpleMqttClient},
    sensor::{new_bme280, SoilMoisture},
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
    let mut bme280 = new_bme280(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c0,
    );

    let mut soil_moisture = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36);

    info!("Setup background event loop");
    // this let variable is necessary so that the Subscription does not get dropped
    let _subscription = event_loop.subscribe(move |message: &MqttCommand| {
        info!("Got message from the event loop: {:?}", message);
        match message {
            MqttCommand::Water(on_off) => info!("Turn on water: {on_off}"),
            MqttCommand::Lamp(percent) => info!("Set lamp dim to: {percent}"),
            MqttCommand::ReadSoilMoisture => {
                if soil_moisture.is_err() {
                    mqqt.safe_message("Error reading soil".to_string());
                    return;
                };

                if let (Ok(perc), Ok(status)) = (
                    soil_moisture.as_mut().unwrap().get_moisture_precentage(),
                    soil_moisture.as_mut().unwrap().get_soil_status(),
                ) {
                    let json = json!( {
                        "measurements": [ {
                            "type":"soil",
                            "value": perc,
                            "status": status.to_string(),
                            "unit": "%"
                        }]
                    });
                    mqqt.safe_message(json.to_string());
                } else {
                    error!("Soil sensor is not connected");
                    mqqt.safe_message("Soil sensor is not connected".to_string())
                }
            }
            MqttCommand::ReadBarometer => {
                if bme280.is_err() {
                    mqqt.safe_message("Error reading barometer".to_string());
                    return;
                };

                if let (Ok(Some(pressure)), Ok(Some(temperature)), Ok(Some(humidity))) = (
                    bme280.as_mut().unwrap().read_pressure(),
                    bme280.as_mut().unwrap().read_temperature(),
                    bme280.as_mut().unwrap().read_humidity(),
                ) {
                    let json = json!( {
                        "measurements": [ {
                            "type":"pressure",
                            "value": pressure,
                            "unit": "Pa"

                        },
                        {
                            "type":"temperature",
                            "value": temperature,
                            "unit": "°C"
                        },
                        {
                            "type":"humidity",
                            "value": humidity,
                            "unit": "%"
                        }]
                    });
                    mqqt.safe_message(json.to_string());
                } else {
                    // Handle the case where one or more sensors are not connected or readings are invalid
                    error!("Sensors are not connected or readings are invalid");
                    mqqt.safe_message(
                        "Sensors are not connected or readings are invalid".to_string(),
                    );
                }
            }
        }
    });

    info!("Ready for action!");
    event_loop.spin(None)?;

    Ok(())
}
