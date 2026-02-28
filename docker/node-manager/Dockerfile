# SPDX-License-Identifier: Apache-2.0

FROM rust:1.86-bookworm AS builder

WORKDIR /src

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p edgerun-node-manager -p edgerun-worker

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    tpm2-tools \
    docker.io \
    tini \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/edgerun-node-manager /usr/bin/edgerun-node-manager
COPY --from=builder /src/target/release/edgerun-worker /usr/bin/edgerun-worker

RUN mkdir -p /run/edgerun /var/lib/edgerun /etc/edgerun

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["edgerun-node-manager", "run", "--local-bridge-listen", "127.0.0.1:7777"]
