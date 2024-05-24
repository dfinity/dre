FROM ghcr.io/actions/actions-runner:2.316.1

LABEL org.opencontainers.image.source="https://github.com/dfinity/dre"

RUN sudo apt-get update && \
     sudo apt-get upgrade -y && \
     sudo apt-get install ca-certificates curl git-all gcc g++ clang pkg-config make -y

RUN mkdir -p /home/runner/.cache/bazel openssl
RUN curl -o openssl/openssl-1.1.1w.tar.gz -L https://www.openssl.org/source/old/1.1.1/openssl-1.1.1w.tar.gz && \
    tar -xzvf openssl/openssl-1.1.1w.tar.gz -C openssl && \
    cd openssl/openssl-1.1.1w && \
    ./config && \
    make && \
    sudo make install
RUN sudo ln -s /usr/local/lib/libssl.so.1.1 /usr/lib64/libssl.so.1.1 && \
    sudo ln -s /usr/local/lib/libssl.so.1.1 /usr/lib/libssl.so.1.1 && \
    sudo ln -s /usr/local/lib/libcrypto.so.1.1 /usr/lib64/libcrypto.so.1.1 && \
    sudo ln -s /usr/local/lib/libcrypto.so.1.1 /usr/lib/libcrypto.so.1.1 && \
    rm -rf /home/runner/openssl