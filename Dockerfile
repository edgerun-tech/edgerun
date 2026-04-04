# syntax=docker/dockerfile:1.7
FROM golang:1.26 AS builder
WORKDIR /src

COPY go/go.mod go/go.sum ./go/
WORKDIR /src/go
RUN go mod download

WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/root/.cache/go-build \
    --mount=type=cache,target=/go/pkg/mod \
    cd go && CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags='-s -w' -o /out/lifegraphd ./cmd/lifegraphd

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /out/lifegraphd /usr/local/bin/lifegraphd

EXPOSE 8080
VOLUME ["/var/lib/lifegraph"]

ENTRYPOINT ["/usr/local/bin/lifegraphd"]
CMD ["--help"]
