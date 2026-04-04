GOCACHE ?= $(CURDIR)/var/go-build-cache

.PHONY: test test-go interop build-go docker-build coverage-go release-build

test: test-go interop

test-go:
	cd go && GOCACHE=$(GOCACHE) go test ./...

interop:
	GOCACHE=$(GOCACHE) ./scripts/check_interop.sh

build-go:
	mkdir -p var/bin
	cd go && GOCACHE=$(GOCACHE) go build -o ../var/bin/lifegraphd ./cmd/lifegraphd

docker-build:
	docker build -t lifegraph-reference-core:local .

coverage-go:
	mkdir -p var/coverage
	cd go && GOCACHE=$(GOCACHE) go test ./... -coverprofile=../var/coverage/go.cover
	cd go && GOCACHE=$(GOCACHE) go tool cover -func=../var/coverage/go.cover

release-build:
	mkdir -p dist
	cd go && GOCACHE=$(GOCACHE) CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags='-s -w' -o ../dist/lifegraphd-linux-amd64 ./cmd/lifegraphd
	cd go && GOCACHE=$(GOCACHE) CGO_ENABLED=0 GOOS=linux GOARCH=arm64 go build -trimpath -ldflags='-s -w' -o ../dist/lifegraphd-linux-arm64 ./cmd/lifegraphd
