# brdev

brdev is a Rust CLI tool that orchestrates development workflows across the Bottlerocket Forest.
It provides higher-level commands for managing local development infrastructure.

## Purpose

brdev provides:

- **Local OCI Registry** - Run a local Docker registry for kit development

## Installation

Build from source:

```bash
cd bottlerocket-forest
cargo build --release -p brdev
```

The binary will be at `target/release/brdev`.

## Usage

### Registry Management

The registry is grove-aware.
All registry commands must be run from within a grove directory.
Each grove gets its own isolated registry container and data volume.

Start a local OCI registry for development:

```bash
brdev registry start
```

Check registry status:

```bash
brdev registry status
```

List published images:

```bash
brdev registry list
```

View registry logs:

```bash
brdev registry logs
```

Stop the registry (preserves data):

```bash
brdev registry stop
```

Remove registry and all data:

```bash
brdev registry clean
```

### Configuration

Container and volume names are derived from the grove name as `brdev-registry-{grove}` and `brdev-registry-data-{grove}`.

The registry port is automatically derived from a hash of the grove name (range 5001-5999).
To override the port, create a `.grove/registry-port` file containing the desired port number.

The registry image defaults to `registry:2` and can be overridden with the `FORESTER_REGISTRY_IMAGE` environment variable.

## Requirements

- Rust toolchain (for building)
- Docker installed and running
- User must be in the `docker` group (or have Docker permissions)

## Development

### Building

Build from the workspace root:

```bash
make build          # Development build
make release-build  # Optimized build
```

### Code Quality

Run all quality checks from the workspace root:

```bash
make integ  # Full test suite (fmt, clippy, deny, tests)
make check  # Quick validation (fmt, clippy, deny, unit tests)
```

Integration tests use the `serial_test` crate with `#[serial(registry)]` to ensure tests that manipulate the Docker registry run one at a time.
Tests use a dedicated test port (5555), and each test starts with a clean state and cleans up after itself.
