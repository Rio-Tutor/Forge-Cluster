use std::{error::Error, net::{IpAddr, SocketAddr}};

use tokio::net::UdpSocket;

const DISCOVERY_PORT: u16 = 7877;
const BEACON_PREFIX: &str = "FORGE_DISCOVERY_V1";

/// Wait for a FORGE master beacon and return its TCP address plus this
/// device's LAN address as seen on the route back to that master.
pub async fn discover_master(
    worker_port: u16,
) -> Result<(SocketAddr, SocketAddr), Box<dyn Error + Send + Sync>> {
    let socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)).await?;
    println!("Searching for FORGE masters on UDP port {DISCOVERY_PORT}...");

    let mut buffer = [0_u8; 256];

    loop {
        let (length, source) = socket.recv_from(&mut buffer).await?;
        let message = std::str::from_utf8(&buffer[..length])?;
        let Some(port_text) = message.strip_prefix(&format!("{BEACON_PREFIX}|")) else {
            continue;
        };
        let master_port: u16 = match port_text.trim().parse() {
            Ok(port) => port,
            Err(_) => continue,
        };

        let master_address = SocketAddr::new(source.ip(), master_port);
        let route = UdpSocket::bind("0.0.0.0:0").await?;
        route.connect(master_address).await?;
        let local_ip = route.local_addr()?.ip();

        if local_ip.is_unspecified() || local_ip.is_loopback() {
            continue;
        }

        let worker_address = SocketAddr::new(local_ip, worker_port);
        println!("Master discovered at {master_address}");
        return Ok((master_address, worker_address));
    }
}

/// Broadcast the master's presence repeatedly so workers can find it
/// without being configured with its IP address.
pub async fn broadcast_master(
    master_port: u16,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;
    let destination = SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::BROADCAST), DISCOVERY_PORT);
    let beacon = format!("{BEACON_PREFIX}|{master_port}");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        interval.tick().await;
        if let Err(error) = socket.send_to(beacon.as_bytes(), destination).await {
            eprintln!("FORGE discovery beacon failed: {error}");
        }
    }
}


/// Determine the local IPv4 address used by the default network interface.
pub async fn local_ipv4_address() -> Result<IpAddr, Box<dyn Error + Send + Sync>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    // A UDP connect to a unicast documentation address selects the default
    // route without sending a packet or requiring broadcast-socket permission.
    socket.connect("192.0.2.1:9").await?;
    let address = socket.local_addr()?.ip();
    if address.is_unspecified() || address.is_loopback() {
        return Err("Could not determine a usable LAN IPv4 address".into());
    }
    Ok(address)
}
