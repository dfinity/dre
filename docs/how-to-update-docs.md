
# How to Update Documentation

We use MkDocs to generate, serve, and search the team documentation.
For full documentation visit [mkdocs.org](https://www.mkdocs.org).

## Commands

* `bazel run "//:mkdocs" new [dir-name]` - Create a new project.
* `bazel run "//:mkdocs" serve` - Start the live-reloading docs server.
* `bazel run "//:mkdocs" build` - Build the documentation site.
* `bazel run "//:mkdocs" -h` - Print help message and exit.

To generate documentation as HTML, you can use convenience script `./bin/mkdocs-build.sh`. The generated documentation will be in the `site` subdirectory of the repo root.

## Project layout

    mkdocs.yml    # The configuration file.
    docs/
        index.md  # The documentation homepage.
        ...       # Other markdown pages, images and other files.
