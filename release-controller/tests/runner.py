import os
import sys
import pytest

if __name__ == "__main__":
    dir_path = os.path.dirname(os.path.dirname(__file__))
    sys.path.append(dir_path)
    sys.path.append(os.path.join(dir_path, "tests"))
    raise SystemExit(
        pytest.main(
            args=sys.argv[1:] if sys.argv[1:] else [dir_path],
        )
    )
