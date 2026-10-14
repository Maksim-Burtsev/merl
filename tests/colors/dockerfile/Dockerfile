# syntax=docker/dockerfile:1.7
ARG NODE_VERSION=20.11
ARG BASE_IMAGE=docker.io/library/node:${NODE_VERSION}-bookworm-slim

FROM ${BASE_IMAGE} AS base
FROM python:3.12-slim@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef AS py

LABEL org.opencontainers.image.source="https://example.com/ledger" \
      maintainer="ops@example.com"

ENV LEDGER_HOME=/opt/ledger \
    PATH="/opt/ledger/bin:$PATH" \
    lower_case=yes

WORKDIR $LEDGER_HOME
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
 && apt-get install -y --no-install-recommends curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    if [ "${NODE_VERSION%%.*}" -ge 20 ]; then \
      echo "modern node" > /tmp/node.txt; \
    fi; \
    for f in a b c; do touch "/tmp/$f"; done

RUN <<EOT bash
  set -e
  echo "building $LEDGER_HOME"
  mkdir -p "$LEDGER_HOME/data"
EOT

COPY --from=py --chown=1000:1000 /usr/local/lib/python3.12 /usr/local/lib/python3.12
COPY package.json yarn.lock ./
ADD --checksum=sha256:24454f830cdb571e2c4ad15481119c43b3cafd48dd869a9b2945d1036d1dc68d https://example.com/a.tgz /tmp/
ONBUILD RUN yarn install --frozen-lockfile

EXPOSE 8080 9090/udp
VOLUME ["/data", "/logs"]
USER 1000:1000
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["node", "server.js", "--port=${PORT}"]
