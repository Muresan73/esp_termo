use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use anyhow::bail;
use dotenvy_macro::dotenv;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::wifi::ClientConfiguration;

use embedded_svc::wifi::Configuration;
use embedded_svc::wifi::Wifi;
use esp_idf_hal::i2c;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::mqtt::client::LwtConfiguration;
use esp_idf_svc::wifi;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

use anyhow::Result;
use embedded_svc::mqtt::client::{Connection, Publish, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::*;
use esp_idf_svc::wifi::*;

const SSID: &str = dotenv!("SSID");
const PASSWORD: &str = dotenv!("PASSWORD");
const USERNAME: &str = dotenv!("USERNAME");
const KEY: &str = dotenv!("KEY");

pub fn connect_2_wifi() -> Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.into(),
        channel: None,
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    println!("Wifi started");

    wifi.connect()?;
    println!("Wifi connected");

    wifi.wait_netif_up()?;
    println!("Wifi netif up");

    Ok(())
}

pub fn new_mqqt_client() -> Result<EspMqttClient> {
    connect_2_wifi()?;

    let error_topic = format!("{}/error", USERNAME);
    let conf = MqttClientConfiguration {
        client_id: Some("esp32-sensore"),
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
    let client = EspMqttClient::new(
        format!("mqtt://{}:{}@test.mosquitto.org3", USERNAME, KEY),
        &conf,
        move |msg| match msg {
            Ok(msg) => println!("MQTT Message: {:?}", msg),
            Err(e) => println!("MQTT Message ERROR: {}", e),
        },
    )?;
    println!("MQTT Listening for messages");

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

// fn wifi(
//     netif_stack: Arc<EspNetifStack>,
//     sys_loop_stack: Arc<EspSysLoopStack>,
//     default_nvs: Arc<EspDefaultNvs>,
// ) -> Result<Box<EspWifi>> {
//     let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

//     wifi.set_configuration(&Configuration::Client(ClientConfiguration {
//         ssid: SSID.into(),
//         password: PASS.into(),
//         ..Default::default()
//     }))?;

//     println!("Wifi configuration set, about to get status");

//     wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
//         .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

//     let status = wifi.get_status();

//     if let Status(
//         ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(
//             _ip_settings,
//         ))),
//         _,
//     ) = status
//     {
//         println!("Wifi connected");
//     } else {
//         bail!("Unexpected Wifi status: {:?}", status);
//     }

//     Ok(wifi)
// }
