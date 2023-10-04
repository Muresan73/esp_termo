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

async fn connect(
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

use async_watch::*;
pub struct WifiRelay {
    wifi: AsyncWifi<EspWifi<'static>>,
    tx: Sender<bool>,
    rx: Receiver<bool>,
}

impl WifiRelay {
    pub async fn new(modem: esp_idf_hal::modem::Modem) -> Result<Self, EspError> {
        let wifi = connect(modem).await?;
        let (tx, rx) = async_watch::channel(false);
        Ok(Self { wifi, tx, rx })
    }
    pub fn get_reciver(&mut self) -> Receiver<bool> {
        self.rx.clone()
    }

    pub async fn disconnect(&mut self) -> Result<(), EspError> {
        self.wifi.disconnect().await?;
        self.tx.send(false).ok();
        Ok(())
    }

    pub async fn reconnect(&mut self) -> Result<(), EspError> {
        self.wifi.connect().await?;
        self.wifi.wait_netif_up().await?;
        self.tx.send(true).ok();
        Ok(())
    }
}
