use async_lock::RwLock;
use edge_executor::Local;
use esp_idf_hal::{prelude::Peripherals, task::executor::EspExecutor};
use log::{error, info, warn};
use std::{result::Result::Ok, sync::Arc, time::Duration};
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
use sensor::{
    bme280::{get_bme280_sensors, new_bme280},
    soil::SoilMoisture,
    Sensor,
};
use trigger::timer::shedule_event;
use utils::wifi::WifiRelay;

fn main() -> anyhow::Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let wifi = futures::executor::block_on(WifiRelay::new(peripherals.modem))?;
    let wifi_handler = Arc::new(RwLock::new(wifi));

    // Setup sensors
    let bme280_i2c = new_bme280(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c0,
    );
    let (mut temp_sensor, mut hum_sensor, mut _bar_sensor) = get_bme280_sensors(bme280_i2c);
    let mut soil_sensor = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36)?;

    // Initialize the async executor
    let esp_executor: EspExecutor<'_, 8, Local> = EspExecutor::new();
    let executor = std::rc::Rc::new(esp_executor);

    // Indicator to show that wifi is connected
    let mut green_led = esp_idf_hal::gpio::PinDriver::output(peripherals.pins.gpio4)?;
    let mut red_led = esp_idf_hal::gpio::PinDriver::output(peripherals.pins.gpio0)?;
    let net_indicator = async {
        let mut sleep = trigger::timer::get_timer()?;
        for _ in 0..3 {
            green_led.set_high()?;
            red_led.set_high()?;
            sleep.after(Duration::from_millis(200))?.await;
            green_led.set_low()?;
            red_led.set_low()?;
            sleep.after(Duration::from_millis(200))?.await;
        }
        let mut net_staus = |net_staus: bool| {
            if net_staus {
                green_led.set_high().ok();
                red_led.set_low().ok();
            } else {
                green_led.set_low().ok();
                red_led.set_high().ok();
            }
        };
        let mut rx_net = wifi_handler.read().await.get_reciver();
        while let Ok(value) = rx_net.recv().await {
            net_staus(value);
        }
        Ok::<(), trigger::timer::TimerError>(())
    };

    // Send notification to discord at 8 AM
    let discord_wifi_handler = wifi_handler.clone();
    let discord_executor = executor.clone();
    let discord_notification = shedule_event(|| {
        fn printer<S: Sensor>(s: &mut S) -> String {
            match s.get_measurment() {
                Ok(value) => format!("{:.1}", value),
                Err(_) => "Sensor not connected".to_string(),
            }
        }
        let status = match soil_sensor.get_status() {
            Ok(status) => status.to_string(),
            Err(_) => "Sensor not connected".to_string(),
        };
        let soil = printer(&mut soil_sensor);
        let hum = printer(&mut hum_sensor);
        let temp = printer(&mut temp_sensor);

        let message = format!(
            r#"
                        Good morning! :sun_with_face:
                        Here is the daily report:
                        > Soil moisture: {soil}{}
                        > Soil moisture status: **{status}**
                        > Temperature: **{temp}{}**
                        > Humidity: **{hum}{}**
                        "#,
            soil_sensor.get_unit(),
            temp_sensor.get_unit(),
            hum_sensor.get_unit()
        )
        .replace('\n', r"\n")
        .replace("  ", "");

        let _ = discord_executor.spawn_local_detached(async {
            let wifi = discord_wifi_handler.read().await;
            match wifi.get_inner().is_connected() {
                Ok(true) => {
                    discord_webhook(message).await.ok();
                }
                Ok(false) => {
                    drop(wifi);
                    let mut wifi = discord_wifi_handler.write().await;
                    wifi.reconnect().await.ok();
                    if let Ok(mut sleep) = trigger::timer::get_timer() {
                        if let Ok(s) = sleep.after(Duration::from_secs(3)) {
                            s.await;
                        }
                    }
                    discord_webhook(message).await.ok();
                    wifi.disconnect().await.ok();
                }
                Err(_) => {
                    error!("Wifi handler not awailable");
                }
            }
        });
    });

    // Start the executor with the tasks
    let tasks = vec![
        executor.spawn_local(discord_notification),
        executor.spawn_local(net_indicator),
    ];
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
