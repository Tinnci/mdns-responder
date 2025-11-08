use crate::error::Result;
use log::{error, info};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

use crate::config::ServiceConfig;
use crate::mdns_service;

const SERVICE_NAME: &str = "MDNSResponder";

define_windows_service!(ffi_service_main, service_main);

pub fn service_main(_args: Vec<OsString>) {
    if let Err(e) = run_service() {
        error!("Service error: {}", e);
    }
}

fn run_service() -> Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop => {
                info!("Received stop control request");
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: std::time::Duration::from_secs(5),
        process_id: None,
    })?;

    let service_thread =
        thread::spawn(move || -> Result<()> { mdns_service::run(Some(shutdown_rx), None) });

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    info!("Service started successfully");

    match service_thread.join() {
        Ok(Ok(_)) => info!("Service stopped gracefully."),
        Ok(Err(e)) => error!("Service thread failed: {}", e),
        Err(_) => error!("Service thread panicked."),
    }

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

pub fn install() -> Result<()> {
    info!("Installing Windows service: {}", SERVICE_NAME);

    let exe_path = std::env::current_exe()?;
    let bin_path = format!("\"{}\"", exe_path.display());

    let output = Command::new("sc")
        .args([
            "create",
            SERVICE_NAME,
            &format!("binPath= {}", bin_path),
            "start=",
            "auto",
            "type=",
            "own",
        ])
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            "Failed to create service. stdout: {}, stderr: {}",
            stdout, stderr
        );
        return Err(crate::error::MdnsError::Service(
            "Service creation failed".to_string(),
        ));
    }

    info!("Service installed successfully");

    let config_path = ServiceConfig::config_path();
    if let Some(config_dir) = config_path.parent() {
        if !config_dir.exists() {
            info!("Creating config directory at {:?}", config_dir);
            std::fs::create_dir_all(config_dir)?;
        }
    }

    if !config_path.exists() {
        info!("Writing default config to {:?}", config_path);
        let default_config = ServiceConfig::default();
        default_config.save_to_file(&config_path)?;
    }

    Command::new("sc")
        .args([
            "description",
            SERVICE_NAME,
            "mDNS Responder - Bonjour service for Windows SMB shares",
        ])
        .output()?;

    info!("Service description set");

    Ok(())
}

pub fn uninstall() -> Result<()> {
    info!("Uninstalling Windows service: {}", SERVICE_NAME);

    Command::new("sc").args(["stop", SERVICE_NAME]).output()?;

    thread::sleep(std::time::Duration::from_secs(2));

    let output = Command::new("sc").args(["delete", SERVICE_NAME]).output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("Failed to delete service: {}", error);
        return Err(crate::error::MdnsError::Service(
            "Service deletion failed".to_string(),
        ));
    }

    info!("Service uninstalled successfully");

    Ok(())
}

pub fn service_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from("C:\\ProgramData\\MDNSResponder")
    } else {
        PathBuf::from("/opt/mdns-responder")
    }
}

pub fn run_dispatcher() -> Result<()> {
    info!("Starting service dispatcher");
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}
