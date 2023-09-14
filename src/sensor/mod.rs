use log::error;
use serde_json::json;

pub mod bme280;
pub mod soil;

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
