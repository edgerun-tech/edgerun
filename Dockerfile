# syntax=docker/dockerfile:1.7
FROM rust:slim-bookworm AS builder
WORKDIR /src

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo fetch --locked

COPY proto ./proto

# Build the node daemon
RUN cargo build --release -p edgerun-node --bin edgerund

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /src/target/release/edgerund /usr/local/bin/edgerund

EXPOSE 8080
VOLUME ["/var/lib/edgerun"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/edgerund", "--version"]

ENTRYPOINT ["/usr/local/bin/edgerund"]
CMD ["--help"]
