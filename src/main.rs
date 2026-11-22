
mod compute;
mod device;
mod network;
mod protocol;
mod registry;
mod scheduler;
mod worker;
mod hardware;

use std::env;
use std::error::Error;

use device::{ConnectionStatus, Device, DeviceRole};
use registry::DeviceRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let arguments: Vec<String> = env::args().collect();

    match arguments.get(1).map(String::as_str) {
        Some("worker") => {
            let bind_address = arguments
                .get(2)
                .map(String::as_str)
                .unwrap_or("0.0.0.0:7878");

            worker::run_worker(bind_address).await?;
        }

        Some("master") => {
            let worker_addresses: Vec<String> =
                arguments.iter().skip(2).cloned().collect();

            if worker_addresses.is_empty() {
                return Err(
                    "Usage: forge master <worker1-ip:port> [worker2-ip:port ...]"
                        .into()
                );
            }

            run_master(&worker_addresses).await?;
        }

        _ => {
            println!("FORGE v0.1 - distributed compute prototype");
            println!();
            println!("Usage:");
            println!("  forge worker [bind-ip:port]");
            println!("  forge master <worker1-ip:port> [worker2-ip:port ...]");
        }
    }

    Ok(())
}

async fn run_master(
    worker_addresses: &[String],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut registry = DeviceRegistry::new();

    registry.register(Device {
        id: 1,
        name: "CachyOS PC".into(),
        processor: "AMD Ryzen 3 3250U with Vega 3".into(),
        ram_gb: 8,
        role: DeviceRole::Master,
        status: ConnectionStatus::Connected,
    }).map_err(std::io::Error::other)?;

    for (index, address) in worker_addresses.iter().enumerate() {
        registry.register(Device {
            id: (index + 2) as u32,
            name: format!("Worker {}", index + 1),
            processor: format!("Remote worker at {address}"),
            ram_gb: 0,
            role: DeviceRole::Worker,
            status: ConnectionStatus::Unknown,
        }).map_err(std::io::Error::other)?;
    }

    println!("FORGE MASTER");
    println!("Registered devices: {}", registry.count());
    println!("Workers: {}", worker_addresses.len());

    for address in worker_addresses {
        println!("Worker: {address}");
    }

    scheduler::run_sum_squares(
        worker_addresses.to_vec(),
        1_000_000,
        10_000,
    ).await?;

    Ok(())
}

