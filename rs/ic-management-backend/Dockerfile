# Leveraging the pre-built Docker images with
# cargo-chef and the Rust toolchain
# https://www.lpalmieri.com/posts/fast-rust-docker-builds/
FROM lukemathwalker/cargo-chef:latest-rust-1.65.0-bullseye AS chef
WORKDIR app

ARG protobuf_version=3.18.1

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get -y update && \
    apt-get install -y cmake clang && \
    apt-get clean autoclean && \
    apt-get autoremove --yes && \
    rm -rf /var/lib/{apt,dpkg,cache,log}/

RUN cd `mktemp -d`; curl --fail -LO https://github.com/protocolbuffers/protobuf/releases/download/v${protobuf_version}/protoc-${protobuf_version}-linux-x86_64.zip; unzip * -d /opt/protoc; chmod -R 755 /opt/protoc/bin; chmod -R u+rwX,go+rX,go-w /opt/protoc/include

ENV PROTOC=/opt/protoc/bin/protoc \
    PROTOC_INCLUDE=/opt/protoc/include

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin ic-management-backend

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye AS runtime
RUN apt-get -y update && \
    apt-get install -y openssl ca-certificates curl && \
    apt-get clean autoclean && \
    apt-get autoremove --yes && \
    rm -rf /var/lib/{apt,dpkg,cache,log}/
WORKDIR app
COPY --from=builder /app/target/release/ic-management-backend /usr/local/bin

ENTRYPOINT ["/usr/local/bin/ic-management-backend"]
