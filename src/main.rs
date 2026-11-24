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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let arguments: Vec<String> = env::args().collect();

    match arguments.get(1).map(String::as_str) {
        Some("worker") => {
            let bind_address = arguments
                .get(2)
                .map(String::as_str)
                .unwrap_or("0.0.0.0:7878");
            let master_address = arguments.get(3).map(String::as_str);
            let advertised_address = arguments.get(4).map(String::as_str);

            if master_address.is_some() != advertised_address.is_some() {
                return Err(
                    "Worker registration requires both <master-ip:port> and <advertised-worker-ip:port>"
                        .into()
                );
            }

            worker::run_worker(
                bind_address,
                master_address,
                advertised_address,
            ).await?;
        }

        Some("master") => {
            let registration_bind = arguments
                .get(2)
                .map(String::as_str)
                .unwrap_or("0.0.0.0:7879");

            network::run_registration_server(registration_bind).await?;
        }

        _ => {
            println!("FORGE v0.2 - worker registration prototype");
            println!();
            println!("Usage:");
            println!("  forge master [registration-bind-ip:port]");
            println!("  forge worker [bind-ip:port] [master-ip:port advertised-worker-ip:port]");
            println!();
            println!("Example:");
            println!("  forge master 0.0.0.0:7879");
            println!("  forge worker 0.0.0.0:7878 192.168.1.10:7879 192.168.1.20:7878");
        }
    }

    Ok(())
}
