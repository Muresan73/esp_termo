use bme280_rs::{Bme280, Configuration, Oversampling, SensorMode};
use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::{
    adc::{config::Config, AdcChannelDriver, AdcDriver, Atten11dB, ADC1},
    gpio::ADCPin,
    peripheral::Peripheral,
};
use esp_idf_hal::{
    i2c::{I2c, I2cConfig, I2cDriver},
    prelude::*,
};

use esp_idf_sys::EspError;
use log::{error, info};
use serde_json::json;

pub trait MessageAble<T>
where
    T: std::fmt::Display,
{
    fn to_json(&mut self) -> Option<String> {
        {
            let json_vec: Option<Vec<_>> = self
                .get_measurment_vec()
                .iter()
                .map(|(m_type, unit, measurement, status)| {
                    if let (Some(msrmnt), Some(stts)) = (measurement, status) {
                        Some(json!( {
                                "type":m_type,
                                "value": msrmnt,
                                "status": stts.to_string(),
                                "unit": unit
                        }))
                    } else {
                        log::warn!("Error reading {m_type}");
                        None
                    }
                })
                .collect();

            if let Some(json_vec) = json_vec {
                Some(json!({ "measurements": json_vec }).to_string())
            } else {
                error!("Sensors are not connected or readings are invalid");
                None
            }
        }
    }

    fn get_measurment_vec(&mut self) -> Vec<(&str, &str, Option<f32>, Option<T>)>;
}

// ================
// BME280 Sensor
// ================

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

// ====================
// Soil Moisture Sensor
// ====================

const MAX_DRY: u16 = 2800;
const MAX_WET: u16 = 1300;

const MOISTURE_RANGE: u16 = MAX_DRY - MAX_WET;
const FULL_PRECENTAGE: f32 = 100.0;
const NO_PRECENTAGE: f32 = 0.0;

#[derive(Clone)]
pub enum SoilStatus {
    Dry,
    Optimal,
    Damp,
    Wet,
}

impl std::fmt::Display for SoilStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SoilStatus::Dry => write!(f, "Dry🔥‼️"),
            SoilStatus::Optimal => write!(f, "Optimal 💚"),
            SoilStatus::Damp => write!(f, "Damp ⚠️"),
            SoilStatus::Wet => write!(f, "Wet 💦"),
        }
    }
}

type MoistureResult<T> = Result<T, EspError>;

pub struct SoilMoisture<'d, T: ADCPin> {
    adc_driver: AdcDriver<'d, ADC1>,
    adc_pin: AdcChannelDriver<'d, T, Atten11dB<ADC1>>,
}

impl<'d, T: ADCPin> SoilMoisture<'d, T>
where
    T: ADCPin<Adc = ADC1>,
{
    /// adc -> the adc from the peripherals
    /// pin -> gpio from peripherals pins that is connected
    pub fn new(adc: ADC1, pin: impl Peripheral<P = T> + 'd) -> MoistureResult<Self> {
        let adc = AdcDriver::new(adc, &Config::new().calibration(true))?;
        let adc_pin: AdcChannelDriver<'_, _, Atten11dB<_>> = AdcChannelDriver::new(pin)?;
        Ok(SoilMoisture {
            adc_driver: adc,
            adc_pin,
        })
    }

    /// Get the raw read of the moisture result, analog read
    pub fn get_raw_moisture(&mut self) -> MoistureResult<u16> {
        self.adc_driver.read(&mut self.adc_pin)
    }

    /// Get precentage read of the moisture.
    pub fn get_moisture_precentage(&mut self) -> MoistureResult<f32> {
        let raw_read = self.get_raw_moisture()?;

        if raw_read > MAX_DRY {
            return Ok(NO_PRECENTAGE);
        } else if raw_read < MAX_WET {
            return Ok(FULL_PRECENTAGE);
        }

        let value_diff = MAX_DRY - raw_read;
        Ok((value_diff as f32 / MOISTURE_RANGE as f32) * FULL_PRECENTAGE)
    }

    /// Get the status of the soil
    /// Dry -> 0-20%
    /// Optimal -> 20-40%
    /// Da -> 40-55%
    /// Wet -> 55-100%
    pub fn get_soil_status(&mut self) -> Option<SoilStatus> {
        let percentage = self.get_moisture_precentage().ok()?;

        match percentage {
            p if p < 20.0 => Some(SoilStatus::Dry),
            p if p < 40.0 => Some(SoilStatus::Optimal),
            p if p < 55.0 => Some(SoilStatus::Damp),
            _ => Some(SoilStatus::Wet),
        }
    }
}

impl<T> MessageAble<SoilStatus> for SoilMoisture<'_, T>
where
    T: ADCPin<Adc = ADC1>,
{
    fn get_measurment_vec(&mut self) -> Vec<(&str, &str, Option<f32>, Option<SoilStatus>)> {
        [(
            "soil-moisture",
            "%",
            self.get_moisture_precentage().ok(),
            self.get_soil_status(),
        )]
        .to_vec()
    }
}
