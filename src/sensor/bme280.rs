use std::sync::{Arc, Mutex};

use bme280_rs::{Bme280, Configuration, Oversampling, SensorMode};
use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::i2c::I2cError;
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
    I2cDriver(#[from] EspError),
    #[error("sensor init failed")]
    SensorInit(#[from] I2cError),
    #[error("sensor not connected")]
    SensorNotConnected(),
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

pub fn get_bme280_sensors(
    bme280_rs: Result<Bme280<I2cDriver<'static>, Delay>, Bme280Error>,
) -> (Bme280TempSensor, Bme280HumiditySensor, Bme280PressureSensor) {
    match bme280_rs {
        Ok(bme280_origin) => {
            let bme280 = Arc::new(Mutex::new(bme280_origin));

            let bme280_temp_sensor = Bme280TempSensor {
                bme280: Some(bme280.clone()),
                ..Default::default()
            };
            let bme280_humidity_sensor = Bme280HumiditySensor {
                bme280: Some(bme280.clone()),
                ..Default::default()
            };
            let bme280_pressure_sensor = Bme280PressureSensor {
                bme280: Some(bme280.clone()),
                ..Default::default()
            };
            (
                bme280_temp_sensor,
                bme280_humidity_sensor,
                bme280_pressure_sensor,
            )
        }
        Err(_) => (
            Bme280TempSensor::default(),
            Bme280HumiditySensor::default(),
            Bme280PressureSensor::default(),
        ),
    }
}

pub struct Bme280TempSensor {
    bme280: Option<Arc<Mutex<Bme280<I2cDriver<'static>, Delay>>>>,
    unit: &'static str,
    name: &'static str,
}
impl Default for Bme280TempSensor {
    fn default() -> Self {
        Self {
            bme280: None,
            unit: "°C",
            name: "temperature",
        }
    }
}
pub enum TempStatus {
    Freezing,
    Cold,
    Optimal,
    Hot,
}

impl Sensor for Bme280TempSensor {
    type Error = Bme280Error;
    type Status = TempStatus;

    fn get_measurment(&mut self) -> Result<f32, Self::Error> {
        let bme280 = self
            .bme280
            .as_mut()
            .ok_or(Self::Error::SensorNotConnected())?;

        let mut bme280 = bme280.lock().or(Err(Self::Error::SensorNotConnected()))?;

        bme280
            .read_temperature()?
            .ok_or(Self::Error::SensorNotConnected())
    }

    fn get_status(&mut self) -> Result<Self::Status, Self::Error> {
        let temp = self.get_measurment()?;
        match temp {
            t if t < 0.0 => Ok(TempStatus::Freezing),
            t if t < 18.0 => Ok(TempStatus::Cold),
            t if t < 25.0 => Ok(TempStatus::Optimal),
            _ => Ok(TempStatus::Hot),
        }
    }

    fn get_unit(&self) -> &str {
        self.unit
    }

    fn get_name(&self) -> &str {
        self.name
    }
}

pub struct Bme280HumiditySensor {
    bme280: Option<Arc<Mutex<Bme280<I2cDriver<'static>, Delay>>>>,
    unit: &'static str,
    name: &'static str,
}
impl Default for Bme280HumiditySensor {
    fn default() -> Self {
        Self {
            bme280: None,
            unit: "%",
            name: "humidity",
        }
    }
}
pub enum HumidityStatus {
    Dry,
    Optimal,
    Moist,
    Wet,
}
impl Sensor for Bme280HumiditySensor {
    type Error = Bme280Error;
    type Status = HumidityStatus;

    fn get_measurment(&mut self) -> Result<f32, Self::Error> {
        let bme280 = self
            .bme280
            .as_mut()
            .ok_or(Self::Error::SensorNotConnected())?;

        let mut bme280 = bme280.lock().or(Err(Self::Error::SensorNotConnected()))?;

        bme280
            .read_humidity()?
            .ok_or(Self::Error::SensorNotConnected())
    }

    fn get_status(&mut self) -> Result<Self::Status, Self::Error> {
        let humidity = self.get_measurment()?;
        match humidity {
            h if h < 30.0 => Ok(HumidityStatus::Dry),
            h if h < 50.0 => Ok(HumidityStatus::Optimal),
            h if h < 70.0 => Ok(HumidityStatus::Moist),
            _ => Ok(HumidityStatus::Wet),
        }
    }

    fn get_unit(&self) -> &str {
        self.unit
    }

    fn get_name(&self) -> &str {
        self.name
    }
}

pub struct Bme280PressureSensor {
    bme280: Option<Arc<Mutex<Bme280<I2cDriver<'static>, Delay>>>>,
    unit: &'static str,
    name: &'static str,
}
impl Default for Bme280PressureSensor {
    fn default() -> Self {
        Self {
            bme280: None,
            unit: "hPa",
            name: "pressure",
        }
    }
}
pub enum PressureStatus {
    Low,
    Optimal,
    High,
}
impl Sensor for Bme280PressureSensor {
    type Error = Bme280Error;
    type Status = PressureStatus;

    fn get_measurment(&mut self) -> Result<f32, Self::Error> {
        let bme280 = self
            .bme280
            .as_mut()
            .ok_or(Self::Error::SensorNotConnected())?;

        let mut bme280 = bme280.lock().or(Err(Self::Error::SensorNotConnected()))?;

        bme280
            .read_pressure()?
            .ok_or(Self::Error::SensorNotConnected())
    }

    fn get_status(&mut self) -> Result<Self::Status, Self::Error> {
        let pressure = self.get_measurment()?;
        match pressure {
            p if p < 1000.0 => Ok(PressureStatus::Low),
            p if p < 1013.0 => Ok(PressureStatus::Optimal),
            _ => Ok(PressureStatus::High),
        }
    }

    fn get_unit(&self) -> &str {
        self.unit
    }

    fn get_name(&self) -> &str {
        self.name
    }
}
