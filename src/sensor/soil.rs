use esp_idf_hal::{
    adc::{config::Config, AdcChannelDriver, AdcDriver, Atten11dB, ADC1},
    gpio::ADCPin,
    peripheral::Peripheral,
};

use super::*;
use esp_idf_sys::EspError;
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
#[derive(Debug, thiserror::Error)]
pub enum MoistureError {
    #[error("Sensor not connected")]
    SensorNotConnected(),
    #[error("EspError internal error")]
    EspError(#[from] EspError),
}
type MoistureResult<T> = Result<T, MoistureError>;

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
        let measurement = self.adc_driver.read(&mut self.adc_pin)?;
        match measurement {
            msmnt if msmnt < 1000 => Err(MoistureError::SensorNotConnected()),
            msmnt => Ok(msmnt),
        }
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
