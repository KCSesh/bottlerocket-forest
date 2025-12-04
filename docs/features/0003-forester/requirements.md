# Feature 0004: Local OCI Registry Management - Requirements Specification

## Overview

This specification defines the requirements for managing a local Docker-based OCI registry container used to store and serve Bottlerocket kits and SDKs during development. The registry enables development workflows without requiring external registry access.

## Functional Requirements

### FR-1: Registry Container Creation

WHEN the user executes forester registry start  
WHERE no registry container exists  
THEN the system SHALL create a Docker container with the following properties:
- Image: registry:2
- Container name: forester-registry-{port} where port defaults to 5000
- Port binding: localhost:{port}:5000
- Volume mount: forester-registry-data-{port}:/var/lib/registry

### FR-2: Idempotent Registry Start

WHEN the user executes forester registry start  
WHERE the registry container already exists and is running  
THEN the system SHALL return successfully without creating a new container

WHERE the registry container exists but is stopped  
THEN the system SHALL start the existing container

### FR-3: Registry Health Verification

WHEN the user executes forester registry start  
THEN the system SHALL wait for the registry to respond to HTTP health checks before returning

WHERE the registry does not become healthy within 10 seconds  
THEN the system SHALL fail with a timeout error

### FR-4: Registry Start Output

WHEN forester registry start completes successfully  
THEN the system SHALL display:
- A success indicator
- The registry URL in the format http://localhost:{port}

### FR-5: Registry Stop

WHEN the user executes forester registry stop  
WHERE the registry container is running  
THEN the system SHALL stop the container

WHERE the registry container is already stopped or does not exist  
THEN the system SHALL return successfully without error

### FR-6: Data Volume Preservation

WHEN the user executes forester registry stop  
THEN the system SHALL preserve the data volume containing registry contents

### FR-7: Registry Status Check

WHEN the user executes forester registry status  
THEN the system SHALL display:
- Container state (not created, stopped, or running)
- Registry URL if running
- Data volume existence status

### FR-8: Registry Status Exit Code

WHEN the user executes forester registry status  
WHERE the registry is running  
THEN the system SHALL exit with code 0

WHERE the registry is not running  
THEN the system SHALL exit with a non-zero code

### FR-9: Registry Image Listing

WHEN the user executes forester registry list  
WHERE the registry is running  
THEN the system SHALL display all images stored in the registry with their tags

WHERE the registry is not running  
THEN the system SHALL fail with an appropriate error

### FR-10: Registry Log Display

WHEN the user executes forester registry logs  
WHERE the registry container is running  
THEN the system SHALL display the container logs

WHERE the --follow flag is provided  
THEN the system SHALL stream logs continuously until interrupted

WHERE the registry is not running  
THEN the system SHALL fail with an error indicating the container is not running

### FR-11: Registry Data Cleanup

WHEN the user executes forester registry clean  
THEN the system SHALL:
1. Stop the registry container if running
2. Remove the registry container if it exists
3. Remove the data volume if it exists

### FR-12: Multiple Registry Instances

WHEN the user configures a non-default port  
THEN the system SHALL derive unique container and volume names from the port number to allow multiple registry instances

## Non-Functional Requirements

### FR-NFR-1: Docker Dependency

WHILE executing any registry command  
THEN the system SHALL require Docker to be installed and accessible

### FR-NFR-2: Type Safety

WHILE managing container state transitions  
THEN the system SHALL use compile-time type checking to prevent invalid operations

### FR-NFR-3: Port Validation

WHILE accepting port configuration  
THEN the system SHALL enforce a minimum port number of 1024 to avoid privileged ports

### FR-NFR-4: Configuration Defaults

WHILE no explicit configuration is provided  
THEN the system SHALL use the following defaults:
- Port: 5000
- Image: registry:2
- Container name: forester-registry-5000
- Volume name: forester-registry-data-5000

## Error Handling

### FR-ERR-1: Docker Unavailable

WHILE executing any registry command  
WHERE Docker is not running or not accessible  
THEN the system SHALL fail with a clear error message indicating Docker is required

### FR-ERR-2: Port Conflict

WHILE starting the registry  
WHERE the configured port is already in use  
THEN the system SHALL fail with an error indicating the port conflict

### FR-ERR-3: Health Check Timeout

WHILE starting the registry  
WHERE the registry does not become healthy within the timeout period  
THEN the system SHALL fail with an error indicating the health check timeout

### FR-ERR-4: Invalid Configuration

WHILE loading registry configuration  
WHERE configuration values are invalid (e.g., empty strings, port < 1024)  
THEN the system SHALL fail with a validation error before attempting operations

## Appendix A: Registry API Endpoints

The registry container exposes the Docker Registry HTTP API V2:

GET  /v2/                          - API version check (health)
GET  /v2/_catalog                  - List repositories
GET  /v2/{name}/tags/list          - List tags for a repository
GET  /v2/{name}/manifests/{ref}    - Get manifest
PUT  /v2/{name}/manifests/{ref}    - Push manifest


## Appendix B: Container State Transitions

NotCreated --[create]--> Running
Stopped    --[start]---> Running
Running    --[stop]----> Stopped
Stopped    --[remove]--> NotCreated
Running    --[remove]--> (stop first, then remove)


## Notes

- The typestate pattern enforces valid state transitions at compile time
- Container and volume names include the port number to support multiple instances
- The registry uses the official Docker registry:2 image
- Data persistence is handled through Docker named volumes
- All operations are designed to be idempotent where appropriate