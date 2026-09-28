# The shop of #307: every d case carries its answer in a comment under it.
# A comment that reads like a stage declares nothing:
# FROM scratch AS runtime
ARG RUST_VERSION=1.81

FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION} AS build-env
WORKDIR /src

FROM build-env AS pricing
#    ^ d: Dockerfile:6
RUN cargo build --release --bin pricing

from build-env as warehouse
#    ^ d: Dockerfile:6
RUN cargo build --release --bin warehouse

FROM pricing AS checks
#    ^ d: Dockerfile:9
RUN --mount=type=cache,target=/cache,from=warehouse cargo test
#                                         ^ d: Dockerfile:13
RUN <<EOT
FROM alpine AS runtime
EOT

FROM debian:bookworm-slim AS runtime
COPY --from=pricing /src/target/release/pricing /usr/bin/
#           ^ d: Dockerfile:9
COPY --from=warehouse /src/target/release/warehouse /usr/bin/
#           ^ d: Dockerfile:13
COPY --from=courier /courier /usr/bin/
#           ^ d: none
COPY --from=runtime /etc/os-release /
#           ^ d: Dockerfile:25
ENV RUST_VERSION=${RUST_VERSION}
#                  ^ d: none

FROM runtime AS release
#    ^ d: Dockerfile:25
COPY --from=build-env /src/README.md /
#           ^ d: Dockerfile:6

FROM release AS smoke
#    ^ d: Dockerfile:37
COPY --from=checks /src/report.txt /
#           ^ d: Dockerfile:17
COPY --from=PRICING /usr/bin/pricing /usr/bin/
#           ^ d: Dockerfile:9

FROM --platform=linux/amd64 warehouse AS warehouse-amd64
#                           ^ d: Dockerfile:13
#                                        ^ d: Dockerfile:49
RUN --mount=type=bind,from=build-env,target=/src true
#                          ^ d: Dockerfile:6
