# Social API

A Rust microservice for managing social interactions (likes, counts, leaderboards)
across multiple content types.

## Quick Start

```bash
docker compose up --build
curl http://localhost:8080/health/ready | jq .
```

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/likes` | Bearer | Like content |
| DELETE | `/v1/likes/{type}/{id}` | Bearer | Unlike content |
| GET | `/v1/likes/{type}/{id}/count` | — | Get like count |
| GET | `/v1/likes/{type}/{id}/status` | Bearer | Check like status |
| GET | `/v1/likes/user` | Bearer | User's likes (paginated) |
| POST | `/v1/likes/batch/counts` | — | Batch counts (max 100) |
| POST | `/v1/likes/batch/statuses` | Bearer | Batch statuses (max 100) |
| GET | `/v1/likes/top` | — | Leaderboard |
| GET | `/v1/likes/stream` | — | SSE events |
| GET | `/health/live` | — | Liveness |
| GET | `/health/ready` | — | Readiness |
| GET | `/metrics` | — | Prometheus |

## Architecture

Hexagonal architecture with trait-based ports. See [ADRs](docs/) for design decisions.

```
HTTP → Routes → Services → Repository (Postgres)
                        → Cache (Redis)
                        → Clients (External APIs)
```

## Development

```bash
cargo build                     # Build
cargo test                      # All tests
cargo clippy -- -D warnings     # Lint
cargo fmt -- --check            # Format check
```

## Test Users

| Token | User ID |
|-------|---------|
| `tok_user_1` | `550e8400-e29b-41d4-a716-446655440001` |
| `tok_user_2` | `550e8400-e29b-41d4-a716-446655440002` |
| `tok_user_3` | `550e8400-e29b-41d4-a716-446655440003` |
| `tok_user_4` | `550e8400-e29b-41d4-a716-446655440004` |
| `tok_user_5` | `550e8400-e29b-41d4-a716-446655440005` |

## License

MIT
