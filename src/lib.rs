pub mod config;
pub mod discovery;
pub mod error;
pub mod mdns_service;
pub mod windows_service;

pub use error::{MdnsError, Result};

#[cfg(test)]
mod tests {
    use crate::mdns_service::run;
    use mdns_sd::{ServiceDaemon, ServiceEvent};
    use std::time::Duration;

    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_mdns_service() {
        // Generate unique name based on timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique_instance = format!("Test-Instance-{}", timestamp & 0xFFFF); // Short suffix
        
        let mut test_config = crate::config::ServiceConfig::default();
        test_config.instance_name = unique_instance.clone();
        let service_name = "_test._tcp.local.".to_string();
        test_config.service_name = service_name.clone();
        
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
        
        let service_thread = std::thread::spawn(move || {
            run(Some(shutdown_rx), Some(test_config)).unwrap();
        });

        std::thread::sleep(Duration::from_secs(3)); // Give more time

        let mdns = ServiceDaemon::new().unwrap();
        let receiver = mdns.browse("_test._tcp.local.").unwrap();

        let expected_fullname = format!("{}.{}", unique_instance, service_name);
        let mut service_found = false;
        
        // Try for 5 seconds
        for _ in 0..50 {
            if let Ok(ServiceEvent::ServiceResolved(info)) = receiver.recv_timeout(Duration::from_millis(100)) {
                if info.get_fullname() == expected_fullname {
                    service_found = true;
                    break;
                }
            }
        }

        shutdown_tx.send(()).unwrap();
        service_thread.join().unwrap();
        
        // CRITICAL: Clean up daemon
        mdns.shutdown().ok(); // Ignore errors on cleanup
        
        assert!(service_found, "mDNS service '{} ({}) ' was not found", unique_instance, expected_fullname);
    }
}