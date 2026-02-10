# Test Plan: throttlesys

## Overview

Test plan for throttlesys, a build parallelism coordinator that works across Docker container boundaries. Tests verify the throttle server, client, protocol, and integration points.

## Test Types

- **Unit**: Test internal logic in isolation, mocks allowed
- **Integration**: Touch real external resources (sockets, processes), NO mocks
- **Benchmark**: Performance verification requiring statistical measurement
- **Not testable**: Cannot be verified by automated test
- **Out of scope**: Requires complete build environment or external systems

## Requirements Coverage

| Req ID | Test Type | Test Name | Description |
|--------|-----------|-----------|-------------|
| THR-1 | unit | test_config_default_tokens | Verifies default of 8 tokens when BUILDSYS_JOBS unset |
| THR-1 | unit | test_config_zero_becomes_one | Verifies BUILDSYS_JOBS=0 treated as 1 |
| THR-1 | unit | test_config_respects_env | Verifies BUILDSYS_JOBS=N creates N tokens |
| THR-2 | unit | test_server_uses_jobserver_protocol | Verifies server uses jobserver::Client internally |
| THR-2 | integration | test_server_pipe_protocol_mode | Verifies server works with jobserver pipe protocol (Make 4.0+) |
| THR-2 | integration | test_server_fifo_protocol_mode | Verifies server works with jobserver FIFO protocol (Make 4.4+) |
| THR-3 | integration | test_server_binds_abstract_socket | Verifies server binds to abstract UDS |
| THR-3 | integration | test_server_unique_socket_name | Verifies socket name includes token and nocache |
| THR-3 | integration | test_server_socket_collision_error | Verifies clear error when socket already bound |
| THR-4 | integration | test_acquire_blocks_until_available | Verifies acquire blocks when no tokens available |
| THR-4 | integration | test_acquire_returns_token | Verifies token returned to requesting client |
| THR-5 | integration | test_release_returns_token_to_pool | Verifies released token becomes available |
| THR-6 | unit | test_ownership_tracks_client_tokens | Verifies server tracks which client owns which tokens |
| THR-7 | integration | test_fifo_ordering | Verifies tokens granted in request arrival order |
| THR-8 | unit | test_spawn_blocking_for_jobserver | Verifies blocking ops use spawn_blocking |
| THR-9 | out-of-scope | - | SDK image build verification requires full build env |
| THR-10 | integration | test_client_connects_to_socket | Verifies client connects via THROTTLE_SOCKET |
| THR-10 | integration | test_client_connection_timeout | Verifies 30s timeout with clear error |
| THR-11 | integration | test_client_sets_cargo_makeflags | Verifies CARGO_MAKEFLAGS contains jobserver auth string in format --jobserver-auth=R,W |
| THR-11 | integration | test_client_sets_makeflags | Verifies MAKEFLAGS contains -jN and jobserver auth matching CARGO_MAKEFLAGS |
| THR-12 | integration | test_client_proxies_acquire | Verifies local pipe read forwards to server |
| THR-12 | integration | test_client_proxies_release | Verifies local pipe write forwards to server |
| THR-13 | out-of-scope | - | twoliter integration requires full build environment |
| THR-14 | out-of-scope | - | buildsys integration requires full build environment |
| THR-15 | out-of-scope | - | Docker --network host verification requires containers |
| THR-16 | out-of-scope | - | Dockerfile integration requires full build environment |
| THR-17 | unit | test_startup_sequence_order | Verifies ThrottleServer::start() completes and socket is bound before returning |
| THR-18 | integration | test_shutdown_waits_for_inflight | Verifies 5s wait for in-flight requests |
| THR-18 | integration | test_shutdown_closes_connections | Verifies all connections closed on shutdown |
| THR-19 | integration | test_signal_terminates_connections | Verifies SIGINT/SIGTERM handling |
| THR-19 | integration | test_signal_logs_final_state | Verifies token state logged on signal |
| THR-20 | integration | test_token_recovery_on_disconnect | Verifies tokens returned within 1s of client disconnect |
| THR-21 | integration | test_graceful_disconnect_releases | Verifies DISCONNECT message releases tokens |
| THR-22 | integration | test_build_continues_after_crash | Verifies other clients continue after one crashes |
| THR-23 | integration | test_client_fails_if_server_gone | Verifies clear error, no deadlock |
| THR-24 | integration | test_missing_socket_fails | Verifies error when THROTTLE_SOCKET not set |
| THR-24 | unit | test_no_fallback_to_uncoordinated | Verifies no silent fallback behavior |
| THR-25 | integration | test_protocol_version_mismatch | Verifies clear error on version mismatch |
| THR-26 | integration | test_multiple_servers_independent | Verifies concurrent builds have separate pools |
| THR-27 | benchmark | bench_token_acquisition_latency | Measures p99 < 10ms when tokens available |
| THR-28 | benchmark | bench_server_overhead | Measures < 1ms typical per operation |
| THR-29 | integration | test_64_concurrent_clients | Verifies server handles 64+ connections |
| THR-30 | benchmark | test_memory_scales_with_clients | Verifies O(clients) memory, not O(transactions) |
| THR-31 | integration | test_cargo_jobserver_compat | Verifies cargo build respects token limit when using throttle client jobserver |
| THR-32 | integration | test_make_pipe_protocol | Verifies make acquires/releases tokens via pipe FDs when MAKEFLAGS set by throttle client |
| THR-32 | integration | test_make_fifo_protocol | Verifies make acquires/releases tokens via FIFO when MAKEFLAGS set by throttle client |
| THR-33 | unit | test_logs_acquire_release_debug | Verifies debug logging of token ops |
| THR-33 | unit | test_logs_recovery_info | Verifies info logging of recovery events |
| THR-33 | unit | test_logs_errors_warning | Verifies warning logging of errors |
| THR-34 | integration | test_completion_report | Verifies peak concurrent and total transactions reported |

## Critical Constraints Verification

| CC ID | Verification Approach | Test Name(s) |
|-------|----------------------|---------------|
| CC-1 | Verify ThrottleServer wraps jobserver::Client, not custom pool | test_server_uses_jobserver_protocol |
| CC-2 | Measure time from disconnect to token availability | test_token_recovery_on_disconnect |
| CC-3 | Check both env vars set after client init | test_client_sets_cargo_makeflags, test_client_sets_makeflags |
| CC-4 | Protocol encode/decode tests with known byte sequences | test_protocol_big_endian |
| CC-5 | Code review + verify no blocking in async context | test_spawn_blocking_for_jobserver |
| CC-6 | Verify client fails, doesn't create local jobserver | test_missing_socket_fails, test_no_fallback_to_uncoordinated |
| CC-7 | Multiple clients request tokens, verify FIFO grant order | test_fifo_ordering |
| CC-8 | Verify server spawn completes before cargo-make call | test_startup_sequence_order |

## Protocol Tests

| Test Name | Description |
|-----------|-------------|
| test_protocol_big_endian | Verifies multi-byte integers use network byte order (CC-4) |
| test_protocol_encode_acquire_request | Verifies AcquireRequest encoding |
| test_protocol_encode_acquire_response | Verifies AcquireResponse encoding with token |
| test_protocol_encode_release_request | Verifies ReleaseRequest encoding with token |
| test_protocol_encode_disconnect | Verifies Disconnect encoding |
| test_protocol_encode_error | Verifies Error encoding with message |
| test_protocol_decode_roundtrip | Verifies encode then decode returns original |
| test_protocol_invalid_version | Verifies error on unknown version byte |
| test_protocol_invalid_type | Verifies error on unknown message type |

## Integration Test Requirements

For throttle-client CLI binary, integration tests MUST:
- Execute the actual `throttle-client` binary
- Use real Unix domain sockets
- Verify actual environment variable changes
- NOT mock the socket layer

For server integration tests:
- Spawn real ThrottleServer on abstract socket
- Connect real clients
- Verify actual token flow

## Benchmark Requirements

Performance tests (THR-27, THR-28) require:
- Statistical measurement over multiple runs
- Warm-up period before measurement
- Report p50, p99, max latencies
- Run in isolated environment to reduce noise

## Out of Scope Tests

The following require a complete Bottlerocket build environment:

| Req ID | What Would Be Tested | Why Out of Scope |
|--------|---------------------|------------------|
| THR-9 | throttle-client binary in SDK image | Requires SDK image build |
| THR-13 | twoliter spawns server before cargo-make | Requires full twoliter + cargo-make |
| THR-14 | buildsys passes socket to docker | Requires buildsys + docker |
| THR-15 | --network host enables socket access | Requires docker container |
| THR-16 | Dockerfile initializes client | Requires full build pipeline |

These should be verified manually or via end-to-end build tests in CI.

## Test Implementation Notes

### Unit Test Location
`twoliter/tools/throttlesys/src/` - inline `#[cfg(test)]` modules

### Integration Test Location
`twoliter/tools/throttlesys/tests/` - separate test files

### Benchmark Location
`twoliter/tools/throttlesys/benches/` - criterion benchmarks

### Test Utilities Needed
- Helper to spawn ThrottleServer and return socket name
- Helper to create ThrottleClient connected to server
- Helper to simulate client crash (drop connection without Disconnect)
- Helper to measure token recovery time

### Mock Strategy
Unit tests may mock:
- `jobserver::Client` for testing server logic without real jobserver
- Time/clock for testing timeouts without waiting

Integration tests mock NOTHING.
