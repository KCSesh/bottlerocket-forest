# Feature 0004: Local OCI Registry Management - Technical Design

## Overview

Forester will manage a local Docker-based OCI registry for development workflows. The registry stores Bottlerocket kits and SDKs, eliminating the need for external registry access during local development.

Design Philosophy: Use Rust's type system to enforce container lifecycle correctness at compile time. Wrap Docker operations in domain types that prevent invalid state transitions. Keep Docker interaction isolated behind clean abstractions.

## Critical Constraints

| ID | Constraint | Rationale | Anti-pattern to avoid |
|----|------------|-----------|----------------------|
| CC-1 | Container lifecycle must use typestate pattern | Prevents runtime errors from invalid operations (e.g., stopping non-running container) | Using runtime checks or Option<State> enums |
| CC-2 | Registry must be accessible at localhost:<port> | Bottlerocket build tools expect HTTP registry endpoint | Using Docker internal networking or Unix sockets |
| CC-3 | Data persistence via named Docker volumes | Survives container restarts, enables clean separation of data/container lifecycle | Bind mounts or ephemeral storage |
| CC-4 | Port-based naming for container/volume | Allows multiple registry instances for parallel workflows | Hardcoded singleton names |

## Architecture

CLI Layer (cli/registry.rs)
  ↓
Registry Operations (registry/mod.rs)
  ↓
├─ Docker Adapter (registry/docker.rs)
├─ Health Checker (registry/health.rs)
├─ Catalog Reader (registry/catalog.rs)
└─ Domain Types (registry/types.rs)


### Layer Responsibilities

Domain - Registry types (RegistryPort, ContainerName, RegistryUrl), configuration, status representation

Ports - Implicit traits: Docker operations, HTTP health checks, catalog API queries

Adapters - Docker CLI/API wrapper, HTTP client for health/catalog, filesystem for config

Application - CLI command handlers that orchestrate registry operations

## Domain Model

### Core Types

RegistryPort
- Validated u16 ≥ 1024 (non-privileged)
- Default: 5000
- Used to derive container/volume names

ContainerName
- Non-empty string, format: forester-registry-{port}
- Unique identifier for Docker container

VolumeName
- Non-empty string, format: forester-registry-data-{port}
- Persistent storage identifier

ImageRef
- Non-empty string, default: registry:2
- OCI image reference for registry container

RegistryUrl
- Combines host + port into HTTP endpoint
- Format: http://localhost:{port}

RegistryState
- NotCreated: Container doesn't exist
- Stopped: Container exists but not running
- Running { url }: Container running and accessible

RegistryConfig
- User-facing configuration (port, image)
- Serializable, loaded from config files/env

RegistryRuntimeConfig
- Runtime configuration with derived names
- Built from RegistryConfig via .into_runtime()

### Domain Operations

start(config: &RegistryRuntimeConfig) -> Result<RegistryUrl, RegistryError>
- Discovers container state, creates/starts as needed
- Waits for health check before returning
- Idempotent: returns URL if already running
- **Invariants**: Never returns until registry responds to HTTP requests

stop(config: &RegistryRuntimeConfig) -> Result<(), RegistryError>
- Stops running container, preserves volume
- Idempotent: succeeds if already stopped
- **Invariants**: Volume data always preserved

status(config: &RegistryRuntimeConfig) -> Result<RegistryStatus, RegistryError>
- Returns current state + volume existence
- Non-mutating, always succeeds if Docker accessible

clean(config: &RegistryRuntimeConfig) -> Result<(), RegistryError>
- Stops container, removes container and volume
- Destructive: permanently deletes registry data
- Idempotent: succeeds if already clean

logs(config: &RegistryRuntimeConfig, follow: bool) -> Result<(), RegistryError>
- Streams container logs to stdout
- Fails if container not running

list_images(url: &RegistryUrl) -> Result<Vec<RegistryImage>, CatalogError>
- Queries registry catalog API
- Returns repository names and tags
- Fails if registry not accessible

### Error Types

RegistryError
- Docker { source: DockerError }: Container/volume operations failed
- HealthCheck { source: HealthError }: Registry didn't become healthy
- ContainerNotRunning: Operation requires running container

DockerError
- CommandFailed: Docker CLI command returned non-zero
- ParseError: Unexpected Docker output format
- NotFound: Container/volume doesn't exist

HealthError
- Timeout: Registry didn't respond within timeout
- Unreachable: Network/connection error

CatalogError
- HttpError: Failed to query catalog API
- ParseError: Invalid JSON response

## Boundaries & Adapters

Trait: ContainerOperations (implicit in docker.rs)
- Purpose: Abstract Docker container lifecycle
- Key methods: discover(), create(), start(), stop(), remove()
- Implementations: Docker CLI via std::process::Command

Trait: HealthCheck (implicit in health.rs)
- Purpose: Verify registry readiness
- Key methods: wait_until_ready(url, timeout)
- Implementations: HTTP polling with exponential backoff

Trait: CatalogClient (implicit in catalog.rs)
- Purpose: Query registry contents
- Key methods: list_repositories(), list_tags(repo)
- Implementations: HTTP client for Docker Registry V2 API

## Module Structure

src/
├── cli/
│   ├── mod.rs           # Command dispatch
│   ├── registry.rs      # Registry subcommands
│   └── theme.rs         # Output styling
├── registry/
│   ├── mod.rs           # Public API (start, stop, status, clean, logs)
│   ├── docker.rs        # Container lifecycle with typestate
│   ├── types.rs         # Domain types
│   ├── health.rs        # Health check polling
│   └── catalog.rs       # Image listing
└── config.rs            # Configuration loading


## Typestate Pattern for Container Lifecycle

### States

Container\<NotCreated\>
- Container doesn't exist in Docker
- Operations: discover(), create()

Container\<Stopped\>
- Container exists but not running
- Operations: start(), remove()

Container\<Running\>
- Container exists and running
- Operations: stop(), url(), logs()

### State Transitions

NotCreated --create()--> Running
NotCreated --discover()--> NotCreated | Stopped | Running
Stopped --start()--> Running
Stopped --remove()--> NotCreated
Running --stop()--> Stopped


### Implementation

rust
pub struct Container<S: ContainerState> {
    name: ContainerName,
    port: RegistryPort,
    volume: VolumeName,
    image: ImageRef,
    _state: PhantomData<S>,
}


Each state transition consumes self and returns a new Container<NewState>, making invalid operations impossible at compile time.

## Design Patterns

Typestate Pattern: Container lifecycle states encoded in type system. Prevents calling stop() on non-running container, start() on running container, etc. Compiler enforces valid state machines.

Newtype Pattern: Domain types (RegistryPort, ContainerName, etc.) wrap primitives with validation. Uses nutype crate for declarative validation rules.

Builder Pattern: RegistryRuntimeConfig uses bon crate for ergonomic construction with derived fields (container/volume names from port).

Error Context Pattern: Uses snafu for error context propagation. Each layer adds context without losing underlying cause.

## Implementation Guidance

- Use std::process::Command for Docker CLI interaction initially; consider bollard crate for async/advanced features later
- Health checks should poll with exponential backoff (100ms, 200ms, 400ms, ...) up to timeout
- Container discovery should parse docker inspect JSON output
- Volume operations use docker volume subcommands
- Catalog API follows Docker Registry HTTP API V2 spec: GET /v2/_catalog, GET /v2/{name}/tags/list
- All Docker operations should capture stderr for error messages
- Configuration loading should support environment variables with FORESTER_REGISTRY_ prefix

## Testing Strategy

- Unit tests: Domain type validation (invalid ports, empty names)
- Integration tests: Full lifecycle (start → status → stop → clean) with real Docker
- Mock tests: Health check retry logic with simulated failures
- Edge cases: Port conflicts, missing Docker daemon, network timeouts, corrupted registry data

## Design Decisions

### DD-1: Typestate pattern for container lifecycle

Decision: Use compile-time state tracking via generic type parameters

Alternatives considered:
1. Runtime state enum with Result returns for invalid operations
2. Separate types for each state without shared Container wrapper
3. Typestate pattern with PhantomData (chosen)

Rationale: Rust's type system can enforce correctness that would otherwise require runtime checks. The compiler prevents invalid operations (e.g., stopping a non-running container), reducing bugs and improving API clarity.

Implications: Slightly more complex type signatures, but eliminates entire classes of runtime errors. Discovery operation returns enum of possible states, then typestate takes over.

### DD-2: Port-based naming for container and volume

Decision: Derive names from port number (forester-registry-{port})

Alternatives considered:
1. Hardcoded singleton names (forester-registry)
2. User-specified names in config
3. Port-based derivation (chosen)

Rationale: Enables multiple registry instances for parallel workflows (e.g., testing different configurations). Automatic derivation prevents naming conflicts and reduces configuration burden.

Implications: Users can run multiple registries by specifying different ports. Default port (5000) gives predictable default names.

### DD-3: Docker CLI vs bollard crate

Decision: Start with std::process::Command, migrate to bollard if needed

Alternatives considered:
1. Docker CLI via Command (chosen for initial implementation)
2. bollard crate (async Docker API client)
3. docker_api crate

Rationale: CLI is simpler for initial implementation, requires no additional dependencies, and Docker is already required for Bottlerocket builds. Bollard offers better performance and error handling but adds complexity.

Implications: May need to parse CLI output (JSON from docker inspect). Can migrate to bollard later without changing public API.

### DD-4: Health check strategy

Decision: Poll HTTP endpoint with exponential backoff until timeout

Alternatives considered:
1. Single HTTP request with no retry
2. Fixed-interval polling
3. Exponential backoff polling (chosen)

Rationale: Registry takes time to start. Exponential backoff balances responsiveness (quick success) with efficiency (fewer requests during slow starts).

Implications: Start command blocks until healthy or timeout. Timeout should be configurable but default to 10 seconds.

## Notes

- Registry uses official registry:2 image from Docker Hub
- Data volume persists across container removals, enabling safe container recreation
- Health check queries GET /v2/ endpoint (registry API base)
- Catalog API requires registry to be running; returns empty list for new registry
- CLI output uses theme module for consistent styling (success/error/info)
- Configuration supports future extensibility (auth, TLS, custom registry images)