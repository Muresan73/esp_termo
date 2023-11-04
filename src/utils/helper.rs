pub mod discord {
    use esp_idf_hal::{adc::Adc, gpio::ADCPin};

    use crate::sensor::{
        bme280::{Bme280HumiditySensor, Bme280TempSensor},
        soil::SoilMoisture,
        Sensor,
    };

    pub fn get_message<T: ADCPin, ADC: Adc>(
        soil_sensor: &mut SoilMoisture<'_, T, ADC>,
        hum_sensor: &mut Bme280HumiditySensor,
        temp_sensor: &mut Bme280TempSensor,
    ) -> String
    where
        T: ADCPin<Adc = ADC>,
    {
        fn printer<S: Sensor>(s: &mut S) -> String {
            match s.get_measurment() {
                Ok(value) => format!("{:.1}", value),
                Err(_) => "Sensor not connected".to_string(),
            }
        }
        let status = match soil_sensor.get_status() {
            Ok(status) => status.to_string(),
            Err(_) => "Sensor not connected".to_string(),
        };
        let soil = printer(soil_sensor);
        let hum = printer(hum_sensor);
        let temp = printer(temp_sensor);

        format!(
            r#"
                        Good morning! :sun_with_face:
                        Here is the daily report:
                        > Soil moisture: {soil}{}
                        > Soil moisture status: **{status}**
                        > Temperature: **{temp}{}**
                        > Humidity: **{hum}{}**
                        "#,
            soil_sensor.get_unit(),
            temp_sensor.get_unit(),
            hum_sensor.get_unit()
        )
        .replace('\n', r"\n")
        .replace("  ", "")
    }
}

pub mod mqtt {
    use esp_idf_svc::eventloop::EspBackgroundEventLoop;
    use log::{error, info};

    use crate::relay::mqtt::{new_mqqt_client, Command, SimplCommandError, SimpleMqttClient};

    fn setup_mqtt() -> Result<(), anyhow::Error> {
        todo!();
        let mut event_loop = EspBackgroundEventLoop::new(&Default::default())?;
        let cmd_loop = event_loop.clone();
        let mqqt_service = new_mqqt_client(move |msg| {
            let _ = match msg {
                Ok(cmd) => cmd_loop.post(&cmd, None),
                Err(err) => cmd_loop.post::<SimplCommandError>(&err.into(), None),
            }
            .map_err(|err| {
                error!("Error posting: {:?}", err);
                err
            });
        })?;
        use std::sync::{Arc, Mutex};
        let mqtt_client = Arc::new(Mutex::new(mqqt_service));
        let mqtt_err = mqtt_client.clone();
        info!("Ready to broadcast ...");
        info!("Setup background event loop");
        let _subscription = event_loop.subscribe(move |message: &Command| {
            info!("Got message from the event loop: {:?}", message);
            match message {
                Command::Water(on_off) => info!("Turn on water: {on_off}"),
                Command::Lamp(percent) => info!("Set lamp dim to: {percent}"),
                Command::ReadSoilMoisture => {
                    if let Ok(mut mqtt) = mqtt_client.lock() {
                        match Some("soil".to_string()) {
                            //soil_moisture.to_json() {
                            Some(msg) => mqtt.safe_message(msg),
                            None => mqtt
                                .error_message("soil moisture sensor is not connected".to_string()),
                        }
                    };
                }
                Command::ReadBarometer => {
                    if let Ok(mut mqtt) = mqtt_client.lock() {
                        // match bme280_rs.as_mut() {
                        //     Ok(bme280) => match bme280.to_json() {
                        //         Some(msg) => mqtt.safe_message(msg),
                        //         None => mqtt.error_message(
                        //             "soil moisture sensor is not connected".to_string(),
                        //         ),
                        //     },
                        //     Err(e) => {
                        //         error!("Error with bme280 driver: {:?}", e);
                        //         mqtt.error_message("Bme280 sensor is not connected".to_string());
                        //     }
                        // }
                    }
                }
                Command::AllSemorData => {
                    todo!("implement all sensor data")
                }
            }
        });
        let _error_sub = event_loop.subscribe(move |err: &SimplCommandError| {
            if let Ok(mut mqtt) = mqtt_err.lock() {
                match err {
                    SimplCommandError::InvalidValue => {
                        mqtt.error_message("Missing or wrong value".to_string());
                    }
                    SimplCommandError::JsonParseError => {
                        mqtt.error_message("Invalid Json".to_string());
                    }
                    SimplCommandError::ParseError => {
                        mqtt.error_message("Invalid encoding ( utf8 parsing failed )".to_string());
                    }
                    SimplCommandError::WrongCommand => {
                        mqtt.error_message("Unknown command".to_string());
                    }
                };
            }
        });
        info!("Ready for action!");
        event_loop.spin(None)?;
        Ok(())
    }
}
