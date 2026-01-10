mod catalog;
mod docker;
mod health;
mod port;

pub use catalog::{CatalogError, ImageTag, RegistryImage, RepositoryName, list_images};
use docker::{Container, ContainerDiscovered};
use snafu::{ResultExt, Snafu};
use std::time::Duration;
pub use port::{
    RegistryConfig, RegistryRuntimeConfig, RegistryState, RegistryStatus, RegistryUrl,
};

const REGISTRY_STARTUP_TIMEOUT_SECS: u64 = 10;

/// Start the local OCI registry container
///
/// Creates and starts a Docker container running the registry image. If the container
/// already exists but is stopped, it will be started. Waits for the registry to become
/// healthy before returning.
pub fn start(config: &RegistryRuntimeConfig) -> Result<RegistryUrl, RegistryError> {
    use registry_error::*;

    let container = Container::new(
        config.container_name.clone(),
        config.port,
        config.volume_name.clone(),
        config.image.clone(),
    );

    let running = match container.discover().context(DockerSnafu)? {
        ContainerDiscovered::Running(c) => c,
        ContainerDiscovered::Stopped(c) => c.start().context(DockerSnafu)?,
        ContainerDiscovered::NotCreated(c) => c.create().context(DockerSnafu)?,
    };

    let url = running.url();

    health::wait_until_ready(&url, Duration::from_secs(REGISTRY_STARTUP_TIMEOUT_SECS))
        .context(HealthCheckSnafu)?;

    Ok(url)
}

/// Stop the local OCI registry container
///
/// Stops the registry container if it is running. Does nothing if the container
/// is already stopped or does not exist.
pub fn stop(config: &RegistryRuntimeConfig) -> Result<(), RegistryError> {
    use registry_error::*;
