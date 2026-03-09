# Social API

A Rust microservice for managing social interactions (likes, counts, leaderboards)
across multiple content types.

## Quick Start

```bash
docker compose up --build

curl http://localhost:8080/health/live
```

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

## Architecture

Hexagonal architecture with Axum, PostgreSQL, and Redis.
