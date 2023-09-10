use anyhow::Result;
use core::str;
use esp_idf_svc::eventloop::{
    EspEventFetchData, EspEventPostData, EspTypedEventDeserializer, EspTypedEventSerializer,
    EspTypedEventSource,
};
use esp_idf_svc::tls::X509;
use log::{error, info};
use serde_json::Value;

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
    fn safe_message(&mut self, msg: String);
}

impl SimpleMqttClient for EspMqttClient {
    fn message(&mut self, msg: String) -> Result<()> {
        self.publish("feeds/message", QoS::AtMostOnce, false, msg.as_bytes())?;
        Ok(())
    }
    fn safe_message(&mut self, msg: String) {
        self.message(msg).unwrap_or_else(|err| {
            error!("Error sending message: {:?}", err);
        });
    }
}

// ==============
// MQTT Commands
// ==============

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MqttCommand {
    Water(bool),
    Lamp(u8),
    ReadBarometer,
    ReadSoilMoisture,
}

impl EspTypedEventSource for MqttCommand {
    fn source() -> *const core::ffi::c_char {
        b"MQTT-COMMAND\0".as_ptr() as *const _
    }
}

impl EspTypedEventDeserializer<MqttCommand> for MqttCommand {
    fn deserialize<R>(
        data: &EspEventFetchData,
        f: &mut impl for<'a> FnMut(&'a MqttCommand) -> R,
    ) -> R {
        f(unsafe { data.as_payload() })
    }
}

impl EspTypedEventSerializer<MqttCommand> for MqttCommand {
    fn serialize<R>(payload: &MqttCommand, f: impl for<'a> FnOnce(&'a EspEventPostData) -> R) -> R {
        f(&unsafe { EspEventPostData::new(Self::source(), Self::event_id(), payload) })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command is not recognized")]
    WrongCommand(CommandJson),
    #[error("Command Value is invalid: {0}")]
    InvalidValue(Value),
    #[error("Command is not valid JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandJson {
    name: String,
    value: Option<serde_json::Value>,
}

//
impl FromStr for MqttCommand {
    fn from_str(input: &str) -> Result<MqttCommand, CommandError> {
        let parsed_command = serde_json::from_str::<CommandJson>(input);
        info!("Got command: {:?}", parsed_command);
        match parsed_command {
            Ok(command) => {
                let error_cmd = command.clone();
                match command.name.as_str() {
                    "water" => {
                        let value = command.value.ok_or(CommandError::WrongCommand(error_cmd))?;
                        let value = value.as_bool().ok_or(CommandError::InvalidValue(value))?;
                        Ok(MqttCommand::Water(value))
                    }
                    "lamp" => {
                        let value = command.value.ok_or(CommandError::WrongCommand(error_cmd))?;
                        let value = value.as_u64().ok_or(CommandError::InvalidValue(value))?;
                        Ok(MqttCommand::Lamp(value as u8))
                    }
                    "read_barometer" => Ok(MqttCommand::ReadBarometer),
                    "read_soil_moisture" => Ok(MqttCommand::ReadSoilMoisture),
                    _ => Err(CommandError::WrongCommand(error_cmd)),
                }
            }
            Err(err) => Err(CommandError::JsonParseError(err)),
        }
    }

    type Err = CommandError;
}
