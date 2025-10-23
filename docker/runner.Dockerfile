FROM ubuntu:20.04

LABEL org.opencontainers.image.source="https://github.com/dfinity/dre"

ENV TZ=UTC
ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_UID=1001

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install ca-certificates curl git-all gcc g++ clang pkg-config make sudo docker.io build-essential \
    libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev libffi-dev liblzma-dev libncurses5-dev libncursesw5-dev \
    xz-utils tk-dev libffi-dev liblzma-dev python-openssl protobuf-compiler libdbus-1-dev softhsm2 libsofthsm2 opensc -y

RUN curl -L https://ziglang.org/download/0.15.1/zig-x86_64-linux-0.15.1.tar.xz | tar -xJ && \
    mv zig-x86_64-linux-0.15.1 /zig
ENV PATH="$PATH:/zig"

RUN curl -L https://github.com/roblabla/MacOSX-SDKs/releases/download/macosx14.5/MacOSX14.5.sdk.tar.xz | tar xJ
ENV SDKROOT=/MacOSX14.5.sdk/

ENV RYE_HOME="/opt/rye"
ENV PATH="$RYE_HOME/shims:$PATH"

RUN curl -sSf https://rye.astral.sh/get | RYE_NO_AUTO_INSTALL=1 RYE_INSTALL_OPTION="--yes" bash

COPY pyproject.toml requirements.lock requirements-dev.lock .python-version README.md ./

RUN rye sync --no-dev --no-lock

# Runner user
RUN adduser --disabled-password --gecos "" --uid $RUNNER_UID runner \
    && usermod -aG sudo runner \
    && usermod -aG docker runner \
    && echo "%sudo   ALL=(ALL:ALL) NOPASSWD:ALL" > /etc/sudoers \
    && echo "Defaults env_keep += \"DEBIAN_FRONTEND\"" >> /etc/sudoers

# GitHub ssh keys
RUN mkdir -p /home/runner/.ssh \
    && chmod 700 /home/runner/.ssh \
    && ssh-keyscan github.com >> /home/runner/.ssh/known_hosts

# Adjust permissions
RUN chown -R runner:runner /home/runner
RUN chown -R runner:runner /etc/softhsm
RUN chown -R runner:runner /var/lib/softhsm

# Setup podman
# From https://www.redhat.com/sysadmin/podman-inside-container
RUN echo 'deb http://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/xUbuntu_20.04/ /' > /etc/apt/sources.list.d/devel:kubic:libcontainers:stable.list && \
    curl -o key https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/xUbuntu_20.04/Release.key && \
    APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE="1" apt-key add key && \
    apt-get update -qq && \
    apt-get install -y software-properties-common uidmap fuse-overlayfs podman

RUN useradd podman; \
    echo podman:10000:5000 > /etc/subuid; \
    echo podman:10000:5000 > /etc/subgid;

VOLUME /var/lib/containers
VOLUME /home/podman/.local/share/containers

ADD https://raw.githubusercontent.com/containers/image_build/147ee0bfd7736f3f2f11d59c7d08b4dd9273e01e/podman/containers.conf /etc/containers/containers.conf
ADD https://raw.githubusercontent.com/containers/image_build/147ee0bfd7736f3f2f11d59c7d08b4dd9273e01e/podman/podman-containers.conf /home/podman/.config/containers/containers.conf

RUN chown podman:podman -R /home/podman

RUN chmod 644 /etc/containers/containers.conf; sed -i -e 's|^#mount_program|mount_program|g' -e '/additionalimage.*/a "/var/lib/shared",' -e 's|^mountopt[[:space:]]*=.*$|mountopt = "nodev,fsync=0"|g' /etc/containers/storage.conf
RUN mkdir -p /var/lib/shared/overlay-images /var/lib/shared/overlay-layers /var/lib/shared/vfs-images /var/lib/shared/vfs-layers; touch /var/lib/shared/overlay-images/images.lock; touch /var/lib/shared/overlay-layers/layers.lock; touch /var/lib/shared/vfs-images/images.lock; touch /var/lib/shared/vfs-layers/layers.lock

ENV _CONTAINERS_USERNS_CONFIGURED=""

ENV HOME=/home/runner
USER runner
WORKDIR /home/runner
SHELL [ "/bin/bash", "-c" ]

RUN source /opt/rye/env

COPY rust-toolchain.toml /usr/src/rust-toolchain.toml

# Rust version should align with one in `rust-toolchain.toml` and `WORKSPACE.bazel`
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain $(grep -oP '(?<=channel = ")[^"]+' /usr/src/rust-toolchain.toml)-x86_64-unknown-linux-gnu -t x86_64-apple-darwin --no-update-default-toolchain
ENV PATH="/home/runner/.cargo/bin:$PATH"

ENV PATH="$PATH:/home/runner/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/"
ENV CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=rust-lld

RUN cargo install --quiet cargo-zigbuild && rustup toolchain install 1.90.0 && cargo +1.90.0 install git-cliff
