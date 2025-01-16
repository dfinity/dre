import os

if __name__ == "__main__":
    import pytest

    dir_path = os.path.dirname(__file__)
    if hasattr(pytest, "main"):
        raise SystemExit(
            pytest.main(
                args=[dir_path, "-vv", "-n=8"],
                plugins=["xdist"],
            )
        )
