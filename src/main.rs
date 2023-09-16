use anyhow::Result;

use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspBackgroundEventLoop;
use futures::executor::block_on;
use log::{error, info};
use std::result::Result::Ok;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod relay;
mod sensor;
mod trigger;
mod utils;
use relay::{
    discord::discord_webhook,
    mqtt::{new_mqqt_client, Command, SimplCommandError, SimpleMqttClient},
};
use sensor::{bme280::new_bme280, soil::SoilMoisture, MessageAble};
use trigger::timer::get_time;
use utils::wifi;

fn main() -> Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let _wifi = wifi::connect(peripherals.modem)?;

    info!("Setup sensors");

    let mut bme280_rs = new_bme280(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c0,
    );

    let mut soil_moisture = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36)?;

    #[cfg(not(feature = "mqtt"))]
    {
        let mut bme280 = bme280_rs?;

        let _timer = block_on(get_time(move || {
            let percent = soil_moisture.get_moisture_precentage();
            let status = soil_moisture.get_soil_status();
            let hum = bme280.read_humidity().unwrap().unwrap();
            let temp = bme280.read_temperature().unwrap().unwrap();
            match (status, percent) {
                (Some(status), Ok(percent)) if status == sensor::soil::SoilStatus::Dry => {
                    let message = format!(
                        r#"
                        Warning!!! :warning:
                        Plants need to be watered!
                        > Soil moisture: **{percent:.1}%**
                        > Soil moisture status: **{status}**
                        > Temperature: **{temp:.1}°C**
                        > Humidity: **{hum:.1}%**
                        "#
                    )
                    .replace('\n', r"\n")
                    .replace("  ", "");
                    let _ = discord_webhook(message);
                }
                (Some(status), Ok(percent)) => {
                    let message = format!(
                        r#"
                        Good morning! :sun_with_face:
                        Here is the daily report:
                        > Soil moisture: {percent:.1}%
                        > Soil moisture status: **{status}**
                        > Temperature: **{temp:.1}°C**
                        > Humidity: **{hum:.1}%**
                        "#
                    )
                    .replace('\n', r"\n")
                    .replace("  ", "");
                    let _ = discord_webhook(message);
                }
                _ => {
                    let message = format!(
                        r#"
                        Bit of a problem! :warning:
                        Sensore not connected
                        "#
                    )
                    .replace('\n', r"\n")
                    .replace("  ", "");
                    let _ = discord_webhook(message);
                }
            };
        }));
    }

    #[cfg(feature = "mqtt")]
    {
        let mut event_loop = EspBackgroundEventLoop::new(&Default::default())?;
        let cmd_loop = event_loop.clone();
        let mqqt_service = new_mqqt_client(move |msg| {
            let _ = match msg {
                Ok(cmd) => cmd_loop.post(&cmd, None),
                Err(err) => cmd_loop.post::<SimplCommandError>(&err.into(), None),
            }
            .map_err(|err| {
                error!("Error posting: {:?}", err);
                err
            });
        })?;

        use std::sync::{Arc, Mutex};
        let mqtt_client = Arc::new(Mutex::new(mqqt_service));
        let mqtt_err = mqtt_client.clone();
        info!("Ready to broadcast ...");

        info!("Setup background event loop");
        // this let variable is necessary so that the Subscription does not get dropped
        let _subscription = event_loop.subscribe(move |message: &Command| {
            info!("Got message from the event loop: {:?}", message);
            match message {
                Command::Water(on_off) => info!("Turn on water: {on_off}"),
                Command::Lamp(percent) => info!("Set lamp dim to: {percent}"),
                Command::ReadSoilMoisture => {
                    if let Ok(mut mqtt) = mqtt_client.lock() {
                        match soil_moisture.to_json() {
                            Some(msg) => mqtt.safe_message(msg),
                            None => mqtt
                                .error_message("soil moisture sensor is not connected".to_string()),
                        }
                    }
                }
                Command::ReadBarometer => {
                    if let Ok(mut mqtt) = mqtt_client.lock() {
                        match bme280_rs.as_mut() {
                            Ok(bme280) => match bme280.to_json() {
                                Some(msg) => mqtt.safe_message(msg),
                                None => mqtt.error_message(
                                    "soil moisture sensor is not connected".to_string(),
                                ),
                            },
                            Err(e) => {
                                error!("Error with bme280 driver: {:?}", e);
                                mqtt.error_message("Bme280 sensor is not connected".to_string());
                            }
                        }
                    }
                }
                Command::AllSemorData => {
                    todo!("implement all sensor data")
                }
            }
        });
        let _error_sub = event_loop.subscribe(move |err: &SimplCommandError| {
            if let Ok(mut mqtt) = mqtt_err.lock() {
                match err {
                    SimplCommandError::InvalidValue => {
                        mqtt.error_message("Missing or wrong value".to_string());
                    }
                    SimplCommandError::JsonParseError => {
                        mqtt.error_message("Invalid Json".to_string());
                    }
                    SimplCommandError::ParseError => {
                        mqtt.error_message("Invalid encoding ( utf8 parsing failed )".to_string());
                    }
                    SimplCommandError::WrongCommand => {
                        mqtt.error_message("Unknown command".to_string());
                    }
                };
            }
        });

        info!("Ready for action!");
        event_loop.spin(None)?;
    }
    // do not deallocate the event_loop after main() returns
    // core::mem::forget(event_loop);

    Ok(())
}
