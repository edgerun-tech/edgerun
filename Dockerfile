# syntax=docker/dockerfile:1.7
FROM rust:slim-bookworm AS builder
WORKDIR /src

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo fetch --locked

# Build the node daemon
RUN cargo build --locked --release -p edgerun-node --bin edged

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /src/target/release/edged /usr/local/bin/edged

EXPOSE 8080
VOLUME ["/var/lib/edgerun"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/edged", "--version"]

ENTRYPOINT ["/usr/local/bin/edged"]
CMD ["--help"]
