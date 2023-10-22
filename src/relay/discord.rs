use dotenvy_macro::dotenv;
use embedded_svc::http::client::Client;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::{error, info};

const DISCORD_WEBHOOK: &str = dotenv!("DISCORD");

pub async fn discord_webhook(content: String) -> anyhow::Result<()> {
    let connection = EspHttpConnection::new(&Configuration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(connection);

    // Prepare payload
    let body = format!("{{\"content\": \"{}\"}}", content);

    // Use of blocking version until async Connection api is stabilized
    post_request(&mut client, DISCORD_WEBHOOK, body.as_bytes()).ok();
    Ok(())
}

use embedded_svc::http::client::asynch::Client as AsyncClient;
use embedded_svc::http::client::asynch::Connection;
use embedded_svc::io::asynch::{Read, Write};
async fn post_request_async<C: Connection>(
    client: &mut AsyncClient<C>,
    url: &str,
    payload: &[u8],
) -> Result<(), C::Error> {
    // Prepare headers and URL
    let content_length_header = format!("{}", payload.len());
    let headers = [
        ("accept", "text/plain"),
        ("content-type", "application/json"),
        ("content-length", &*content_length_header),
    ];

    let mut request = client.post(url, &headers).await?;
    request.write_all(payload).await?;
    request.flush().await?;
    info!("-> POST {}", url);
    let mut response = request.submit().await?;

    info!("Process response");
    let status = response.status();
    info!("<- {}", status);
    let (_headers, body) = response.split();
    let mut buf = [0u8; 1024];
    let bytes_read = body.read(&mut buf).await?;

    info!("Read {} bytes", bytes_read);
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => info!(
            "Response body (truncated to {} bytes): {:?}",
            buf.len(),
            body_string
        ),
        Err(e) => error!("Error decoding response body: {}", e),
    };

    Ok(())
}

use embedded_svc::http::client::Client as HttpClient;
fn post_request(
    client: &mut HttpClient<EspHttpConnection>,
    url: &str,
    payload: &[u8],
) -> anyhow::Result<()> {
    use embedded_svc::{io::Write, utils::io};

    // Prepare headers and URL
    let content_length_header = format!("{}", payload.len());
    let headers = [
        ("accept", "text/plain"),
        ("content-type", "application/json"),
        ("content-length", &*content_length_header),
    ];

    // Send request
    let mut request = client.post(url, &headers)?;
    request.write_all(payload)?;
    request.flush()?;
    info!("-> POST {}", url);
    let mut response = request.submit()?;

    // Process response
    let status = response.status();
    info!("<- {}", status);
    let (_headers, mut body) = response.split();
    let mut buf = [0u8; 1024];
    let bytes_read = io::try_read_full(&mut body, &mut buf).map_err(|e| e.0)?;
    info!("Read {} bytes", bytes_read);
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => info!(
            "Response body (truncated to {} bytes): {:?}",
            buf.len(),
            body_string
        ),
        Err(e) => error!("Error decoding response body: {}", e),
    };

    // Drain the remaining response bytes
    while body.read(&mut buf)? > 0 {}

    Ok(())
}
