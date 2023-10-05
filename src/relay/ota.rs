use dotenvy_macro::dotenv;
use embedded_svc::http::client::*;
use embedded_svc::utils::io;
use esp_idf_svc::errors::EspIOError;
use esp_idf_svc::http::client::*;
use log::info;

const OTA_ENDPOINT: &str = dotenv!("OTA_ENDPOINT");

#[derive(Debug, thiserror::Error)]
pub enum OtaError {
    #[error("EspIO internal error")]
    IO(#[from] EspIOError),
    #[error("OTA internal error")]
    Ota(#[from] esp_ota::Error),
    #[error("Esp internal error")]
    Esp(#[from] esp_idf_sys::EspError),
    #[error("Bad response")]
    BadResponse(u16),
}

pub fn fetch_ota_update() -> Result<(), OtaError> {
    info!("About to fetch content from ota");

    let connection = EspHttpConnection::new(&Configuration::default())?;
    let mut client = Client::wrap(connection);
    let mut response = client.get(OTA_ENDPOINT)?.submit()?;

    if response.status() / 100 != 2 {
        return Err(OtaError::BadResponse(response.status()));
    }

    info!("start update");
    let mut ota = esp_ota::OtaUpdate::begin()?;

    let mut body = [0_u8; 4096];
    while io::try_read_full(&mut response, &mut body).map_err(|err| err.0)? > 0 {
        ota.write(&body)?;
    }
    log::warn!("#L {:#x} {:#x}", &body[0], &body[1]);

    info!("OTA update fetched");
    // Performs validation of the newly written app image and completes the OTA update.
    let mut completed_ota = ota.finalize()?;
    info!("OTA update finalized");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Sets the newly written to partition as the next partition to boot from.
    completed_ota.set_as_boot_partition()?;
    info!("All good, ready to restart");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Restarts the CPU, booting into the newly written app.
    completed_ota.restart();
}
