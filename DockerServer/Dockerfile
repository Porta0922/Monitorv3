FROM rust:1.99.1-slim-bookworm@sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY shared_types.rs shared_types.rs
COPY agent/Cargo.toml agent/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
COPY agent/src agent/src
COPY server/src server/src
COPY server/migrations server/migrations
COPY server/build.rs server/build.rs
COPY scripts/production/grants.sql scripts/production/grants.sql
COPY tools/agent-transport-probe tools/agent-transport-probe

RUN cargo build --locked --release -p activity-monitor-server

FROM builder AS validation
RUN CARGO_BUILD_JOBS=1 cargo build --locked --release -p agent-transport-probe
RUN CARGO_BUILD_JOBS=1 cargo test --locked --release -p activity-monitor-server

FROM debian:bookworm-slim@sha256:7ff6ba31527b9e336162fa061746c366c4b2032f4d2cff2078bea43f78efe3f8 AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/activity-monitor-server /usr/local/bin/activity-monitor-server

EXPOSE 3000

CMD ["activity-monitor-server"]

# Production selects this target explicitly; development retains the legacy runtime.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2 AS production
WORKDIR /app
COPY --from=builder /app/target/release/activity-monitor-server /usr/local/bin/activity-monitor-server
USER 65532:65532
EXPOSE 3000
CMD ["/usr/local/bin/activity-monitor-server"]
FROM runtime AS default
