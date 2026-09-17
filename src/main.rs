mod lovense;
mod server;

use std::sync::Arc;
use buttplug_client::{
    connector::ButtplugRemoteClientConnector, serializer::ButtplugClientJSONSerializer,
    ButtplugClient, ButtplugClientError, ButtplugClientEvent,
};
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketClientTransport;
use futures::StreamExt;
use server::start_servers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("===========================================");
    println!("    Lovense Intiface Bridge");
    println!("===========================================\n");

    // Step 1: Create Buttplug client
    let client = Arc::new(ButtplugClient::new("Lovense Intiface Bridge"));

    // Step 2: Set up event handlers
    let mut events = client.event_stream();
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            match event {
                ButtplugClientEvent::DeviceAdded(device) => {
                    println!("[+] Device connected: {} (index: {})", device.name(), device.index());
                }
                ButtplugClientEvent::DeviceRemoved(info) => {
                    println!("[-] Device disconnected: {} (index: {})", info.name(), info.index());
                }
                ButtplugClientEvent::ServerDisconnect => {
                    println!("[!] Intiface server connection lost!");
                }
                ButtplugClientEvent::Error(err) => {
                    println!("[!] Error: {}", err);
                }
                _ => {}
            }
        }
    });

    // Step 3: Connect to the Intiface Central server
    let server_url = std::env::var("INTIFACE_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:12345".to_string());
    println!("Connecting to Intiface Central at {}...", server_url);
    let connector = ButtplugRemoteClientConnector::<
        ButtplugWebsocketClientTransport,
        ButtplugClientJSONSerializer,
    >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
        &server_url,
    ));

    match client.connect(connector).await {
        Ok(_) => {
            println!("Connected to Intiface Central!\n");
            // Start scanning for devices
            if let Err(e) = client.start_scanning().await {
                eprintln!("Warning: Failed to start device scanning: {}", e);
            } else {
                println!("Device scanning started.\n");
            }
        }
        Err(e) => {
            match e {
                ButtplugClientError::ButtplugConnectorError(error) => {
                    eprintln!("Warning: Could not connect to Intiface Central ({}).", server_url);
                    eprintln!("Make sure Intiface Central is running and started.");
                    eprintln!("Error: {}", error);
                    eprintln!("The HTTP/HTTPS server will continue running and serve requests.\n");
                }
                _ => eprintln!("Warning: Intiface connection error: {}", e),
            }
        }
    }

    // Step 4: Start HTTP and HTTPS servers
    let http_port = std::env::var("LOVENSE_HTTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(20010);
    let https_port = std::env::var("LOVENSE_HTTPS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(30010);

    println!("Starting Lovense HTTP/HTTPS servers...");
    println!("  HTTP:  http://0.0.0.0:{}/command", http_port);
    println!("  HTTPS: https://0.0.0.0:{}/command", https_port);

    start_servers(client, http_port, https_port).await?;

    Ok(())
}
