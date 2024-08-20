# Getting Started

## Downloading the pre-built binary

We build the DRE tool for Linux and for MacOS (Darwin). We tested on Ubuntu 22.04 and 24.04.

On Linux, you can download the tool with:

```bash
mkdir -p $HOME/bin
curl -L https://github.com/dfinity/dre/releases/latest/download/dre-x86_64-unknown-linux -o $HOME/bin/dre
chmod +x $HOME/bin/dre
```

On MacOS you can use the following:

```bash
mkdir -p $HOME/bin
curl -L https://github.com/dfinity/dre/releases/latest/download/dre-x86_64-apple-darwin -o $HOME/bin/dre
chmod +x $HOME/bin/dre
```

Make sure that `$HOME/bin` is added to your path. If it's not, you might get an error such as:
```bash
❯ dre
Command 'dre' not found, did you mean:
[...]
```

To fix this issue, you can run
```bash
export PATH=$HOME/bin:$PATH
```

And you can also add this to your $HOME/.bashrc file (or the one appropriate for your shell).
