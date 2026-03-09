# Stage 1: Build
FROM rust:1.93-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY social-api/Cargo.toml social-api/
COPY mock-services/Cargo.toml mock-services/

RUN mkdir -p social-api/src mock-services/src && \
    echo "fn main() {}" > social-api/src/main.rs && \
    echo "fn main() {}" > mock-services/src/main.rs

RUN cargo build --release -p social-api 2>/dev/null || true

COPY social-api/src social-api/src
COPY social-api/migrations social-api/migrations

RUN touch social-api/src/main.rs

RUN cargo build --release -p social-api

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false appuser

WORKDIR /app

COPY --from=builder /app/target/release/social-api .

RUN chown appuser:appuser /app/social-api

USER appuser

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:8080/health/live || exit 1

ENTRYPOINT ["./social-api"]
