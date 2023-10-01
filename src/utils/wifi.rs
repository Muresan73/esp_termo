use core::str;
use embedded_svc::wifi::Configuration;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_sys::EspError;
use log::info;

use dotenvy_macro::dotenv;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::wifi::ClientConfiguration;

use esp_idf_sys as _;

use esp_idf_svc::nvs::*;
use esp_idf_svc::wifi::*;

const SSID: &str = dotenv!("SSID");
const PASSWORD: &str = dotenv!("PASSWORD");

pub async fn connect(
    modem: esp_idf_hal::modem::Modem,
) -> Result<AsyncWifi<EspWifi<'static>>, EspError> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;
    let timer_service = EspTaskTimerService::new()?;

    let mut wifi = AsyncWifi::wrap(esp_wifi, sys_loop, timer_service)?;

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.into(),
        channel: None,
    });

    info!("Wifi configured");

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start().await?;
    info!("Wifi started");

    wifi.connect().await?;
    info!("Wifi connected");

    wifi.wait_netif_up().await?;
    info!("Wifi netif up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(wifi)
}

pub async fn reconnect(wifi: &mut AsyncWifi<EspWifi<'static>>) -> Result<(), EspError> {
    wifi.connect().await?;
    wifi.wait_netif_up().await?;
    Ok(())
}
