use dotenvy_macro::dotenv;
use embedded_svc::http::client::Client;
use esp_idf_svc::http::client::*;
use log::{error, info};

use embedded_svc::{io::Write, utils::io};

const DISCORD_WEBHOOK: &str = dotenv!("DISCORD");

pub fn discord_webhook() -> anyhow::Result<()> {
    let mut client = Client::wrap(EspHttpConnection::new(&Configuration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?);

    let body = r#"{
            "content": "Hello, World!",
            "embeds": [
                {
                    "title": "Hello, Embed!",
                    "description": "This is an embedded §message."
                }
        ]
    }"#;

    post_request(&mut client, DISCORD_WEBHOOK, body.as_bytes())?;
    Ok(())
}

fn post_request(
    client: &mut Client<EspHttpConnection>,
    url: &str,
    payload: &[u8],
) -> anyhow::Result<()> {
    // Prepare payload

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
