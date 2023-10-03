use anyhow::Result;

use async_lock::Mutex;
use edge_executor::Local;
use esp_idf_hal::{prelude::Peripherals, task::executor::EspExecutor};
use esp_idf_svc::eventloop::EspBackgroundEventLoop;
use log::{error, info};
use std::result::Result::Ok;
use std::sync::Arc;
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
use trigger::timer::shedule_event;
use utils::wifi;

use crate::{trigger::timer::showtime, utils::wifi::reconnect};

fn main() -> Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let wifi = futures::executor::block_on(wifi::connect(peripherals.modem))?;
    let wifi_handler = Arc::new(Mutex::new(wifi));

    #[cfg(feature = "sensor")]
    {
        info!("Setup sensors");

        let mut bme280_rs = new_bme280(
            peripherals.pins.gpio21,
            peripherals.pins.gpio22,
            peripherals.i2c0,
        );

        let mut soil_moisture = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36)?;

        let mut bme280 = bme280_rs?;

        let discord_wifi_handler = wifi_handler.clone();
        let discord_task = shedule_event(move || {
            discord_wifi_handler.lock().unwrap().is_connected();

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
        });
    }

    info!("Fun about to begin ...");
    let mut green_led = esp_idf_hal::gpio::PinDriver::output(peripherals.pins.gpio4)?;
    let mut red_led = esp_idf_hal::gpio::PinDriver::output(peripherals.pins.gpio0)?;

    use std::time::Duration;
    std::thread::sleep(Duration::from_secs(3));

    let executor: EspExecutor<'_, 8, Local> = EspExecutor::new();

    let task = executor.spawn(shedule_event(showtime))?;
    let testtask = executor.spawn_local(async {
        let mut sleep = trigger::timer::get_timer()?;

        for _ in 0..3 {
            green_led.set_high()?;
            red_led.set_high()?;
            sleep.after(Duration::from_secs(1))?.await;
            green_led.set_low()?;
            red_led.set_low()?;
            sleep.after(Duration::from_secs(1))?.await;
        }

        let mut net_staus = |net_staus: bool| {
            if net_staus {
                green_led.set_high();
                red_led.set_low();
            } else {
                green_led.set_low();
                red_led.set_high();
            }
        };

        loop {
            {
                let wifi = wifi_handler.lock().await;
                net_staus(wifi.is_connected()?);
                net_staus(true);
            }

            let res = discord_webhook(String::from("timout")).await;
            if res.is_err() {
                net_staus(false);
            }

            showtime();
            {
                let mut wifi = wifi_handler.lock().await;
                wifi.disconnect().await?;
                net_staus(false);
                utils::power::enter_light_sleep(chrono::Duration::hours(1).to_std().unwrap());
                reconnect(&mut wifi).await?;
                net_staus(wifi.is_connected()?);
                info!("Wifi connected");
                sleep.after(Duration::from_secs(2))?.await;
            }

            showtime();
        }
    })?;
    let tasks = vec![executor.spawn(task), executor.spawn(testtask)];

    executor.run_tasks(|| true, tasks);

    log::warn!("Tasks completed");
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
