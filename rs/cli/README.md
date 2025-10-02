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
cargo install --git https://github.com/dfinity/dre.git dre
```

## Usage

```shell
dre --help
```

## Troubleshooting

