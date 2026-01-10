use crate::grove_old::{self as grove, GroveError};
use crate::registry::types::{GroveRegistryConfig, RegistryConfig, RegistryRuntimeConfig};
use snafu::{ResultExt, Snafu};
use std::fs;

/// Load registry configuration from environment variables
///
/// Reads configuration from environment variables prefixed with `FORESTER_REGISTRY_`.
/// Falls back to defaults if variables are not set.
pub fn load_config() -> Result<RegistryRuntimeConfig, ConfigError> {
    use config_error::*;

    dotenvy::dotenv().ok();

    let config: RegistryConfig = envy::prefixed("FORESTER_REGISTRY_")
        .from_env()
        .context(LoadFailedSnafu)?;

    Ok(config.into_runtime())
}

/// Load grove-aware registry configuration
///
/// Uses grove name to derive container name, volume name, and port.
/// Port can be overridden via .grove/registry-port file.
pub fn load_grove_config() -> Result<RegistryRuntimeConfig, ConfigError> {
    use config_error::*;

    let (grove_root, grove_name) = grove::find_grove_root().context(GroveSnafu)?;

    let port_file = grove_root.join(".grove/registry-port");
    let config = if port_file.exists() {
        let port_str = fs::read_to_string(&port_file).context(PortFileReadSnafu {
            path: port_file.clone(),
        })?;
        let port: u16 = port_str
            .trim()
            .parse()
            .context(PortFileParseSnafu { path: port_file })?;
        GroveRegistryConfig::with_port(grove_name, port).context(InvalidPortSnafu)?
    } else {
        GroveRegistryConfig::new(grove_name)
    };

    Ok(config.into_runtime())
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum ConfigError {
    #[snafu(display("Failed to load configuration from environment"))]
    LoadFailed { source: envy::Error },

    #[snafu(display("Not in a grove"))]
    Grove { source: GroveError },

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
    InvalidPort {
        source: crate::registry::types::RegistryPortError,
    },
}
