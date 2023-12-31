use core::str;
use esp_idf_svc::eventloop::{
    EspEventFetchData, EspEventPostData, EspTypedEventDeserializer, EspTypedEventSerializer,
    EspTypedEventSource,
};
use esp_idf_svc::tls::X509;
use log::{error, info};
use macro_lib::EspEvent;
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
use esp_idf_sys::{self as _, EspError};

const USERNAME: &str = dotenv!("USERNAME");
const KEY: &str = dotenv!("KEY");
const MQTT_SERVER: &str = dotenv!("MQTT_SERVER");
const CERT: &[u8] = include_bytes!("../../certs/cert.pem");

pub fn new_mqqt_client<'a>(
    process_message: impl Fn(Result<Command, CommandError>) + Send + 'static,
) -> Result<EspMqttClient<'a>, EspError> {
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
        format!("mqtts://{MQTT_SERVER}").as_str(),
        &conf,
        move |message_event| {
            match message_event {
                Ok(Received(msg)) => {
                    let result: Result<(), CommandError> = match msg.details() {
                        Complete => from_utf8(msg.data())
                            .map_err(CommandError::ParseError)
                            .and_then(|text| text.parse())
                            .map(|cmd| {
                                process_message(Ok(cmd));
                            }),
                        _ => {
                            info!("Received partial message: {:?}", message_event);
                            Ok(())
                        }
                    };
                    if let Err(error) = result {
                        error!("Error processing message: {:?}", error);
                        process_message(Err(error));
                    }
                }

                Ok(_) => {
                    info!("Received from MQTT: {:?}", message_event)
                }
                Err(e) => {
                    error!("Error from MQTT: {:?}", e)
                }
            };
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
    fn message(&mut self, msg: String) -> Result<(), EspError>;
    fn safe_message(&mut self, msg: String);
    fn error_message(&mut self, msg: String);
}

impl SimpleMqttClient for EspMqttClient<'_> {
    fn message(&mut self, msg: String) -> Result<(), EspError> {
        self.publish("feeds/message", QoS::AtMostOnce, false, msg.as_bytes())?;
        Ok(())
    }
    fn safe_message(&mut self, msg: String) {
        self.message(msg).unwrap_or_else(|err| {
            error!("Error sending message: {:?}", err);
        });
    }
    fn error_message(&mut self, msg: String) {
        let _ = self
            .publish("error/message", QoS::AtMostOnce, false, msg.as_bytes())
            .map_err(|err| {
                error!("Error sending message: {:?}", err);
            });
    }
}

#[derive(Debug, PartialEq, Clone, Copy, EspEvent)]
pub enum Command {
    Water(bool),
    Lamp(u8),
    ReadBarometer,
    ReadSoilMoisture,
    AllSemorData,
}

#[derive(Debug, Clone, Copy, EspEvent)]
pub enum SimplCommandError {
    WrongCommand,
    InvalidValue,
    JsonParseError,
    ParseError,
}
impl From<CommandError> for SimplCommandError {
    fn from(error: CommandError) -> Self {
        match error {
            CommandError::WrongCommand(_) => SimplCommandError::WrongCommand,
            CommandError::InvalidValue(_) => SimplCommandError::InvalidValue,
            CommandError::JsonParseError(_) => SimplCommandError::JsonParseError,
            CommandError::ParseError(_) => SimplCommandError::ParseError,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command is not recognized")]
    WrongCommand(CommandJson),
    #[error("Command Value is invalid: {0}")]
    InvalidValue(Value),
    #[error("Command is not valid JSON")]
    JsonParseError(serde_json::error::Category),
    #[error("Message is not valid UTF8")]
    ParseError(#[from] std::str::Utf8Error),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandJson {
    name: String,
    value: Option<serde_json::Value>,
}

//
impl FromStr for Command {
    fn from_str(input: &str) -> Result<Command, CommandError> {
        let parsed_command = serde_json::from_str::<CommandJson>(input);
        info!("Got command: {:?}", parsed_command);
        match parsed_command {
            Ok(command) => {
                let error_cmd = command.clone();
                match command.name.as_str() {
                    "water" => {
                        let value = command.value.ok_or(CommandError::WrongCommand(error_cmd))?;
                        let value = value.as_bool().ok_or(CommandError::InvalidValue(value))?;
                        Ok(Command::Water(value))
                    }
                    "lamp" => {
                        let value = command.value.ok_or(CommandError::WrongCommand(error_cmd))?;
                        let value = value.as_u64().ok_or(CommandError::InvalidValue(value))?;
                        Ok(Command::Lamp(value as u8))
                    }
                    "read_barometer" => Ok(Command::ReadBarometer),
                    "read_soil_moisture" => Ok(Command::ReadSoilMoisture),
                    "all" => Ok(Command::AllSemorData),
                    _ => Err(CommandError::WrongCommand(error_cmd)),
                }
            }
            Err(err) => Err(CommandError::JsonParseError(err.classify())),
        }
    }

    type Err = CommandError;
}
