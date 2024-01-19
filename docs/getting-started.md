# Getting Started

## Downloading the pre-built binary

These instructions are for a Linux based system. We tested on Ubuntu 22.04.

```bash
mkdir -p $HOME/bin
curl -L https://github.com/dfinity/dre/releases/download/v0.1.0/dre -o $HOME/bin/dre
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
