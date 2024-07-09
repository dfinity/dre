FROM ubuntu:20.04

LABEL org.opencontainers.image.source="https://github.com/dfinity/dre"

ENV TZ=UTC
ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_UID=1001

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install ca-certificates curl git-all gcc g++ clang pkg-config make sudo docker.io build-essential \
    libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev libffi-dev liblzma-dev libncurses5-dev libncursesw5-dev \
    xz-utils tk-dev libffi-dev liblzma-dev python-openssl -y

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
RUN mkdir -p /home/runner/.ssh && \
    ssh-keyscan github.com >> /home/runner/.ssh/known_hosts

# Adjust permissions
RUN chown -R runner:runner /home/runner

ENV HOME=/home/runner
USER runner
WORKDIR /home/runner
