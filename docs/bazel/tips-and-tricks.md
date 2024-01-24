
??? tip "Refresh Python dependencies in Bazel"

    Steps:
    1. `poetry add <dependency>`
    2. Run `./bin/poetry-export.sh`
    3. Use regular bazel operations, the new dependency should now be available
