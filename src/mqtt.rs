use anyhow::Result;
use core::str;
use log::{error, info};

use std::str::from_utf8;
use std::time::Duration;

use dotenvy_macro::dotenv;

use esp_idf_svc::mqtt::client::LwtConfiguration;
use esp_idf_svc::mqtt::client::MqttProtocolVersion;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use embedded_svc::mqtt::client::Details::Complete;
use embedded_svc::mqtt::client::{Event::Received, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use esp_idf_sys as _;
const USERNAME: &str = dotenv!("USERNAME");
const KEY: &str = dotenv!("KEY");
const MQTT_SERVER: &str = dotenv!("MQTT_SERVER");

pub fn new_mqqt_client(process_message: impl Fn(String) + Send + 'static) -> Result<EspMqttClient> {
    let conf = MqttClientConfiguration {
        client_id: Some("esp32-sensore"),
        protocol_version: Some(MqttProtocolVersion::V3_1_1),
        client_certificate: None,
        private_key_password: Some(KEY),
        username: Some(USERNAME),
        keep_alive_interval: Some(Duration::from_secs(120)),
        lwt: Some(LwtConfiguration {
            topic: "status/sensor",
            qos: QoS::ExactlyOnce,
            payload: "connection lost".as_bytes(),
            retain: true,
        }),
        ..Default::default()
    };
    info!("MQTT Conecting ...");
    let mut client = EspMqttClient::new(
        format!("mqtt://{MQTT_SERVER}"),
        &conf,
        move |message_event| match message_event {
            Ok(Received(msg)) => match msg.details() {
                Complete => match from_utf8(msg.data()) {
                    Ok(text) => process_message(text.to_string()),
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
