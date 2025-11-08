#![allow(unused_imports)]

use crate::error::Result;
use log::info;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::time::Duration;

/// Simple test to verify service discovery works (debug-only)
#[cfg(debug_assertions)]
pub fn test_discovery() -> Result<()> {
    info!("Starting service discovery test...");
    
    let daemon = ServiceDaemon::new()
        .map_err(|e| crate::error::MdnsError::Service(format!("Failed to create daemon: {}", e)))?;
    let receiver = daemon.browse("_smb._tcp.local.")
        .map_err(|e| crate::error::MdnsError::Service(format!("Failed to browse: {}", e)))?;
    
    info!("Browsing for SMB services for 10 seconds...");
    let mut found_count = 0;
    
    for _ in 0..10 {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                found_count += 1;
                let hostname = info.get_hostname();
                info!("Found: {} at {}", info.get_fullname(), hostname);
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
    
    info!("Discovery test complete. Found {} services.", found_count);
    Ok(())
}
