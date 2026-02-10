# throttlesys - Requirements Specification

## Overview

This specification defines the requirements for throttlesys, a build parallelism coordinator that works across Docker container boundaries. It translates the concept in [concept.md](./concept.md) into testable requirements.

**Requirement ID Prefix**: `THR` (THRottlesys)

## Functional Requirements

### Throttle Server

#### THR-1: Server Creation

**WHEN** twoliter starts a build command (`build kit`, `build variant`)  
**THEN** the system **SHALL** create a throttle server with a configurable number of tokens

**WHERE** the user has set `BUILDSYS_JOBS=N` (N > 0)  
**THEN** the system **SHALL** create the server with N tokens

**WHERE** `BUILDSYS_JOBS` is not set  
**THEN** the system **SHALL** default to 8 tokens

**WHERE** `BUILDSYS_JOBS=0`  
**THEN** the system **SHALL** treat this as `BUILDSYS_JOBS=1`

#### THR-2: Protocol Support

**WHEN** creating a throttle server  
**THEN** the system **SHALL** wrap the `jobserver` crate to support both pipe and FIFO protocols

#### THR-3: Socket Binding

**WHEN** the throttle server is created  
**THEN** the system **SHALL** bind to an abstract Unix domain socket

**THEN** the socket name **SHALL** be unique per build invocation (e.g., `throttlesys-{token}-{nocache}`)

**WHERE** the socket name is already bound  
**THEN** the system **SHALL** fail with a clear error message

#### THR-4: Token Request Handling

**WHILE** the throttle server is running  
**WHEN** a client sends a token acquire request  
**THEN** the system **SHALL** block until a token is available

**THEN** the system **SHALL** return the token to the requesting client

#### THR-5: Token Release Handling

**WHILE** the throttle server is running  
**WHEN** a client sends a token release request  
**THEN** the system **SHALL** return the token to the pool

#### THR-6: Token Ownership Tracking

**WHILE** the throttle server is running  
**THEN** the system **SHALL** track which client connection owns each outstanding token

#### THR-7: Token Fairness

**WHILE** multiple clients are waiting for tokens  
**THEN** the system **SHALL** grant tokens in FIFO order based on request arrival time

#### THR-8: Async Integration

**WHILE** the throttle server is running  
**THEN** the system **SHALL** handle blocking jobserver operations without blocking the async event loop

**THEN** the system **SHALL** use `spawn_blocking` for jobserver crate calls

### Throttle Client (Container Side)

#### THR-9: Client Binary

**THEN** the system **SHALL** provide a `throttle-client` binary in the SDK container image

#### THR-10: Client Socket Connection

**WHEN** a Docker build starts  
**WHERE** `THROTTLE_SOCKET` build arg is set and non-empty  
**THEN** the throttle client **SHALL** connect to the server via the abstract socket

**WHERE** connection cannot be established within 30 seconds  
**THEN** the system **SHALL** fail with an error indicating the server is unavailable

#### THR-11: Local Jobserver Presentation

**WHILE** the client is connected to the server  
**THEN** the system **SHALL** present a local jobserver interface

**THEN** the system **SHALL** set both `CARGO_MAKEFLAGS` and `MAKEFLAGS` environment variables

**THEN** cargo and make inside the container **SHALL** be able to use this interface without modification

#### THR-12: Token Proxying

**WHILE** the client is running  
**WHEN** a process reads from the local jobserver pipe (acquire)  
**THEN** the system **SHALL** forward the request to the throttle server and block until granted

**WHILE** the client is running  
**WHEN** a process writes to the local jobserver pipe (release)  
**THEN** the system **SHALL** forward the release to the throttle server

### twoliter Integration

#### THR-13: Server Lifecycle in twoliter

**WHEN** twoliter executes a build command  
**THEN** the throttle server **SHALL** be spawned as a tokio task before invoking cargo-make

**THEN** the socket name **SHALL** be passed to cargo-make via `THROTTLE_SOCKET` environment variable

**WHEN** the build completes (success or failure)  
**THEN** the throttle server task **SHALL** be shut down

#### THR-14: buildsys Integration

**WHEN** buildsys invokes `docker build`  
**THEN** the system **SHALL** pass the socket name via `--build-arg THROTTLE_SOCKET=<name>`

#### THR-15: Network Namespace Requirement

**WHEN** invoking `docker build` with throttle support  
**THEN** the system **SHALL** use `--network host` to enable abstract socket access

**WHERE** `--network host` is unavailable  
**THEN** the system **SHALL** fail with a clear error explaining the requirement

#### THR-16: Client Initialization in Dockerfile

**WHEN** the Dockerfile executes  
**THEN** the throttle client **SHALL** be initialized before any cargo or make invocations

### Lifecycle Management

#### THR-17: Startup Sequence

**WHEN** twoliter starts a build  
**THEN** throttle server creation **SHALL** complete before cargo-make invocation  
**THEN** socket binding **SHALL** complete before any docker build

#### THR-18: Shutdown Sequence

**WHEN** a build completes normally  
**THEN** the server **SHALL** wait up to 5 seconds for in-flight token requests  
**THEN** the server **SHALL** close all client connections  
**THEN** the server **SHALL** release all resources

#### THR-19: Signal Handling

**WHEN** twoliter receives SIGINT or SIGTERM  
**THEN** the server **SHALL** terminate all client connections within 5 seconds  
**THEN** the server **SHALL** log final token state  
**THEN** the server **SHALL** exit cleanly

### Failure Handling

#### THR-20: Token Recovery on Client Disconnect

**WHILE** the throttle server is running  
**WHEN** a client connection closes unexpectedly  
**THEN** the system **SHALL** return all tokens owned by that client to the pool within 1 second

#### THR-21: Graceful Client Disconnect

**WHEN** a client sends a DISCONNECT message  
**THEN** the server **SHALL** release all tokens held by that client  
**THEN** the server **SHALL** close the connection

#### THR-22: Build Continuation After Container Failure

**WHEN** a container crashes or is OOM-killed  
**THEN** the server **SHALL** recover tokens per THR-20  
**THEN** other in-flight builds **SHALL** continue without interruption

#### THR-23: Graceful Degradation

**WHERE** the throttle server becomes unavailable during a build  
**THEN** in-flight container builds **SHALL** fail with a clear error message  
**THEN** the system **SHALL NOT** deadlock

#### THR-24: Missing Throttle Socket

**WHILE** the client is starting inside a container  
**WHERE** `THROTTLE_SOCKET` is not set or empty  
**THEN** the system **SHALL** fail with an error indicating throttle server is required  
**THEN** the system **SHALL NOT** fall back to uncoordinated parallelism

#### THR-25: Protocol Mismatch

**WHILE** the client is connecting  
**WHERE** the server and client protocol versions are incompatible  
**THEN** the system **SHALL** fail with a clear error message indicating version mismatch

### Concurrent Builds

#### THR-26: Multiple Build Invocations

**WHEN** multiple `twoliter build` commands run concurrently on the same host  
**THEN** each build **SHALL** have its own independent throttle server  
**THEN** builds **SHALL NOT** interfere with each other's token pools

## Non-Functional Requirements

### Performance

#### THR-27: Token Acquisition Latency

**WHILE** tokens are available in the pool  
**THEN** token acquisition **SHALL** complete in under 10 milliseconds (p99)

#### THR-28: Server Overhead

**THEN** the throttle server **SHALL** add negligible latency per token operation (< 1ms typical)

### Scalability

#### THR-29: Concurrent Client Support

**THEN** the throttle server **SHALL** support at least 64 concurrent client connections

#### THR-30: Memory Efficiency

**THEN** server memory usage **SHALL** be O(number of clients), not O(number of token transactions)

### Compatibility

#### THR-31: Cargo Compatibility

**THEN** the system **SHALL** work with cargo versions that support the `jobserver` crate

#### THR-32: Make Compatibility

**THEN** the system **SHALL** work with GNU Make 4.0+ (pipe protocol)  
**THEN** the system **SHALL** work with GNU Make 4.4+ (FIFO protocol)

### Observability

#### THR-33: Token State Logging

**WHILE** the throttle server is running  
**THEN** the system **SHALL** log token acquisitions and releases at debug level  
**THEN** the system **SHALL** log token recovery events at info level  
**THEN** the system **SHALL** log errors at warning level

#### THR-34: Build Parallelism Visibility

**WHEN** a build completes  
**THEN** the system **SHALL** report peak concurrent jobs and total token transactions

## Appendix A: Protocol Message Format

Messages between throttle server and client over the Unix socket:

```
+--------+--------+--------+------------------+
| Version| Type   | Length | Payload          |
| 1 byte | 1 byte | 2 bytes| variable         |
+--------+--------+--------+------------------+

Version: 0x01
Types: 0x01=AcquireRequest, 0x02=AcquireResponse, 0x03=ReleaseRequest,
       0x04=ReleaseResponse, 0x05=Disconnect, 0x06=Error
```

## Appendix B: Environment Variables

| Variable | Set By | Used By | Description |
|----------|--------|---------|-------------|
| `BUILDSYS_JOBS` | User | twoliter | Number of tokens to create |
| `THROTTLE_SOCKET` | twoliter | buildsys, Dockerfile | Abstract socket name for throttle server |
| `CARGO_MAKEFLAGS` | throttle client | cargo | Jobserver auth string for cargo |
| `MAKEFLAGS` | throttle client | make | Jobserver auth string for make |

## Appendix C: Integration Points

```
twoliter build variant
        │
        ├──▶ Spawn ThrottleServer (tokio task)
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
                            │                   └──▶ throttle-client connects, sets CARGO_MAKEFLAGS
                            │                             │
                            │                             └──▶ cargo (in container) uses jobserver
                            │
                            ├──▶ buildsys (package 2) ...
                            │
                            └──▶ buildsys (package N) ...
```
