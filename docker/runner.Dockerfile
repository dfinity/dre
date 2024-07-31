FROM ubuntu:20.04

LABEL org.opencontainers.image.source="https://github.com/dfinity/dre"

ENV TZ=UTC
ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_UID=1001

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install ca-certificates curl git-all gcc g++ clang pkg-config make sudo docker.io build-essential \
    libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev libffi-dev liblzma-dev libncurses5-dev libncursesw5-dev \
    xz-utils tk-dev libffi-dev liblzma-dev python-openssl protobuf-compiler -y

RUN curl -L https://ziglang.org/builds/zig-linux-x86_64-0.14.0-dev.321+888708ec8.tar.xz | tar -xJ && \
    mv zig-linux-x86_64-0.14.0-dev.321+888708ec8 /zig
ENV PATH="$PATH:/zig"

RUN curl -L https://github.com/roblabla/MacOSX-SDKs/releases/download/13.3/MacOSX13.3.sdk.tar.xz | tar xJ
ENV SDKROOT=/MacOSX13.3.sdk/

RUN mkdir -p openssl && \
    curl -o openssl/openssl-1.1.1w.tar.gz -L https://www.openssl.org/source/old/1.1.1/openssl-1.1.1w.tar.gz && \
    tar -xzvf openssl/openssl-1.1.1w.tar.gz -C openssl && \
    cd openssl/openssl-1.1.1w && \
    ./config && \
    make && \
    make install
RUN ln -s /usr/local/lib/libssl.so.1.1 /usr/lib64/libssl.so.1.1 && \
    ln -s /usr/local/lib/libssl.so.1.1 /usr/lib/libssl.so.1.1 && \
    ln -s /usr/local/lib/libcrypto.so.1.1 /usr/lib64/libcrypto.so.1.1 && \
    ln -s /usr/local/lib/libcrypto.so.1.1 /usr/lib/libcrypto.so.1.1 && \
    rm -rf openssl

RUN mkdir python3.12 && \
    curl -o python3.12/Python-3.12.0.tar.xz -L https://www.python.org/ftp/python/3.12.0/Python-3.12.0.tgz && \
    tar -xzvf python3.12/Python-3.12.0.tar.xz -C python3.12 && \
    cd python3.12/Python-3.12.0 && \
    ./configure --enable-optimizations && \
    make -j 8 && \
    make altinstall && \
    ln -s python3.12 /usr/local/bin/python && \
    ln -s pip3.12 /usr/local/bin/pip

COPY requirements.txt .
RUN pip install -r requirements.txt

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

# Rust version should align with one in `rust-toolchain.toml` and `WORKSPACE.bazel`
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.79.0-x86_64-unknown-linux-gnu -t x86_64-apple-darwin --no-update-default-toolchain
ENV PATH="/home/runner/.cargo/bin:$PATH"

ENV PATH="$PATH:/home/runner/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/"
ENV CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=rust-lld

RUN cargo install cargo-zigbuild
