use crate::grove::{GroveContext, GroveContextError};
use crate::grove::registry::{GroveRegistryConfig, GroveRegistryConfigError};
use crate::registry::RegistryRuntimeConfig;
use snafu::{ResultExt, Snafu};

/// Load grove-aware registry configuration
///
/// Uses grove name to derive container name, volume name, and port.
/// Port can be overridden via .grove/registry-port file.
pub fn load_grove_config() -> Result<RegistryRuntimeConfig, ConfigError> {
    use config_error::*;

    let grove = GroveContext::detect().context(GroveSnafu)?;
    let config = GroveRegistryConfig::for_grove(&grove).context(RegistryConfigSnafu)?;
    Ok(config.into_runtime())
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum ConfigError {
    #[snafu(display("Not in a grove"))]
    Grove { source: GroveContextError },

    #[snafu(display("Failed to load registry configuration"))]
    RegistryConfig { source: GroveRegistryConfigError },
}
