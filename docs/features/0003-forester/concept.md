markdown
---
feature: 0004-forester
status: proposed
---

# Forester: Forest Orchestration Tool

## Problem

The bottlerocket-forest repository enables AI agents to work effectively on Bottlerocket by creating a meta-monorepo structure. Bottlerocket is spread across many repositories, which makes it difficult for AI agents to discover and navigate the codebase. While monorepos are ideal for AI agents due to their discoverability, Bottlerocket's distributed architecture is necessary for its development model. The forest bridges this gap by organizing all component repositories in one place with documented workflows (skills) and supporting tooling.

However, working across this meta-monorepo involves many multi-step operations that agents must coordinate: building kits, publishing to registries, building variants that consume those kits, and managing test environments. Each of these workflows requires precise sequencing and state management. When agents perform these operations through ad-hoc command sequences, they risk hallucinations, missed steps, and inconsistent state.

We need to shift these common meta-repository actions into deterministic, well-tested tools. This reduces cognitive load for both humans and AI agents, minimizes errors, and creates a reliable foundation for development workflows.

## Solution

Forester will be a Rust CLI tool that provides deterministic, higher-level commands for orchestrating development workflows across the forest. It will wrap and coordinate lower-level operations (like twoliter builds and Docker commands) into cohesive workflows that are easy to invoke and hard to misuse.

The first capability will be local OCI registry management. Forester will handle starting, stopping, and managing a local Docker registry container where kits, SDKs, and host-container images can be pushed during development. This eliminates the need for external registry access (like AWS ECR) during local iteration, making development faster and safer—especially for AI agents that shouldn't have cloud credentials.

## How It Works

A developer or AI agent working on a Bottlerocket kit needs to test their changes. Instead of manually starting a Docker registry, remembering the correct port and volume names, and checking if it's healthy, they run:


forester registry start

Forester creates and starts the registry container, waits for it to become healthy, and reports the URL. If the registry is already running, the command is idempotent—it simply confirms the registry is available.

When building a kit with twoliter, they can now push it to `localhost:5000` without any external dependencies. To see what's in the registry:


forester registry list

This shows all images and tags currently stored, helping them verify their kit was published correctly.

When they're done for the day:


forester registry stop

The registry stops but preserves its data. Tomorrow they can start it again and their images are still there. If they want a clean slate:


forester registry clean

This removes the container and all stored data, ready for a fresh start.

## Benefits

Local registry management makes test iteration significantly faster. Developers can build, push, and test kits in seconds without network latency or authentication overhead. The workflow becomes: edit code, build kit, push to local registry, build variant, test. This tight feedback loop accelerates development.

For AI agents, this is especially valuable. Agents don't need AWS credentials or access to external registries, reducing security risk. The deterministic commands prevent common mistakes like forgetting to wait for registry health or using inconsistent container names. When an agent follows a skill that says "start the registry," there's one correct command with predictable behavior.

The registry's persistent storage means work isn't lost between sessions. Developers can stop the registry to free resources without losing their locally-built kits. This makes it practical to run the registry only when needed.

By implementing this in Rust, we leverage the type system and compiler to guide correct implementations. The typestate pattern in the Docker module makes invalid operations impossible at compile time—you can't stop a container that isn't running, for example. This design philosophy will extend to future Forester capabilities.

## Technical Notes

The registry uses the official `registry:2` Docker image on `localhost:5000` by default. Container and volume names are derived from the port number to support multiple registry instances if needed. The implementation uses a typestate pattern to enforce valid container state transitions at compile time.

Health checking ensures the registry is actually ready before returning from `start`, preventing race conditions in build scripts. The catalog API integration allows listing images without requiring external tools.

Future Forester capabilities will include build orchestration (coordinating kit builds, publishing, and variant builds), development status reporting (what's built, what's running), and test environment management. Each capability will follow the same principles: deterministic, agent-friendly, and composable.