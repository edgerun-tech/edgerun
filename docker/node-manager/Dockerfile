# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:24.04 AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    tpm2-tools \
    docker.io \
    tini \
  && rm -rf /var/lib/apt/lists/*

COPY docker/node-manager/bin/edgerun-node-manager /usr/bin/edgerun-node-manager
COPY docker/node-manager/bin/edgerun-worker /usr/bin/edgerun-worker

RUN mkdir -p /run/edgerun /var/lib/edgerun /etc/edgerun

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["edgerun-node-manager", "run", "--local-bridge-listen", "127.0.0.1:7777"]
