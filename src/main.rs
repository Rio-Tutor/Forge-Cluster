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
async fn main() -> Result<(), Box<dyn Error>> {
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
            let worker_address = arguments
                .get(2)
                .ok_or("Usage: forge master <worker-ip:port>")?;

            run_master(worker_address).await?;
        }
        _ => {
            println!("FORGE v0.1 - distributed compute prototype");
            println!("Usage:");
            println!("  forge worker [bind-ip:port]");
            println!("  forge master <worker-ip:port>");
        }
    }

    Ok(())
}

async fn run_master(worker_address: &str) -> Result<(), Box<dyn Error>> {
    let mut registry = DeviceRegistry::new();

    registry.register(Device {
        id: 1,
        name: "CachyOS PC".into(),
        processor: "AMD Ryzen 3 3250U with Vega 3".into(),
        ram_gb: 8,
        role: DeviceRole::Master,
        status: ConnectionStatus::Connected,
    })?;

    registry.register(Device {
        id: 2,
        name: "Android worker".into(),
        processor: "OnePlus Nord CE 4 Lite / Snapdragon 695".into(),
        ram_gb: 8,
        role: DeviceRole::Worker,
        status: ConnectionStatus::Unknown,
    })?;

    println!("FORGE MASTER");
    println!("Registered devices: {}", registry.count());
    println!("Connecting to worker at {worker_address}");

    scheduler::run_demo_tasks(worker_address).await?;

    Ok(())
}
