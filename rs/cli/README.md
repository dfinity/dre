# Release CLI

Release CLI is used to enable faster and easier IC network operations for the Release Team.

Features include:

* HSM auto-detection
* Neuron auto-detection
* Node replacement
* All ic-admin get & propose commands

## Install

```shell
cargo install --git ssh://git@gitlab.com/dfinity-lab/core/release release_cli
```

## Usage

```shell
release_cli --help
```

## Troubleshooting

If you get an error like (observed on ubuntu 22.04)

``` shell
...ic-admin: error while loading shared libraries: libssl.so.1.1: cannot open shared object file: No such file or directory
```

you will need to install libssl 1.1.x. A simple solution is to install it from source directly from the OpenSSL website
https://www.openssl.org/source/.

Once downloaded and extracted, you can install it by running

``` shell
./config
make
make test
sudo make install
sudo ln -s /usr/local/lib/libssl.so.1.1  /usr/lib/libssl.so.1.1
sudo ln -s /usr/local/lib/libcrypto.so.1.1 /usr/lib/libcrypto.so.1.1
```
