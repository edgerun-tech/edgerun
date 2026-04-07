# syntax=docker/dockerfile:1.7
FROM rust:1.85 AS builder
WORKDIR /src

# Cache dependencies first
COPY rust/Cargo.toml rust/Cargo.lock ./rust/
COPY rust/crates ./rust/crates
WORKDIR /src/rust
RUN cargo fetch --locked

WORKDIR /src
COPY proto ./proto
COPY rust ./rust

# Build the node daemon
RUN cd rust && cargo build --release -p edgerun-node --bin edgerund

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /src/rust/target/release/edgerund /usr/local/bin/edgerund

EXPOSE 8080
VOLUME ["/var/lib/edgerun"]

ENTRYPOINT ["/usr/local/bin/edgerund"]
CMD ["--help"]
