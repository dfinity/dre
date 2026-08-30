"""Repository rule to generate requirements.lock from uv.lock"""

def _requirements_lock_impl(repository_ctx):
    """Generate requirements.lock from uv.lock"""
    
    # Check if uv is available
    uv_result = repository_ctx.execute(["which", "uv"])
    if uv_result.return_code != 0:
        # uv not found, create an empty requirements.lock
        repository_ctx.file("requirements.lock", "# uv not found, please install uv and run: uv export --format requirements-txt --output-file requirements.lock")
        return
    
    # Check if uv.lock exists
    uv_lock_path = repository_ctx.path("uv.lock")
    if not uv_lock_path.exists:
        repository_ctx.file("requirements.lock", "# uv.lock not found")
        return
    
    # Generate requirements.lock from uv.lock
    result = repository_ctx.execute([
        "uv", "export", "--format", "requirements-txt", "--output-file", "requirements.lock"
    ])
    
    if result.return_code != 0:
        fail("Failed to generate requirements.lock: " + result.stderr)

requirements_lock = repository_rule(
    implementation = _requirements_lock_impl,
    attrs = {},
)