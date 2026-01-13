//! Grove-specific registry configuration.
//!
//! Derives container names, volume names, and ports from grove context,
//! with optional port override via `.grove/registry-port` file.

use bon::Builder;
use snafu::{ResultExt, Snafu};
use std::fs;

use crate::grove::GroveContext;
use crate::registry::port::{
    ContainerName, ImageRef, RegistryPort, RegistryPortError, RegistryRuntimeConfig, VolumeName,
};

/// Grove-specific registry configuration.
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct GroveRegistryConfig {
    grove_name: String,
    port: RegistryPort,
    image: ImageRef,
}

impl GroveRegistryConfig {
    /// Creates configuration from a grove context.
    ///
    /// Derives port from grove name hash, checking for `.grove/registry-port` override.
    pub fn for_grove(grove: &GroveContext) -> Result<Self, GroveRegistryConfigError> {
        use grove_registry_config_error::*;

        let grove_name = grove.name().to_string();
        let port_file = grove.root().join(".grove/registry-port");

        let port = if port_file.exists() {
            let port_str = fs::read_to_string(&port_file).context(PortFileReadSnafu {
                path: port_file.clone(),
            })?;
            let port: u16 = port_str
                .trim()
                .parse()
                .context(PortFileParseSnafu { path: port_file })?;
            RegistryPort::try_new(port).context(InvalidPortSnafu)?
        } else {
            derive_port(&grove_name)
        };

        Ok(Self {
            grove_name,
            port,
            image: ImageRef::default(),
        })
    }

    /// Returns the grove name.
    pub fn grove_name(&self) -> &str {
        &self.grove_name
    }

    /// Returns the port.
    pub fn port(&self) -> RegistryPort {
        self.port
    }

    /// Converts to runtime configuration with derived container and volume names.
    #[expect(clippy::unwrap_used, reason = "format strings are non-empty")]
    pub fn into_runtime(self) -> RegistryRuntimeConfig {
        let container_name =
            ContainerName::try_new(format!("brdev-registry-{}", self.grove_name)).unwrap();
        let volume_name =
            VolumeName::try_new(format!("brdev-registry-data-{}", self.grove_name)).unwrap();
        RegistryRuntimeConfig::new(self.port, self.image, container_name, volume_name)
    }
}

/// Derives port from grove name hash (range 5001-6999).
#[expect(clippy::unwrap_used, reason = "port range 5001-6999 always >= 1024")]
fn derive_port(grove_name: &str) -> RegistryPort {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    grove_name.hash(&mut hasher);
    let hash = hasher.finish();
    let port = 5001 + (hash % 1999) as u16;
    RegistryPort::try_new(port).unwrap()
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum GroveRegistryConfigError {
    #[snafu(display("Failed to read port file: {}", path.display()))]
    PortFileRead {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Failed to parse port file: {}", path.display()))]
    PortFileParse {
        path: std::path::PathBuf,
        source: std::num::ParseIntError,
    },

    #[snafu(display("Invalid port number"))]
    InvalidPort { source: RegistryPortError },
}
