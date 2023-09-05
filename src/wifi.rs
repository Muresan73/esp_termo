use anyhow::Result;
use core::str;
use embedded_svc::wifi::Configuration;
use esp_idf_svc::eventloop::EspSystemEventLoop;

use dotenvy_macro::dotenv;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::wifi::ClientConfiguration;

use esp_idf_sys as _;

use esp_idf_svc::nvs::*;
use esp_idf_svc::wifi::*;

const SSID: &str = dotenv!("SSID");
const PASSWORD: &str = dotenv!("PASSWORD");

pub fn connect(modem: esp_idf_hal::modem::Modem) -> Result<Box<EspWifi<'static>>> {
    println!("Wifi getting resources");

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    println!("Wifi aquired resources");
    let mut esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop)?;

    println!("Wifi blocker");

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.into(),
        channel: None,
    });

    println!("Wifi configured");

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    println!("Wifi started");

    wifi.connect()?;
    println!("Wifi connected");

    wifi.wait_netif_up()?;
    println!("Wifi netif up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!("Wifi DHCP info: {:?}", ip_info);
    Ok(Box::new(esp_wifi))
}
