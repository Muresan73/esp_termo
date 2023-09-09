use anyhow::Result;
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

pub fn new_bme280<I2C: I2c>(
    sda: impl Peripheral<P = impl InputPin + OutputPin> + 'static,
    scl: impl Peripheral<P = impl InputPin + OutputPin> + 'static,
    i2c_pin: impl Peripheral<P = I2C> + 'static,
) -> Result<Bme280<I2cDriver<'static>, Delay>, anyhow::Error> {
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

// ====================
// Soil Moisture Sensor
// ====================

const MAX_DRY: u16 = 2800;
const MAX_WET: u16 = 1300;

const MOISTURE_RANGE: u16 = MAX_DRY - MAX_WET;
const FULL_PRECENTAGE: f32 = 100.0;
const NO_PRECENTAGE: f32 = 0.0;

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
    pub fn get_soil_status(&mut self) -> MoistureResult<SoilStatus> {
        let percentage = self.get_moisture_precentage()?;

        match percentage {
            p if p < 20.0 => Ok(SoilStatus::Dry),
            p if p < 40.0 => Ok(SoilStatus::Optimal),
            p if p < 55.0 => Ok(SoilStatus::Damp),
            _ => Ok(SoilStatus::Wet),
        }
    }
}
