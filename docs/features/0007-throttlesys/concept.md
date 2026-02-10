---
feature: 0007-throttlesys
status: proposed
---

# throttlesys

## Problem

Bottlerocket builds suffer from uncoordinated parallelism. Today, `BUILDSYS_JOBS=8` tells cargo to run up to 8 build scripts concurrently, and each build script invokes buildsys which spawns a `docker build` command. But that's where coordination ends.

Inside each Docker container, cargo runs again, spawning rustc and additional build scripts. Each container's cargo creates its own jobserver, unaware of the others. The result is a pyramid of parallelism: 8 containers × N jobs per container = potential for massive oversubscription of CPU and memory.

The symptoms are familiar: builds that thrash, OOM kills on memory-constrained machines, and a collection of retry hacks in buildsys to work around BuildKit bugs that surface under parallel load. The code comments tell the story: "we can't do much to control the concurrency here" and "BuildKit has a bug that causes failures during parallel docker build executions."

The fundamental issue is that the Docker boundary severs the jobserver protocol. GNU make and cargo have solved cross-process parallelism coordination for decades using jobservers—a simple token-passing scheme where processes acquire a token before doing work and release it when done. But this elegant solution stops at the container wall.

## Solution

We introduce **throttlesys**, a new tool in the twoliter ecosystem that coordinates parallelism across Docker container boundaries. Like pipesys passes file descriptors into containers, throttlesys passes jobserver tokens.

When twoliter starts a build, it creates a throttle server with N tokens (matching the user's desired parallelism). The server listens on an abstract Unix domain socket. Each buildsys invocation, before running `docker build`, passes the socket name as a build argument. Inside the container, a throttle client connects to that socket and presents itself as a normal jobserver to cargo and make.

From cargo's perspective inside the container, nothing changes—it inherits a jobserver via `CARGO_MAKEFLAGS` and uses it normally. But instead of tokens sitting in a local pipe, each acquire and release crosses the container boundary to the central pool. Eight containers sharing 8 tokens means at most 8 concurrent jobs across the entire build, not 8 × 8.

## How It Works

A developer runs `twoliter build variant` on a machine with 8 cores. Before invoking cargo-make, twoliter spawns a throttle server on an abstract socket with 8 tokens.

As cargo-make orchestrates the build, it runs `cargo build` with `--jobs 8`. Cargo discovers package dependencies and invokes buildsys for each package. Each buildsys process receives `THROTTLE_SOCKET` from the environment, passes it to `docker build` as a build argument, and proceeds.

Early in the Dockerfile, before any compilation, the throttle client connects to that socket and sets up `CARGO_MAKEFLAGS` pointing to a local pipe that it manages. When cargo inside the container wants to compile a crate, it reads from its local jobserver pipe. The throttle client intercepts this, sends a request over the Unix socket to the host, and blocks until the central server grants a token. When rustc finishes and cargo releases the token, it flows back through the server to the central pool.

If a container crashes mid-build—perhaps an OOM kill—the server notices the socket disconnection. It knows exactly which tokens that container held and returns them to the pool. The build continues, slightly slower but not deadlocked.

The developer sees what they expect: their 8-core machine runs 8 compilations at a time, regardless of how many containers are active. Memory pressure stays predictable. The BuildKit retry hacks become unnecessary because we're no longer hammering the system with uncoordinated parallel builds.

## Benefits

True parallelism control replaces the current approximation. Instead of "roughly 8 docker builds, each doing whatever it wants," we get "exactly N concurrent compilation jobs, period." This makes builds predictable and reproducible across different machines.

Resource usage becomes bounded. A laptop with 4 cores and 16GB RAM can build Bottlerocket by setting `BUILDSYS_JOBS=2`, confident that only 2 compilations run at once. Today, that same setting still allows runaway parallelism inside containers.

The retry hacks in buildsys can eventually be removed. The BuildKit bugs we work around are triggered by parallel pressure—pressure that proper throttling eliminates. We're treating the cause, not the symptoms.

Future optimizations become possible. With centralized job tracking, we could implement priority scheduling (kernel packages before userspace), better progress reporting, or even distributed builds across multiple machines sharing a throttle server.

## Technical Notes

throttlesys follows the pattern established by pipesys and buildsys—a focused tool in `twoliter/tools/` that handles one concern well.

The throttle server wraps the `jobserver` crate (maintained by rust-lang), which handles the GNU make jobserver protocol. The crate is blocking-only, so the server uses tokio's `spawn_blocking` to integrate with the async runtime.

The integration follows the OUTPUT_SOCKET pattern from pipesys: twoliter spawns the throttle server as a tokio task, passes the socket name via environment to buildsys, which passes it as a build argument to docker. The throttle client binary lives in the SDK image.

Token recovery on client disconnect is critical. The GNU make jobserver protocol has no built-in recovery—crashed processes lose their tokens forever. throttlesys tracks token ownership per connection and reclaims tokens when connections drop.
