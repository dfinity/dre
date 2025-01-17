import os

if __name__ == "__main__":
    import pytest
    import sys

    dir_path = os.path.dirname(__file__)
    sys.path.append(dir_path)
    sys.path.append(os.path.join(dir_path, "tests"))

    if hasattr(pytest, "main"):
        raise SystemExit(
            pytest.main(
                args=[dir_path, "-vv", "-n=8"],
                plugins=["xdist"],
            )
        )
