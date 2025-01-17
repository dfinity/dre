import os

if __name__ == "__main__":
    import pytest
    import sys

    dir_path = os.path.dirname(__file__)
    sys.path.append(dir_path)
    sys.path.append(os.path.join(dir_path, "tests"))

    if hasattr(pytest, "main") or os.getenv("RUN_ANYWAY"):
        args = ["-vv", "-n=8"] + (sys.argv[1:] if sys.argv[1:] else [dir_path])
        raise SystemExit(
            pytest.main(
                args=args,
                plugins=["xdist"],
            )
        )
