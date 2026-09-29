"""Historical audits distinguish absent clone history from corrupt evidence."""
import subprocess

import pytest

from .history_support import historical_file


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


@pytest.fixture
def histories(tmp_path):
    root = tmp_path / "full"
    root.mkdir()
    git(root, "init", "-b", "main")
    git(root, "config", "user.name", "Fixture")
    git(root, "config", "user.email", "fixture@example.invalid")
    git(root, "config", "commit.gpgsign", "false")
    (root / "input").write_text("original\n")
    git(root, "add", "input")
    git(root, "commit", "-m", "Original")
    original = git(root, "rev-parse", "HEAD")
    (root / "input").write_text("current\n")
    git(root, "commit", "-am", "Current")
    shallow = tmp_path / "shallow"
    subprocess.run(["git", "clone", "--depth=1", root.as_uri(), str(shallow)],
                   check=True, capture_output=True)
    return root, shallow, original


def test_full_history_reads_original_and_rejects_missing_evidence(histories):
    root, _, original = histories
    assert historical_file(root, original, "input") == b"original\n"
    with pytest.raises(subprocess.CalledProcessError):
        historical_file(root, original, "missing")
    with pytest.raises(pytest.fail.Exception, match="requires commit"):
        historical_file(root, "0" * 40, "input")


def test_shallow_history_is_explicitly_skipped_or_required(histories, monkeypatch):
    _, shallow, original = histories
    monkeypatch.delenv("FEVC_REQUIRE_HISTORY", raising=False)
    with pytest.raises(pytest.skip.Exception, match="fetch full history"):
        historical_file(shallow, original, "input")
    monkeypatch.setenv("FEVC_REQUIRE_HISTORY", "1")
    with pytest.raises(pytest.fail.Exception, match="fetch full history"):
        historical_file(shallow, original, "input")
    with pytest.raises(subprocess.CalledProcessError):
        historical_file(shallow, git(shallow, "rev-parse", "HEAD"), "missing")
