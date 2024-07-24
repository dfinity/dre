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

ENV HOME=/home/runner
USER runner
WORKDIR /home/runner

# Rust version should align with one in `rust-toolchain.toml` and `WORKSPACE.bazel`
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.79.0-x86_64-unknown-linux-gnu -t x86_64-apple-darwin --no-update-default-toolchain
ENV PATH="/home/runner/.cargo/bin:$PATH"

ENV PATH="$PATH:/home/runner/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/"
ENV CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=rust-lld

RUN cargo install cargo-zigbuild

RUN mkdir podman && \
    curl -o podman/podman.tar.gz -L https://github.com/containers/podman/releases/download/v5.2.0-rc2/podman-remote-static-linux_amd64.tar.gz && \
    tar -xzvf podman/podman.tar.gz -C podman && \
    sudo mv podman/bin/podman-remote-static-linux_amd64 /usr/bin/podman && \
    rm -rf podman
