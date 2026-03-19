# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.86-slim AS builder

# Build deps for Diesel's postgres and sqlite features
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libpq-dev libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency compilation by copying manifests first, building a dummy
# binary, then overlaying the real source.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
COPY migrations ./migrations
# Touch main.rs so Cargo notices the source changed
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nn2 /usr/local/bin/nn2

EXPOSE 8080

ENTRYPOINT ["nn2"]
# Bind to 0.0.0.0 so the port is reachable from outside the container.
# DATABASE_URL (or PGHOST/PGUSER/PGPASSWORD/PGDATABASE) must be supplied via
# environment variables or a .env file mounted at /app/.env.
CMD ["api-serve", "--bind", "0.0.0.0:8080"]
