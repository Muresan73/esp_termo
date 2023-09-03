use anyhow::Result;
use bme280_rs::{Bme280, Configuration, Oversampling, SensorMode};
use embedded_hal::delay::DelayUs;
use esp_idf_hal::delay::Delay;
use esp_idf_hal::{
    delay::FreeRtos,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    prelude::*,
};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    println!("program started :)");

    let peripherals = Peripherals::take().unwrap();

    // 1. Instanciate the SDA and SCL pins, correct pins are in the training material.
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    // 2. Instanciate the i2c peripheral
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    // 3. Create an instance of the bme280 sensor.
    let delay = Delay;
    let mut bme280 = Bme280::new(i2c, delay);
    let is_init = bme280.init();

    if let Err(error) = is_init {
        println!("{:?}", error)
    } else {
        // 4. Read and print the sensor's device ID.
        match bme280.chip_id() {
            Ok(id) => {
                println!("Device ID BME280: {:#02x}", id);
            }
            Err(e) => {
                print!("{:?}", e);
            }
        };

        let configuration = Configuration::default()
            .with_temperature_oversampling(Oversampling::Oversample1)
            .with_pressure_oversampling(Oversampling::Oversample1)
            .with_humidity_oversampling(Oversampling::Oversample1)
            .with_sensor_mode(SensorMode::Normal);
        bme280.set_sampling_configuration(configuration)?;
    }

    loop {
        // 5. This loop initiates measurements, reads values and prints humidity in % and Temperature in °C.
        FreeRtos.delay_ms(100u32);

        if let Some(humidity) = bme280.read_humidity()? {
            println!("Humidity: {:.2} %", humidity);
        } else {
            println!("Humidity reading was disabled");
        }
        if let Some(temperature) = bme280.read_temperature()? {
            println!("Temperature: {} C", temperature);
        } else {
            println!("Temperature reading was disabled");
        }
        if let Some(pressure) = bme280.read_pressure()? {
            println!("Pressure: {:.2} Pa", pressure);
        } else {
            println!("Pressure reading was disabled");
        }
        println!();
        FreeRtos.delay_ms(5000u32);
    }
}
