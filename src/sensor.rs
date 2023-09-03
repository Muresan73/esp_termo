use anyhow::Result;
use bme280_rs::{Bme280, Configuration, Oversampling, SensorMode};
use esp_idf_hal::delay::Delay;
use esp_idf_hal::{
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    prelude::*,
};

pub fn new_bme280() -> Result<Bme280<I2cDriver<'static>, Delay>, anyhow::Error> {
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
    Ok(bme280)
}
