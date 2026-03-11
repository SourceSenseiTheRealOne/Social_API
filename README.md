# Social API

A Rust microservice for managing social interactions (likes, counts, leaderboards)
across multiple content types.

## Quick Start

```bash
docker compose up --build

curl http://localhost:8080/health/live
```

## API Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health/live` | GET | Liveness probe |
| `/health/ready` | GET | Readiness probe |
| `/metrics` | GET | Prometheus metrics |
| `/likes/{content_type}/{content_id}` | POST | Toggle like (idempotent) |
| `/likes/{content_type}/{content_id}` | GET | Get like status |
| `/likes/{content_type}/{content_id}/count` | GET | Get like count |
| `/likes/batch/counts` | POST | Batch get counts |
| `/likes/leaderboard` | GET | Top liked content |

## Architecture

```
                 ┌─────────────────┐
 HTTP ──────────►│   Route Layer   │
 (Axum)          │  (thin adapters) │
                 └────────┬────────┘
                          │
                 ┌────────▼────────┐
                 │  Service Layer  │
                 │ (business logic) │
                 │  depends only   │
                 │  on traits      │
                 └────────┬────────┘
                          │
             ┌────────────┼────────────┐
             │            │            │
    ┌────────▼───┐ ┌──────▼─────┐ ┌───▼────────┐
    │ Repository │ │   Cache    │ │  Clients   │
    │ (Postgres) │ │  (Redis)   │ │  (HTTP)    │
    └────────────┘ └────────────┘ └────────────┘
```

Hexagonal architecture with trait-based ports and adapters.

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

## Architecture Decision Records

- [ADR-001: Hexagonal Architecture](docs/ADR-001-hexagonal-architecture.md)
- [ADR-002: Like Counts Strategy](docs/ADR-002-like-counts-strategy.md)
- [ADR-003: Cache Stampede Protection](docs/ADR-003-cache-stampede-protection.md)
- [ADR-004: Circuit Breaker Design](docs/ADR-004-circuit-breaker-design.md)
- [Cache Staleness Window](docs/STALENESS.md)
