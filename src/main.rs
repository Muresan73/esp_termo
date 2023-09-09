use anyhow::{Ok, Result};
use embedded_hal::delay::DelayUs;
use esp_idf_hal::{delay::FreeRtos, prelude::Peripherals};

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use log::{error, info};
use serde_json::json;

// mod mqtt;
mod sensor;
// mod wifi;
use crate::sensor::{new_bme280, SoilMoisture};

fn main() -> Result<()> {
    info!("program started :)");
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let mut soil_moisture = SoilMoisture::new(peripherals.adc1, peripherals.pins.gpio36).unwrap();

    //let mut adc = AdcDriver::new(peripherals.adc1, &Config::new().calibration(true)).unwrap();
    /*let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<'_, Gpio5, Atten11dB<_>> =
    AdcChannelDriver::new(peripherals.pins.gpio5).unwrap();*/
    //let x = MyStruct::new(adc);

    loop {
        //println!("ADC value: {}", adc.read(&mut adc_pin).unwrap());
        println!(
            "Moisture level: {}",
            soil_moisture.get_soil_status().unwrap()
        );
        println!(
            "Moisture level: {}%",
            soil_moisture.get_moisture_precentage().unwrap()
        );
        FreeRtos.delay_ms(1000u32);
    }

    // let _wifi = wifi::connect(peripherals.modem)?;

    // let mut mqqt = new_mqqt_client(|cmd| match cmd {
    //     MqttCommand::Water(on_off) => info!("Turn on water: {on_off}"),
    //     MqttCommand::Lamp(percent) => info!("Set lamp dim to: {percent}"),
    // })?;
    // let mut bme280 = new_bme280(
    //     peripherals.pins.gpio21,
    //     peripherals.pins.gpio22,
    //     peripherals.i2c0,
    // )
    // .map_err(|err| {
    //     error!("Sensors are not connected");
    //     if mqqt
    //         .message("Bme280 sensor is not connected".into())
    //         .is_err()
    //     {
    //         error!("Bme280 sensor is not connected");
    //     }
    //     err
    // })?;

    // info!("Ready to broadcast ...");

    // for _ in 1..5 {
    //     // 5. This loop initiates measurements, reads values and prints humidity in % and Temperature in °C.
    //     FreeRtos.delay_ms(100u32);
    //     use std::result::Result::Ok;

    //     if let (Ok(Some(pressure)), Ok(Some(temperature)), Ok(Some(humidity))) = (
    //         bme280.read_pressure(),
    //         bme280.read_temperature(),
    //         bme280.read_humidity(),
    //     ) {
    //         // All sensor readings are available and valid

    //         let json = json!( {
    //             "measurements": [ {
    //                 "type":"pressure",
    //                 "value": pressure,
    //                 "unit": "Pa"

    //             },
    //             {
    //                 "type":"temperature",
    //                 "value": temperature,
    //                 "unit": "°C"
    //             },
    //             {
    //                 "type":"humidity",
    //                 "value": humidity,
    //                 "unit": "%"
    //             }]
    //         });
    //         mqqt.message(json.to_string())?;
    //     } else {
    //         // Handle the case where one or more sensors are not connected or readings are invalid
    //         error!("Sensors are not connected or readings are invalid");
    //         mqqt.message("Sensors are not connected or readings are invalid".to_string())?;
    //     }

    //     info!("Waiting 5 seconds");
    //     FreeRtos.delay_ms(5000u32);
    // }
    Ok(())
}
