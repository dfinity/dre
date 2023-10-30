FROM docker.io/rust:1.66.1-bullseye AS builder
RUN apt-get -y update && \
    apt-get install -y openssl ca-certificates curl clang

ARG mold_version=1.11.0
RUN curl -fsSL https://github.com/rui314/mold/releases/download/v${mold_version}/mold-${mold_version}-x86_64-linux.tar.gz | tar -xz -C /usr/local --strip-components 1

ARG protobuf_version=3.18.1
RUN cd `mktemp -d`; curl --fail -LO https://github.com/protocolbuffers/protobuf/releases/download/v${protobuf_version}/protoc-${protobuf_version}-linux-x86_64.zip; unzip * -d /opt/protoc; chmod -R 755 /opt/protoc/bin; chmod -R u+rwX,go+rX,go-w /opt/protoc/include

ENV PROTOC=/opt/protoc/bin/protoc \
    PROTOC_INCLUDE=/opt/protoc/include

WORKDIR /app
COPY . .
RUN cargo build --release --bin slack-notifications

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye AS runtime
RUN apt-get -y update && \
    apt-get install -y openssl ca-certificates && \
    apt-get clean autoclean && \
    apt-get autoremove --yes && \
    rm -rf /var/lib/{apt,dpkg,cache,log}/
WORKDIR app
COPY --from=builder /app/target/release/slack-notifications /usr/local/bin

ENTRYPOINT ["/usr/local/bin/slack-notifications"]
