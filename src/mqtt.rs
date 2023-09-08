use anyhow::Result;
use core::str;
use esp_idf_svc::tls::X509;
use log::{error, info};

use serde::{Deserialize, Serialize};
use std::str::{from_utf8, FromStr};
use std::time::Duration;

use dotenvy_macro::dotenv;

use embedded_svc::mqtt::client::Details::Complete;
use embedded_svc::mqtt::client::{Event::Received, QoS};
use esp_idf_svc::mqtt::client::LwtConfiguration;
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

const USERNAME: &str = dotenv!("USERNAME");
const KEY: &str = dotenv!("KEY");
const MQTT_SERVER: &str = dotenv!("MQTT_SERVER");
const CERT: &[u8] = include_bytes!("../certs/cert.pem");

pub fn new_mqqt_client(
    process_message: impl Fn(MqttCommand) + Send + 'static,
) -> Result<EspMqttClient> {
    let conf = MqttClientConfiguration {
        client_id: Some("esp32-sensore"),
        server_certificate: Some(X509::pem_until_nul(CERT)),
        password: Some(KEY),
        username: Some(USERNAME),
        keep_alive_interval: Some(Duration::from_secs(120)),
        lwt: Some(LwtConfiguration {
            topic: "status/sensor",
            qos: QoS::ExactlyOnce,
            payload: "connection lost".as_bytes(),
            retain: false,
        }),
        ..Default::default()
    };
    info!("MQTT Conecting ...");
    let mut client = EspMqttClient::new(
        format!("mqtts://{MQTT_SERVER}"),
        &conf,
        move |message_event| match message_event {
            Ok(Received(msg)) => match msg.details() {
                Complete => match from_utf8(msg.data()) {
                    Ok(text) => {
                        if let Ok(cmd) = text.parse() {
                            process_message(cmd)
                        }
                    }
                    Err(e) => error!("Error decoding message: {:?}", e),
                },
                _ => error!("Received partial message: {:?}", msg),
            },

            Ok(_) => info!("Received from MQTT: {:?}", message_event),
            Err(e) => error!("Error from MQTT: {:?}", e),
        },
    )?;

    client.subscribe("station/cmd", QoS::AtLeastOnce)?;

    info!("MQTT Listening for messages");

    client.publish(
        "status/sensor",
        QoS::AtMostOnce,
        false,
        "connected".to_string().as_bytes(),
    )?;

    Ok(client)
}

pub trait SimpleMqttClient {
    fn message(&mut self, msg: String) -> Result<()>;
}

impl SimpleMqttClient for EspMqttClient {
    fn message(&mut self, msg: String) -> Result<()> {
        self.publish("feeds/message", QoS::AtMostOnce, false, msg.as_bytes())?;
        Ok(())
    }
}

// ==============
// MQTT Commands
// ==============

#[derive(Debug, PartialEq)]
pub enum MqttCommand {
    Water(bool),
    Lamp(u8),
}

#[derive(Debug)]
pub struct WrongCommandError(String);

impl std::error::Error for WrongCommandError {}

impl std::fmt::Display for WrongCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Not recognized command: {}", self.0)
    }
}

#[derive(Serialize, Deserialize)]
struct CommandJson {
    name: String,
    value: serde_json::Value,
}

impl FromStr for MqttCommand {
    fn from_str(input: &str) -> Result<MqttCommand, WrongCommandError> {
        let parsed_command = serde_json::from_str::<CommandJson>(input);

        if let Ok(command) = parsed_command {
            match command.name.as_str() {
                "water" => {
                    let value = command.value.as_bool().ok_or_else(|| {
                        WrongCommandError(format!("Wrong value for water command: {}", input))
                    })?;
                    Ok(MqttCommand::Water(value))
                }
                "lamp" => {
                    let value = command.value.as_u64().ok_or_else(|| {
                        WrongCommandError(format!("Wrong value for lamp command: {}", input))
                    })?;
                    Ok(MqttCommand::Lamp(value as u8))
                }
                _ => Err(WrongCommandError(input.to_string())),
            }
        } else {
            Err(WrongCommandError(input.to_string()))
        }
    }

    type Err = WrongCommandError;
}
