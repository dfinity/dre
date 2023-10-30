# Release CLI

Release CLI is used to enable faster and easier IC network operations for the Release Team.

Features include:

* HSM auto-detection
* Neuron auto-detection
* Node replacement
* All ic-admin get & propose commands

### Mac OS users with M1 chip

Before installing you need to install OpenSSL@3 for intel processor:

```shell
/usr/sbin/softwareupdate --install-rosetta

# Install homebrew for intel apps
cd ~/Downloads
mkdir homebrew
curl -L https://github.com/Homebrew/brew/tarball/master | tar xz --strip 1 -C homebrew
sudo mv homebrew /usr/local/homebrew
alias axbrew='arch -x86_64 /usr/local/homebrew/bin/brew'

# Install openssl@3
axbrew install openssl@3
sudo ln -s /usr/local/homebrew/Cellar/openssl@3/3.0.8 /usr/local/opt/openssl@3
```

## Install

```shell
cargo install --git ssh://git@gitlab.com/dfinity-lab/core/release release_cli
```

Make sure you have `libssl.so.1.1` on your system (Ubuntu 22.04 and later
will not carry it).  See below under *Troubleshooting* to get that going.

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
# The following adds the libraries to your system
# path, where Cargo will look for them.
sudo ln -s /usr/local/lib/libssl.so.1.1  /usr/lib/libssl.so.1.1
sudo ln -s /usr/local/lib/libcrypto.so.1.1 /usr/lib/libcrypto.so.1.1
# If you would rather not modify anything under /usr,
# you can instead set the LD_LIBRARY_PATH= variable
# to /usr/local in your ~/.bashrc.
```
