use std::fmt::Display;

use log::error;
use serde_json::{json, Value};

pub mod bme280;
pub mod soil;

pub trait MessageAble {
    fn to_json(&mut self) -> Value;
}

pub trait Sensor {
    type Error;
    type Status;

    fn get_unit(&self) -> &str;
    fn get_name(&self) -> &str;

    fn get_measurment(&mut self) -> Result<f32, Self::Error>;
    fn get_status(&mut self) -> Result<Self::Status, Self::Error>;
}

impl<ST, E, S> MessageAble for S
where
    ST: Display,
    E: std::fmt::Debug,
    S: Sensor<Error = E, Status = ST>,
{
    fn to_json(&mut self) -> Value {
        if let (Ok(stat), Ok(msrmnt)) = (self.get_status(), self.get_measurment()) {
            json!( {
                    "type":self.get_name(),
                    "value": msrmnt,
                    "status": stat.to_string(),
                    "unit": self.get_unit()
            })
        } else {
            json!( {
                    "type":self.get_name(),
                    "value": self.get_measurment().unwrap(),
                    "status": "Not connected",
            })
        }
    }
}
