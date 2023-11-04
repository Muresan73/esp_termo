#![feature(never_type)]
use async_lock::RwLock;
use esp_idf_hal::{gpio::PinDriver, prelude::Peripherals, task::block_on};
use futures::join;
use log::{error, info, warn};
use std::{rc::Rc, result::Result::Ok, time::Duration};

use edge_executor::LocalExecutor;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod relay;
mod sensor;
mod trigger;
mod utils;

use relay::discord::discord_webhook;
use sensor::{
    bme280::{get_bme280_sensors, new_bme280},
    soil::SoilMoisture,
};
use trigger::timer::shedule_event;
use utils::{helper::discord::get_message, wifi::WifiRelay};

fn main() -> anyhow::Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    // Setup wifi
    let wifi = block_on(WifiRelay::new(peripherals.modem))?;
    let wifi_handler = Rc::new(RwLock::new(wifi));

    // Setup sensors
    let bme280_i2c = new_bme280(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c0,
    );
    let (mut temp_sensor, mut hum_sensor, mut _bar_sensor) = get_bme280_sensors(bme280_i2c);
    let mut soil_sensor = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36)?;

    // Initialize the async executor
    let executor: LocalExecutor = Default::default();

    let mut pump_relay = PinDriver::output(peripherals.pins.gpio13)?;
    pump_relay.set_low().ok();
    let pump = async {
        let delay_service = trigger::timer::get_timer().unwrap();
        let mut timer = delay_service.timer().unwrap();

        loop {
            pump_relay.set_high().ok();
            timer.after(Duration::from_secs(1)).await.ok();
            pump_relay.set_low().ok();
            timer.after(Duration::from_secs(5)).await.ok();
        }
    };

    // Send notification to discord at 8 AM
    let discord_wifi_handler = wifi_handler.clone();
    let discord_notification = shedule_event(|| {
        let message = get_message(&mut soil_sensor, &mut hum_sensor, &mut temp_sensor);

        executor
            .spawn(async {
                let wifi = discord_wifi_handler.read().await;
                match wifi.get_inner().is_connected() {
                    Ok(true) => {
                        discord_webhook(message).await.ok();
                    }
                    Ok(false) => {
                        drop(wifi);
                        let mut wifi = discord_wifi_handler.write().await;
                        wifi.reconnect().await.ok();
                        trigger::timer::safe_sleep(Duration::from_secs(3)).await;
                        discord_webhook(message).await.ok();
                        wifi.disconnect().await.ok();
                    }
                    Err(_) => {
                        error!("Wifi handler not awailable");
                    }
                }
            })
            .detach();
    });

    // Start the executor with the tasks
    block_on(executor.run(async {
        let _ = join!(executor.spawn(discord_notification), executor.spawn(pump));
    }));

    warn!("Tasks completed");

    Ok(())
}
