# Cloud Engine Controller Backend

A backend service for managing Google Cloud Platform VMs and their association with Internet Computer nodes.

## Features

- **Internet Identity Authentication**: Validate IC delegations from frontend applications
- **GCP VM Management**: List, provision, and delete VMs in Google Cloud
- **ICP Node Mapping**: Map GCP VMs to ICP nodes using local registry sync
- **Subnet Management**: Create and delete subnets via NNS proposals

## API Documentation

The service exposes a Swagger UI at `/swagger-ui/` for interactive API documentation and testing.

## Running

```bash
cargo run --release -- \
    --targets-dir /path/to/registry/store \
    --gcp-credentials-file /path/to/service-account.json
```

## Configuration

| Flag | Description | Default |
|------|-------------|---------|
| `--targets-dir` | Directory for registry local store | Required |
| `--port` | Server port | 8000 |
| `--gcp-credentials-file` | Path to GCP service account JSON | Optional (uses ADC) |
| `--poll-interval` | Registry sync interval | 30s |
| `--nns-url` | NNS URL for registry sync | https://ic0.app |

## Architecture

```
Frontend (with II) --> Backend API --> GCP Compute API
                                   --> ICP Registry (NNS)
                                   --> NNS Governance (Proposals)
```

## Development

```bash
# Run tests
cargo test

# Run with hot reload
cargo watch -x run
```
