import os


if __name__ == "__main__":
    import pytest

    dir_path = os.path.dirname(os.path.realpath(__file__))
    raise SystemExit(pytest.main([dir_path]))
