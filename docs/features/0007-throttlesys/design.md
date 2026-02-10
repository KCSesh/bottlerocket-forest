# throttlesys - Technical Design

## Overview

This document specifies the technical architecture for throttlesys, a build parallelism coordinator that works across Docker container boundaries. It implements the requirements in [requirements.md](./requirements.md) and realizes the vision described in [concept.md](./concept.md).

**Design Philosophy**: Focus on types, their relationships, and key architectural decisions. Leave specific coding decisions to implementors.

throttlesys is a new tool in `twoliter/tools/` following the pattern of pipesys and buildsys. It consists of:
- **ThrottleServer** - Host-side component that wraps `jobserver::Client` and exposes it over UDS
- **ThrottleClient** - Container-side component that connects to server and presents local jobserver to cargo/make
- **twoliter integration** - Spawns server, passes socket to buildsys
- **buildsys integration** - Passes socket to docker build

## Critical Constraints

Non-negotiable implementation requirements. Deviating from these requires updating this document first.

| ID | Constraint | Rationale | Anti-pattern to avoid |
|----|------------|-----------|----------------------|
| CC-1 | ThrottleServer MUST wrap `jobserver::Client`, not implement custom token pool | The crate handles protocol details and supports both pipe and FIFO protocols (THR-2) | Implementing custom pipe-based token passing |
| CC-2 | Token recovery MUST complete within 1 second of client disconnect | Leaked tokens reduce parallelism; slow recovery causes cascading delays (THR-20) | Waiting for timeout or manual intervention |
| CC-3 | Client MUST set both `CARGO_MAKEFLAGS` and `MAKEFLAGS` | Cargo uses one, make uses the other (THR-11) | Setting only one environment variable |
| CC-4 | Protocol messages MUST use big-endian (network byte order) | Standard for network protocols; see Appendix A in requirements.md | Using native endianness |
| CC-5 | Blocking jobserver operations MUST use `spawn_blocking` | The `jobserver` crate is blocking-only; blocking tokio causes deadlocks (THR-8) | Calling acquire() directly in async context |
| CC-6 | Container builds MUST fail if server unavailable, not fall back | Fallback defeats the purpose and can cause OOM (THR-24) | Silently creating local jobserver |
| CC-7 | Token requests MUST be granted in FIFO order | Prevents starvation (THR-7) | Random or LIFO ordering |
| CC-8 | ThrottleServer MUST be spawned BEFORE cargo-make invocation | Server must be ready before any docker builds start (THR-17) | Starting server lazily on first connection |

## Architecture

### System Integration

```
twoliter build variant
        │
        ├──▶ Spawn ThrottleServer (tokio task, THR-13)
        │         │
        │         └──▶ Bind to @throttlesys-{token}-{nocache}
        │
        ├──▶ Set THROTTLE_SOCKET env var
        │
        └──▶ CargoMake::exec()
                  │
                  └──▶ cargo build --jobs N
                            │
                            ├──▶ buildsys (package 1)
                            │         │
                            │         └──▶ docker build --build-arg THROTTLE_SOCKET=...
                            │                   │
                            │                   └──▶ throttle-client → CARGO_MAKEFLAGS
                            │
                            └──▶ buildsys (package N) ...
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           HOST                                  │
│  ┌─────────────┐     ┌──────────────┐     ┌─────────────────┐  │
│  │  twoliter   │────▶│ jobserver::  │◀───▶│ ThrottleServer  │  │
│  │             │     │ Client(N)    │     │ (tokio + UDS)   │  │
│  └─────────────┘     └──────────────┘     └────────┬────────┘  │
│                                                    │           │
│ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ │
│                     --network host                 │           │
│  ┌────────────────────────────────────────────────┼──────────┐ │
│  │                 DOCKER CONTAINER               │          │ │
│  │  ┌──────────────┐     ┌────────────────────────▼───────┐  │ │
│  │  │    cargo     │────▶│      ThrottleClient           │  │ │
│  │  │  (build.rs)  │     │  (local pipe ◀──▶ UDS socket) │  │ │
│  │  └──────────────┘     └────────────────────────────────┘  │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

**twoliter** (`twoliter/twoliter/src/`) - Creates ThrottleServer, passes socket to cargo-make via env

**buildsys** (`twoliter/tools/buildsys/`) - Reads THROTTLE_SOCKET from env, passes to docker as build arg

**throttlesys** (`twoliter/tools/throttlesys/`) - ThrottleServer and ThrottleClient implementations

**SDK image** - Contains `throttle-client` binary for container-side use

## Domain Model

### Core Types

**ThrottleConfig** (implements THR-1)
- Purpose: Configuration for the throttle server
- Key properties: `tokens: usize` (from BUILDSYS_JOBS, default 8, minimum 1), `socket_name: String`
- Validation: `tokens >= 1`, `socket_name` non-empty

**Token**
- Purpose: Represents permission to run one job
- Key properties: `Token(u8)` where value is 1..=N (not 0, to distinguish from EOF)

**TokenOwnership**
- Purpose: Tracks which client owns which tokens for recovery
- Key properties: `client_id: ClientId`, `tokens: Vec<Token>`, `acquired_at: Instant`
- Invariant: Sum of all owned tokens + available tokens = total tokens

**PendingRequest** (implements THR-7)
- Purpose: Queued token request for FIFO ordering
- Key properties: `client_id: ClientId`, `requested_at: Instant`, `response_channel: oneshot::Sender<Token>`

**Message**
- Purpose: Protocol message between server and client
- Variants: `AcquireRequest`, `AcquireResponse(Token)`, `ReleaseRequest(Token)`, `ReleaseResponse`, `Disconnect`, `Error(String)`


### Domain Operations

**ThrottleServer::start(config: ThrottleConfig) -> Result<Self, ServerStartError>** (THR-1, THR-3)
- Creates jobserver with N tokens, binds to abstract socket
- **Invariants**: Socket must not already be bound

**ThrottleServer::acquire_token(client_id: ClientId) -> Result<Token, AcquireError>** (THR-4, THR-7)
- Queues request, grants in FIFO order (CC-7)
- Uses `spawn_blocking` for jobserver crate calls (CC-5)
- Records ownership for recovery

**ThrottleServer::release_token(client_id: ClientId, token: Token) -> Result<(), ReleaseError>** (THR-5)
- Releases token, removes from ownership, wakes next pending request

**ThrottleServer::recover_tokens(client_id: ClientId)** (THR-20, THR-22)
- Called on client disconnect, returns all owned tokens within 1 second (CC-2)

**ThrottleServer::shutdown(timeout: Duration)** (THR-18, THR-19)
- Graceful shutdown: stop accepting, drain in-flight ops, close connections

**ThrottleClient::connect(socket_name: &str) -> Result<Self, ClientConnectError>** (THR-10, THR-11)
- Connects to server, creates local pipe, sets `CARGO_MAKEFLAGS` and `MAKEFLAGS` (CC-3)

### Error Types

**ServerStartError**
- `SocketCollision { socket_name }` - Socket already bound (THR-3)
- `JobserverCreate { source }` - Failed to create jobserver::Client (THR-1, THR-2)

**ClientConnectError**
- `SocketNotSet` - THROTTLE_SOCKET not set (CC-6, THR-24)
- `ConnectionFailed { socket, source }` - Cannot reach server (THR-10)
- `ConnectionTimeout { timeout }` - Server didn't respond (THR-10)
- `VersionMismatch { expected, actual }` - Protocol incompatibility (THR-25)

## Integration Design

### twoliter Changes (THR-13, THR-17)

**File: `twoliter/twoliter/src/cmd/build.rs`**

Before invoking `CargoMake::exec()`, spawn the throttle server:

```rust
// In BuildKit::run() and BuildVariant::run()
let tokens = env::var("BUILDSYS_JOBS").ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(8)
    .max(1);

let socket_name = format!("throttlesys-{}-{}", token, nocache);  // Unique per build (THR-26)
let server = ThrottleServer::start(ThrottleConfig { tokens, socket_name: socket_name.clone() })?;

env::set_var("THROTTLE_SOCKET", &socket_name);

// ... existing CargoMake::exec() call ...

server.shutdown(Duration::from_secs(5)).await;
```

### buildsys Changes (THR-14, THR-15)

**File: `twoliter/tools/buildsys/src/builder.rs`**

Pass THROTTLE_SOCKET to docker build:

```rust
// In DockerBuild::build_args()
if let Ok(socket) = env::var("THROTTLE_SOCKET") {
    args.build_arg("THROTTLE_SOCKET", &socket);
}
```

### Dockerfile Changes (THR-16)

**File: `twoliter/twoliter/embedded/build.Dockerfile`**

Initialize throttle client before cargo invocations:

```dockerfile
ARG THROTTLE_SOCKET
RUN --mount=... \
    if [ -n "$THROTTLE_SOCKET" ]; then \
        throttle-client --socket "$THROTTLE_SOCKET" --daemonize || exit 1; \
    fi && \
    # ... existing cargo/rpmbuild commands ...
```

### SDK Image Changes

The `throttle-client` binary must be available in the SDK image. Options:
1. Build as part of SDK (preferred - always available)
2. Copy from twoliter build context

## Module Structure

```
twoliter/tools/throttlesys/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API: ThrottleServer, ThrottleClient
    ├── server.rs           # ThrottleServer implementation
    ├── client.rs           # ThrottleClient implementation  
    ├── protocol.rs         # Wire protocol encode/decode
    ├── ownership.rs        # Token ownership tracking + FIFO queue
    └── bin/
        └── throttle-client.rs  # CLI binary for container use
```

All modules must stay under 550 LOC limit (enforced by linter).

## Design Patterns

**Proxy Pattern**: ThrottleServer acts as intermediary between container clients and the real jobserver.

**Adapter Pattern**: Wraps blocking `jobserver::Client` to work with async runtime via `spawn_blocking`.

**Message Passing**: Protocol uses explicit message types for versioning and clear semantics.

## Testing Strategy

**Unit tests** (domain logic with mock adapters):
- Protocol encode/decode roundtrip
- Token ownership: acquire, release, recover on disconnect
- FIFO ordering: multiple pending requests granted in order (CC-7)

**Integration tests** (real adapters):
- ThrottleServer + ThrottleClient over real UDS
- Token acquisition end-to-end
- Recovery on simulated client crash
- Multiple concurrent clients

**End-to-end tests**:
- Full twoliter build with throttlesys enabled
- Verify parallelism is actually bounded

## Design Decisions

### DD-1: New Tool vs Extend pipesys

**Decision**: Create new `throttlesys` tool rather than extending pipesys.

**Rationale**: Different concerns - pipesys passes file descriptors, throttlesys coordinates parallelism. Separate tools are easier to test and maintain.

### DD-2: Server in twoliter vs buildsys

**Decision**: Spawn ThrottleServer in twoliter, not buildsys.

**Rationale**: twoliter is the build orchestrator; it knows when the build starts and ends. buildsys runs per-package and would create multiple servers.

### DD-3: Fail vs Fallback When Server Unavailable

**Decision**: Fail with clear error, do not fall back to uncoordinated parallelism.

**Rationale**: Uncoordinated parallelism defeats the feature's purpose and can cause OOM (THR-24).

### DD-4: Follow OUTPUT_SOCKET Pattern

**Decision**: Use the OUTPUT_SOCKET pattern (tokio task in same process) rather than BYPASS_SOCKET pattern (separate container).

**Rationale**: Throttle server doesn't need filesystem mounting - just socket communication. Simpler lifecycle management.

## Wire Protocol

All multi-byte integers are big-endian (network byte order) per CC-4.

```
+--------+--------+--------+------------------+
| Version| Type   | Length | Payload          |
| 1 byte | 1 byte | 2 bytes| variable         |
+--------+--------+--------+------------------+

Version: 0x01
Types: 0x01=AcquireRequest, 0x02=AcquireResponse, 0x03=ReleaseRequest,
       0x04=ReleaseResponse, 0x05=Disconnect, 0x06=Error
```

## Observability (THR-33, THR-34)

**Logging Strategy:**
- DEBUG: Every token acquire/release (THR-33)
- INFO: Client connect/disconnect, token recovery events (THR-33)
- WARN: Connection timeouts, protocol errors (THR-33)

**Metrics:**
- `tokens_acquired`, `tokens_released`, `tokens_recovered` counters
- `peak_concurrent_jobs` gauge
- `client_connections` gauge

**Build Completion Report (THR-34):**
```
throttlesys: peak_concurrent=8, total_acquired=1247, total_released=1247, recovered=3
```

## Performance Considerations (THR-27, THR-28, THR-29, THR-30)

**Latency (THR-27, THR-28):**
- UDS round-trip: ~0.1ms
- Token acquire with available token: <1ms typical, <10ms p99
- Overhead per operation: <1ms

**Scalability (THR-29, THR-30):**
- Tokio handles thousands of concurrent connections
- Memory: O(clients), ~1KB per connection
- Support 64+ concurrent clients easily

## Notes

- Reference requirements by ID (THR-N) when relevant
- Reference critical constraints by ID (CC-N) in code review
- Research findings are at `./planning/throttlesys-research/research-summary.md`
