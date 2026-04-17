FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.toml
COPY shared_types.rs shared_types.rs
COPY agent/Cargo.toml agent/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
COPY agent/src agent/src
COPY server/src server/src

RUN cargo build --release -p activity-monitor-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/activity-monitor-server /usr/local/bin/activity-monitor-server

EXPOSE 3000

CMD ["activity-monitor-server"]
