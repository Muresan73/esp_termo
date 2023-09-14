use bme280_rs::{Bme280, Configuration, Oversampling, SensorMode};
use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::{
    i2c::{I2c, I2cConfig, I2cDriver},
    prelude::*,
};

use esp_idf_sys::EspError;
use log::{error, info};

use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Bme280Error {
    #[error("i2c driver init failed")]
    I2cDriverError(#[from] EspError),
    #[error("sensor init failed")]
    SensorInitError(#[from] esp_idf_hal::i2c::I2cError),
}

pub fn new_bme280<I2C: I2c>(
    sda: impl Peripheral<P = impl InputPin + OutputPin> + 'static,
    scl: impl Peripheral<P = impl InputPin + OutputPin> + 'static,
    i2c_pin: impl Peripheral<P = I2C> + 'static,
) -> Result<Bme280<I2cDriver<'static>, Delay>, Bme280Error> {
    // 1. Instanciate the SDA and SCL pins, correct pins are in the training material.
    // 2. Instanciate the i2c peripheral
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c_pin, sda, scl, &config)?;

    // 3. Create an instance of the bme280 sensor.
    let delay = Delay;
    let mut bme280 = Bme280::new(i2c, delay);
    bme280.init()?;

    // 4. Read and print the sensor's device ID.
    match bme280.chip_id() {
        Ok(id) => {
            info!("Device ID BME280: {:#02x}", id);
        }
        Err(e) => {
            error!("{:?}", e);
        }
    };

    let configuration = Configuration::default()
        .with_temperature_oversampling(Oversampling::Oversample1)
        .with_pressure_oversampling(Oversampling::Oversample1)
        .with_humidity_oversampling(Oversampling::Oversample1)
        .with_sensor_mode(SensorMode::Normal);
    bme280.set_sampling_configuration(configuration)?;
    Ok(bme280)
}

pub trait Bme280Extention {
    fn read_temperature_status(&mut self) -> Option<String>;
    fn read_pressure_status(&mut self) -> Option<String>;
    fn read_humidity_status(&mut self) -> Option<String>;
}

impl Bme280Extention for Bme280<I2cDriver<'static>, Delay> {
    fn read_temperature_status(&mut self) -> Option<String> {
        let temp = self.read_temperature().ok()??;
        match temp {
            t if t < 0.0 => Some("Freezing".to_string()),
            t if t < 18.0 => Some("Cold".to_string()),
            t if t < 25.0 => Some("Optimal".to_string()),
            _ => Some("Hot".to_string()),
        }
    }
    fn read_humidity_status(&mut self) -> Option<String> {
        let humidity = self.read_humidity().ok()??;
        match humidity {
            h if h < 30.0 => Some("Dry".to_string()),
            h if h < 50.0 => Some("Optimal".to_string()),
            h if h < 70.0 => Some("Moist".to_string()),
            _ => Some("Wet".to_string()),
        }
    }
    fn read_pressure_status(&mut self) -> Option<String> {
        let pressure = self.read_pressure().ok()??;
        match pressure {
            p if p < 1000.0 => Some("Low".to_string()),
            p if p < 1013.0 => Some("Optimal".to_string()),
            _ => Some("High".to_string()),
        }
    }
}

impl MessageAble<String> for Bme280<I2cDriver<'static>, Delay> {
    fn get_measurment_vec(&mut self) -> Vec<(&str, &str, Option<f32>, Option<String>)> {
        [
            (
                "temperature",
                "°C",
                self.read_temperature().unwrap_or(None),
                self.read_temperature_status(),
            ),
            (
                "humidity",
                "%",
                self.read_humidity().unwrap_or(None),
                self.read_humidity_status(),
            ),
            (
                "pressure",
                "hPa",
                self.read_pressure().unwrap_or(None),
                self.read_pressure_status(),
            ),
        ]
        .to_vec()
    }
}
