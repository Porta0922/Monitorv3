# Build packages only: no service installation and no test suite.
ARG BUILD_IMAGE=rust:1.99.1-slim-bookworm@sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be
FROM rust:1.99.1-slim-bookworm@sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be AS toolchain
FROM ${BUILD_IMAGE} AS build
COPY --from=toolchain /usr/local/rustup /usr/local/rustup
COPY --from=toolchain /usr/local/cargo/bin /usr/local/cargo/bin
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 dpkg-dev build-essential ca-certificates pkg-config libssl-dev \
    libx11-dev libxi-dev libxtst-dev libxcb1-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml shared_types.rs ./
COPY agent/Cargo.toml agent/Cargo.toml
COPY agent/src agent/src
COPY agent/ui agent/ui
COPY server/Cargo.toml server/Cargo.toml
COPY server/src server/src
COPY server/build.rs server/build.rs
COPY server/migrations server/migrations
COPY scripts/production/grants.sql scripts/production/grants.sql
COPY tools/agent-transport-probe tools/agent-transport-probe
COPY packaging/linux packaging/linux
COPY packaging/common packaging/common
COPY packaging/agent.env.example packaging/agent.env.example
ARG BUILD_CACHE_ID=monitor-agent-debian12
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${BUILD_CACHE_ID},target=/src/target \
    cargo build --locked --release -p activity-monitor-agent && \
    version=$(cargo metadata --offline --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "activity-monitor-agent"))') && \
    arch=$(dpkg --print-architecture) && \
    hash=$(sha256sum target/release/activity-monitor-agent | cut -d ' ' -f 1) && \
    mkdir -p /out && \
    python3 packaging/linux/build-deb.py --binary target/release/activity-monitor-agent \
      --sha256 "$hash" --version "$version" --architecture "$arch" \
      --output "/out/monitor-agent-${version}-${arch}.deb" && \
    cp /etc/os-release /out/build-os-release.txt && \
    rustc -Vv > /out/rustc.txt && \
    cd /out && sha256sum *.deb > SHA256SUMS
FROM scratch AS artifact
COPY --from=build /out/ /
