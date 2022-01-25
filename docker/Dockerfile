FROM ubuntu:20.04

ARG rust_version=nightly
ARG ic_git_revision=37201f2d611394581a30de9ea78cc742b3672ea4
ARG protobuf_version=3.18.1

# The image expects the following volumes mounted from the host:
# /cache - holding the sccache artifacts
# /cargo - is the CARGO_HOME, holding cargo binaries and cloned repos
# /builds/git - a read only volume with the git repository which should be built

ENV TZ=UTC

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt -yq update && \
    apt -yqq install --no-install-recommends \
        git openssh-client curl xz-utils time apt-transport-https ca-certificates lsb-release gnupg2 && \
        curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add - && \
        echo "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null && \
    apt -yq update && \
    apt -yqq install --no-install-recommends \
        build-essential pkg-config libssl-dev cmake libbz2-dev gcc-multilib rsync \
        lldb liblz4-dev librocksdb-dev libsnappy-dev libzstd-dev vim nano jq \
        zip unzip python3 python3-requests python3-pip python3-cbor shellcheck \
        liblmdb0 libsqlite3-dev sqlite3 \
        docker-ce-cli iputils-ping sudo gosu && \
    chmod +s /usr/sbin/gosu

RUN cd `mktemp -d`; curl --fail -LO https://github.com/protocolbuffers/protobuf/releases/download/v${protobuf_version}/protoc-${protobuf_version}-linux-x86_64.zip; unzip * -d /opt/protoc; chmod -R 755 /opt/protoc/bin; chmod -R u+rwX,go+rX,go-w /opt/protoc/include

# Add the `ubuntu` user, and the required folders
RUN useradd -ms /bin/bash -u 1000 ubuntu && \
    mkdir -p /home/ubuntu && \
    chown -R ubuntu.ubuntu /home/ubuntu && \
    mkdir -p /cargo /cargo_target /builds /builds/dfinity-lab && \
    chown -R 1000.1000 /cargo /cargo_target /builds

# Allow passworless sudo. entrypoint.sh relies on sudo
RUN usermod -a -G sudo ubuntu && echo "ubuntu ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Install Rust and Cargo in /opt
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    CARGO_TARGET_DIR=/cargo_target \
    ZSTD_LIB_DIR=/usr/lib \
    PROTOC=/opt/protoc/bin/protoc \
    PROTOC_INCLUDE=/opt/protoc/include \
    PATH=/opt/cargo/bin:$PATH

RUN curl --fail https://sh.rustup.rs -sSf \
        | sh -s -- -y --default-toolchain ${rust_version}-x86_64-unknown-linux-gnu --no-modify-path && \
    rustup default ${rust_version}-x86_64-unknown-linux-gnu && \
    chown -R ubuntu.ubuntu ${RUSTUP_HOME} ${CARGO_HOME}

# Cargo maintains a local cache of the registry index and of git checkouts of crates at CARGO_HOME
# Set this to a host-mounted volume
ENV CARGO_HOME=/cargo

# Init for reaping zombies and performing signal forwarding (https://github.com/krallin/tini)
ENV TINI_VERSION v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini.asc /tini.asc
RUN gpg --batch --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys 595E85A6B1B4779EA4DAAEC70B588DFF0527A9B7 && \
    gpg --batch --verify /tini.asc /tini && chmod +x /tini

COPY factsdb/requirements.txt /py-requirements1.txt
COPY scripts/requirements.txt /py-requirements2.txt
COPY docker/requirements.txt  /py-requirements3.txt
COPY scripts/jira-integration/requirements.txt /py-requirements4.txt
RUN ls py-requirements*.txt | xargs -n1 pip3 install --no-cache-dir -r

# Used to detect the Dockerfile changes and automatically rebuild the image
COPY docker /docker

# Download ic-admin
RUN  curl --fail https://download.dfinity.systems/ic/${ic_git_revision}/release/ic-admin.gz -o - | gunzip -c >| /usr/bin/ic-admin && \
    chmod +x /usr/bin/ic-admin

USER 1000

# Adds the GitHub and GitLab signatures to known_hosts so that `git pull` calls don't require user approval
RUN mkdir --mode=0700 /home/ubuntu/.ssh && \
    printf "# github.com:22 SSH-2.0-babeld-c34a939f\n|1|TtiEWekp4T2g6QFM8DnKUWYsXdw=|Vfo6utz2X8h5YSK2kjy1NrrVDQs= ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEAq2A7hRGmdnm9tUDbO9IDSwBK6TbQa+PXYPCPy6rbTrTtw7PHkccKrpp0yVhp5HdEIcKr6pLlVDBfOLX9QUsyCOV0wzfjIJNlGEYsdlLJizHhbn2mUjvSAHQqZETYP81eFzLQNnPHt4EVVUh7VfDESU84KezmD5QlWpXLmvU31/yMf+Se8xhHTvKSCZIFImWwoG6mbUoWf9nzpIoaSjB+weqqUUmpaaasXVal72J+UX2B+2RPW3RcT0eOzQgqlJL3RKrTJvdsjE3JEAvGq3lGHSZXy28G3skua2SmVi/w4yCE6gbODqnTWlg7+wC604ydGXA8VJiS5ap43JXiUFFAaQ==\n# gitlab.com:22 SSH-2.0-OpenSSH_7.9p1 Debian-10+deb10u2\n|1|fzEEbwQf9OkgzIxxgASmZp9L0Ec=|rty0872nCLuKDCsl3fuGSEmS81U= ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQCsj2bNKTBSpIYDEGk9KxsGh3mySTRgMtXL583qmBpzeQ+jqCMRgBqB98u3z++J1sKlXHWfM9dyhSevkMwSbhoR8XIq/U0tCNyokEi/ueaBMCvbcTHhO7FcwzY92WK4Yt0aGROY5qX2UKSeOvuP4D6TPqKF1onrSzH9bx9XUf2lEdWT/ia1NEKjunUqu1xOB/StKDHMoX4/OKyIzuS0q/T1zOATthvasJFoPrAjkohTyaDUz2LN5JoH839hViyEG82yB+MjcFV5MU3N1l1QL3cVUCh93xSaua1N85qivl+siMkPGbO5xR/En4iEY6K2XPASUEMaieWVNTRCtJ4S8H+9\n# gitlab.com:22 SSH-2.0-OpenSSH_7.9p1 Debian-10+deb10u2\n|1|k6eNETvB6tZStZlifmM3Y6qrmkw=|AWemKhMqEjFAPf/125ARU2xLpmk= ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBFSMqzJeV9rUzU4kWitGjeR4PWSa29SPqJ1fVkhtj3Hw9xjLVXVYrU9QlYWrOLXBpQ6KWjbjTDTdDkoohFzgbEY=\n# gitlab.com:22 SSH-2.0-OpenSSH_7.9p1 Debian-10+deb10u2\n|1|/5OjzdrCg7KAVMh9Q3xfH0OvqLY=|9Yl++Oih6L3ENgUjQCx3uqFPvfc= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAfuCHKVTjquxvt6CM6tdG4SLp1Btn/nOeHHE5UOzRdf\n# gitlab.com:22 SSH-2.0-OpenSSH_7.9p1 Debian-10+deb10u2\n" >> /home/ubuntu/.ssh/known_hosts && chmod 0600 /home/ubuntu/.ssh/known_hosts

COPY docker/entrypoint.sh /entrypoint.sh
WORKDIR /

ENTRYPOINT ["/tini", "--", "/entrypoint.sh"]
