"""Read immutable audit inputs without confusing clone depth with a failure."""
import os
import subprocess

import pytest


def historical_file(root, commit, relative):
    present = subprocess.run(["git", "cat-file", "-e", f"{commit}^{{commit}}"],
                             cwd=root, capture_output=True)
    if present.returncode:
        shallow = subprocess.check_output(
            ["git", "rev-parse", "--is-shallow-repository"], cwd=root, text=True,
        ).strip() == "true"
        message = f"historical audit requires commit {commit}; fetch full history"
        if shallow and os.environ.get("FEVC_REQUIRE_HISTORY") != "1":
            pytest.skip(message)
        pytest.fail(message)
    # A missing file at an available commit is always an audit failure.
    return subprocess.check_output(["git", "show", f"{commit}:{relative}"], cwd=root)
