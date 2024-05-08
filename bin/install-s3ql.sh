#!/bin/bash

set -eExuo pipefail

RELEASE_VERSION="5.2.0"
RELEASE_FILE="s3ql-${RELEASE_VERSION}.tar.gz"
SIG_FILE="${RELEASE_FILE}.asc"

pip_install() {
    # System-wide install
    # sudo pip3 --no-cache-dir install "$@"
    # User install
    pip3 --no-cache-dir install --user "$@"
}

# Debian packages
sudo apt-get install -y curl python3-dev build-essential pkg-config \
    libffi-dev libattr1-dev libsqlite3-dev libfuse-dev libfuse3-dev fuse3 psmisc

pip_install cryptography defusedxml apsw trio pyfuse3 requests llfuse async_generator typing

# Download S3QL source code
curl -L -O "https://github.com/s3ql/s3ql/releases/download/s3ql-${RELEASE_VERSION}/${RELEASE_FILE}"

# Install S3QL
tar -xf "$RELEASE_FILE"
(
    cd "s3ql-${RELEASE_VERSION}/"
    # Do not require google-auth and google-auth-oauthlib packages
    sed -e "/'google-auth',/d" -e "/'google-auth-oauthlib',/d" -i ./setup.py
    # Build and install
    python3 ./setup.py build_ext --inplace
    # python3 ./setup.py install
    python3 ./setup.py install --user
)

s3qlctrl --version

echo "S3ql version ${RELEASE_VERSION} installed successfully."

rm -rf "$RELEASE_FILE" "s3ql-${RELEASE_VERSION}/"
