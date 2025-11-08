use crate::error::{MdnsError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub instance_name: String,
    pub port: u16,
    pub hostname: String,
    pub workgroup: String,
    pub description: String,
    pub shares: Vec<ShareConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareConfig {
    pub name: String,
    pub path: String,
    pub comment: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            service_name: "_smb._tcp.local.".to_string(),
            instance_name: "Windows-Share".to_string(),
            port: 445,
            hostname: "windows-pc.local".to_string(),
            workgroup: "WORKGROUP".to_string(),
            description: "Windows SMB Share via mDNS".to_string(),
            shares: vec![ShareConfig {
                name: "Public".to_string(),
                path: "C:\\Users\\Public\\Documents".to_string(),
                comment: "Public shared folder".to_string(),
            }],
            bind_address: None,
        }
    }
}

impl ServiceConfig {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ServiceConfig = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        self.validate()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        if cfg!(target_os = "windows") {
            PathBuf::from("C:\\ProgramData\\MDNSResponder\\config.json")
        } else {
            PathBuf::from("/etc/mdns-responder/config.json")
        }
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Validate service name format (must end with .local.)
        if !self.service_name.ends_with(".local.") {
            return Err(MdnsError::ConfigValidation(
                "service_name must end with '.local.'".to_string(),
            ));
        }

        // Validate instance name is not empty
        if self.instance_name.is_empty() {
            return Err(MdnsError::ConfigValidation(
                "instance_name cannot be empty".to_string(),
            ));
        }

        // Validate instance name length (max 63 characters per DNS label)
        if self.instance_name.len() > 63 {
            return Err(MdnsError::ConfigValidation(
                "instance_name exceeds 63 characters".to_string(),
            ));
        }

        // Validate port is valid (allow any port for testing)
        if self.port == 0 {
            return Err(MdnsError::ConfigValidation("port cannot be 0".to_string()));
        }

        // Validate hostname is not empty
        if self.hostname.is_empty() {
            return Err(MdnsError::ConfigValidation(
                "hostname cannot be empty".to_string(),
            ));
        }

        // Validate hostname format (basic DNS compliance)
        if !self
            .hostname
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-')
            || self.hostname.starts_with('-')
            || self.hostname.ends_with('-')
        {
            return Err(MdnsError::ConfigValidation(
                "hostname must contain only alphanumeric characters and hyphens".to_string(),
            ));
        }

        // Validate at least one share is configured
        if self.shares.is_empty() {
            return Err(MdnsError::ConfigValidation(
                "at least one share must be configured".to_string(),
            ));
        }

        // Validate each share
        for (i, share) in self.shares.iter().enumerate() {
            if share.name.is_empty() {
                return Err(MdnsError::ConfigValidation(format!(
                    "share[{}]: name cannot be empty",
                    i
                )));
            }
            if share.path.is_empty() {
                return Err(MdnsError::ConfigValidation(format!(
                    "share[{}]: path cannot be empty",
                    i
                )));
            }
        }

        Ok(())
    }
}
