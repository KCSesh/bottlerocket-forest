//! Port and URL types for the local OCI registry.

use bon::Builder;
use nutype::nutype;
use std::fmt;

/// Port number for the registry HTTP server.
///
/// Enforces a minimum value of 1024 to avoid privileged ports.
#[nutype(
    validate(greater_or_equal = 1024),
    derive(Debug, Clone, Copy, PartialEq, Eq)
)]
pub struct RegistryPort(u16);

impl Default for RegistryPort {
    fn default() -> Self {
        Self::try_new(5000).unwrap()
    }
}

/// HTTP URL for accessing the registry.
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct RegistryUrl {
    host: String,
    port: RegistryPort,
}

impl RegistryUrl {
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port.into_inner()
    }
}

impl fmt::Display for RegistryUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "http://{}:{}", self.host, self.port.into_inner())
    }
}

/// Name of the Docker container running the registry.
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, AsRef, Deref)
)]
pub(crate) struct ContainerName(String);

/// Name of the Docker volume for persistent registry storage.
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, AsRef, Deref)
)]
pub(crate) struct VolumeName(String);

/// OCI image reference for the registry container.
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, AsRef, Deref)
)]
pub(crate) struct ImageRef(String);

impl Default for ImageRef {
    fn default() -> Self {
        Self::try_new("registry:2").unwrap()
    }
}

/// Current lifecycle state of the registry container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryState {
    NotCreated,
    Stopped,
    Running { url: RegistryUrl },
}

/// Complete status information for the registry.
#[derive(Debug, Clone, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct RegistryStatus {
    pub state: RegistryState,
    pub volume_exists: bool,
}

/// Runtime registry configuration with derived container and volume names.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RegistryRuntimeConfig {
    pub port: RegistryPort,
    pub(crate) image: ImageRef,
    pub(crate) container_name: ContainerName,
    pub(crate) volume_name: VolumeName,
}

impl RegistryRuntimeConfig {
    /// Creates a new runtime config (crate-internal).
    pub(crate) fn new(
        port: RegistryPort,
        image: ImageRef,
        container_name: ContainerName,
        volume_name: VolumeName,
    ) -> Self {
        Self { port, image, container_name, volume_name }
    }
}

impl Default for RegistryRuntimeConfig {
    fn default() -> Self {
        let port = RegistryPort::default();
        let container_name =
            ContainerName::try_new(format!("forester-registry-{}", port.into_inner())).unwrap();
        let volume_name =
            VolumeName::try_new(format!("forester-registry-data-{}", port.into_inner())).unwrap();
        Self {
            port,
            image: ImageRef::default(),
            container_name,
            volume_name,
        }
    }
}
