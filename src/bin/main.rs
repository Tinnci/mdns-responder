use mdns_responder::{mdns_service, windows_service};
#[cfg(debug_assertions)]
use mdns_responder::discovery;
use mdns_responder::Result;
use log::info;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        env_logger::builder().init();
        match args[1].as_str() {
            "install" => {
                info!("Installing Windows service...");
                windows_service::install()?;
            }
            "uninstall" => {
                info!("Uninstalling Windows service...");
                windows_service::uninstall()?;
            }
            "run" => {
                info!("Running mDNS responder service in foreground...");
                mdns_service::run(None, None)?;
            }
            #[cfg(debug_assertions)]
            "discover" => {
                info!("Discovering mDNS services on network...");
                discovery::test_discovery()?;
            }
            _ => {
                #[cfg(debug_assertions)]
                let usage_msg = "Usage: {} [install|uninstall|run|discover]";
                #[cfg(not(debug_assertions))]
                let usage_msg = "Usage: {} [install|uninstall|run]";
                eprintln!("{}", usage_msg.replace("{}", &args[0]));
                std::process::exit(1);
            }
        }
    } else {
        windows_service::run_dispatcher()?;
    }

    Ok(())
}
