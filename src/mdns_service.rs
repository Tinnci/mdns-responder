use crate::config::ServiceConfig;
use crate::error::Result;
use log::{info, warn};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::net::{IpAddr, UdpSocket};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

fn get_local_ip() -> Result<String> {
    use ipconfig::get_adapters;

    // Strategy 1: Find the first valid physical/Ethernet adapter with IPv4
    let adapters = get_adapters()?;

    for adapter in adapters {
        let adapter_desc = adapter.description().to_string();

        // Skip virtual/VPN interfaces
        if adapter_desc.contains("Virtual")
            || adapter_desc.contains("VPN")
            || adapter_desc.contains("Hyper-V")
            || adapter_desc.contains("Bluetooth")
        {
            continue;
        }

        // Check if adapter is up and has IP addresses
        if adapter.ip_addresses().is_empty() {
            continue;
        }

        // Find first valid private IPv4 address
        for ip_addr in adapter.ip_addresses() {
            if let IpAddr::V4(ipv4) = ip_addr {
                let octets = ipv4.octets();
                // Check if it's a private address (10.x.x.x, 172.16-31.x.x, 192.168.x.x)
                let is_private = match octets[0] {
                    10 => true,
                    172 if octets[1] >= 16 && octets[1] <= 31 => true,
                    192 if octets[1] == 168 => true,
                    _ => false,
                };

                if is_private {
                    info!("Selected IP from adapter '{}': {}", adapter_desc, ipv4);
                    return Ok(ipv4.to_string());
                }
            }
        }
    }

    // Fallback: UDP socket method (more reliable than before)
    warn!("No physical adapter found, falling back to UDP socket detection");
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // Use public DNS as fallback target (more reliable than mDNS)
    socket.connect("8.8.8.8:80")?;
    let local_addr = socket.local_addr()?;
    Ok(local_addr.ip().to_string())
}

pub fn run(
    shutdown_rx: Option<Receiver<()>>,
    config_override: Option<ServiceConfig>,
) -> Result<()> {
    info!("Initializing mDNS Responder Service...");

    let config = if let Some(config) = config_override {
        config
    } else {
        let config_path = ServiceConfig::config_path();
        ServiceConfig::from_file(&config_path).or_else(|e| {
            warn!(
                "Failed to load config from {:?}: {}, using defaults",
                config_path, e
            );
            Ok::<_, crate::error::MdnsError>(ServiceConfig::default())
        })?
    };
    info!("Using configuration: {:?}", config);

    // Get actual local IP address
    let ip_addr = if let Some(bind_addr) = &config.bind_address {
        info!("Using manually configured bind address: {}", bind_addr);
        bind_addr.clone()
    } else {
        let detected_ip = get_local_ip()?;
        info!("Auto-detected local IP address: {}", detected_ip);
        detected_ip
    };

    let daemon = Arc::new(
        ServiceDaemon::new().map_err(|e| crate::error::MdnsError::Service(e.to_string()))?,
    );

    let mut txt_records = HashMap::new();

    // Standard SMB/CIFS TXT records (RFC 6763 compatible)
    txt_records.insert("vers".to_string(), "3.0".to_string());
    txt_records.insert("nt".to_string(), "hardware".to_string());
    txt_records.insert("flags".to_string(), "1".to_string());

    // Custom properties
    txt_records.insert("workgroup".to_string(), config.workgroup.clone());
    txt_records.insert("description".to_string(), config.description.clone());
    let share_paths: Vec<String> = config
        .shares
        .iter()
        .map(|s| s.path.replace('\\', "/"))
        .collect();
    txt_records.insert("path".to_string(), share_paths.join(","));

    // Ensure hostname ends with .local. for proper mDNS resolution
    let hostname_fqdn = if config.hostname.ends_with(".local.") {
        config.hostname.clone()
    } else if config.hostname.ends_with(".local") {
        format!("{}.", config.hostname)
    } else {
        format!("{}.local.", config.hostname)
    };
    info!("Using hostname: {}", hostname_fqdn);

    let service_info = ServiceInfo::new(
        &config.service_name,
        &config.instance_name,
        &hostname_fqdn,
        &ip_addr,
        config.port,
        Some(txt_records),
    )
    .map_err(|e| crate::error::MdnsError::Service(e.to_string()))?;

    daemon
        .register(service_info)
        .map_err(|e| crate::error::MdnsError::Service(e.to_string()))?;
    info!(
        "Successfully registered {} on port {} with IP {}",
        config.instance_name, config.port, ip_addr
    );

    // Wait for shutdown signal
    if let Some(shutdown_rx) = shutdown_rx {
        shutdown_rx.recv().ok();
        info!("Received shutdown signal from service control handler.");
    } else {
        let (tx, rx) = std::sync::mpsc::channel();
        ctrlc::set_handler(move || tx.send(()).unwrap())
            .map_err(|e| crate::error::MdnsError::Thread(e.to_string()))?;
        info!("Waiting for Ctrl-C...");
        rx.recv().ok();
        info!("Received Ctrl-C signal.");
    }

    graceful_shutdown(daemon)
}

fn graceful_shutdown(daemon: Arc<ServiceDaemon>) -> Result<()> {
    info!("Initiating graceful shutdown of mDNS daemon...");

    let shutdown_result = Arc::new(Mutex::new(None));
    let shutdown_result_clone = Arc::clone(&shutdown_result);

    let shutdown_thread = thread::spawn(move || {
        let result = daemon.shutdown();
        let mut locked = shutdown_result_clone.lock().unwrap();
        *locked = Some(result);
    });

    let timeout = Duration::from_secs(SHUTDOWN_TIMEOUT_SECS);
    let start = std::time::Instant::now();

    loop {
        if shutdown_thread.is_finished() {
            let result = shutdown_result.lock().unwrap();
            if let Some(Ok(_)) = *result {
                info!("Graceful shutdown completed successfully");
                return Ok(());
            } else if let Some(Err(e)) = &*result {
                warn!("Daemon shutdown reported error: {}", e);
                return Ok(());
            }
        }

        if start.elapsed() > timeout {
            warn!(
                "Daemon shutdown exceeded {}s timeout, but will continue gracefully",
                SHUTDOWN_TIMEOUT_SECS
            );
            return Ok(());
        }

        thread::sleep(Duration::from_millis(100));
    }
}
