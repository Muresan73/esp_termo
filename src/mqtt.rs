use anyhow::Result;
use core::str;

use std::time::Duration;

use dotenvy_macro::dotenv;

use esp_idf_svc::mqtt::client::LwtConfiguration;
use esp_idf_svc::mqtt::client::MqttProtocolVersion;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};

const USERNAME: &str = dotenv!("USERNAME");
const KEY: &str = dotenv!("KEY");
const MQTT_SERVER: &str = dotenv!("MQTT_SERVER");

pub fn new_mqqt_client() -> Result<EspMqttClient> {
    let error_topic = format!("error/{}", USERNAME);
    let conf = MqttClientConfiguration {
        client_id: Some("esp32-sensore"),
        protocol_version: Some(MqttProtocolVersion::V3_1_1),

        keep_alive_interval: Some(Duration::from_secs(120)),
        lwt: Some(LwtConfiguration {
            topic: error_topic.as_str(),
            qos: QoS::AtMostOnce,
            payload: "connection lost".as_bytes(),
            retain: false,
        }),
        ..Default::default()
    };

    println!("MQTT Conecting ...");
    let mut client = EspMqttClient::new(
        format!("mqtt://{MQTT_SERVER}"),
        &conf,
        move |msg| match msg {
            Ok(msg) => println!("MQTT Message: {:?}", msg),
            Err(e) => println!("MQTT Message ERROR: {}", e),
        },
    )?;
    println!("MQTT Listening for messages");

    client.publish(
        "status/sensor",
        QoS::AtMostOnce,
        false,
        "connected".to_string().as_bytes(),
    )?;
    // loop {
    //     println!("Before publish");

    //     // TODO get values
    //     // let temperature = bmp180.get_temperature();

    //     client.publish(
    //         format!("{}/feeds/temperature", USERNAME).as_str(),
    //         QoS::AtMostOnce,
    //         false,
    //         format!("{}", 11).as_bytes(),
    //     )?;
    //     println!("Published message");

    //     sleep(Duration::from_millis(60_000));
    // }
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
