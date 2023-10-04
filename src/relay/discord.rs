use dotenvy_macro::dotenv;
use embedded_svc::http::client::asynch::Client;
use embedded_svc::http::client::asynch::*;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::{error, info};

use embedded_svc::io::asynch::{Read, Write};

const DISCORD_WEBHOOK: &str = dotenv!("DISCORD");

pub async fn discord_webhook(content: String) -> anyhow::Result<()> {
    let connection = EspHttpConnection::new(&Configuration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let unblocking_connection = TrivialUnblockingConnection::new(connection);
    let mut client = Client::wrap(unblocking_connection);

    // Prepare payload
    let body = format!("{{\"content\": \"{}\"}}", content);
    post_request(&mut client, DISCORD_WEBHOOK, body.as_bytes()).await?;
    Ok(())
}

async fn post_request<C: embedded_svc::http::client::Connection>(
    client: &mut Client<TrivialUnblockingConnection<C>>,
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
