# Leveraging the pre-built Docker images with
# cargo-chef and the Rust toolchain
# https://www.lpalmieri.com/posts/fast-rust-docker-builds/
FROM lukemathwalker/cargo-chef:latest-rust-1.60.0-bullseye AS chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin mercury-management-backend

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye AS runtime
RUN apt-get -y update && \
    apt-get install -y openssl ca-certificates && \
    apt-get clean autoclean && \
    apt-get autoremove --yes && \
    rm -rf /var/lib/{apt,dpkg,cache,log}/
WORKDIR app
COPY --from=builder /app/target/release/mercury-management-backend /usr/local/bin

ENTRYPOINT ["/usr/local/bin/mercury-management-backend"]
