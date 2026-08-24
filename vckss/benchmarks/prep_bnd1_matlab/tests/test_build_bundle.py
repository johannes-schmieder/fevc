from __future__ import annotations

import subprocess

import pytest
from build_bundle import require_clean_head

COMMIT = "a" * 40


def completed(stdout: bytes) -> subprocess.CompletedProcess[bytes]:
    return subprocess.CompletedProcess(args=[], returncode=0, stdout=stdout)


def test_bundle_source_accepts_exact_clean_head(tmp_path, monkeypatch) -> None:
    replies = iter((completed(f"{COMMIT}\n".encode()), completed(b"")))
    monkeypatch.setattr(subprocess, "run", lambda *args, **kwargs: next(replies))
    require_clean_head(tmp_path, COMMIT)


def test_bundle_source_rejects_mismatched_head(tmp_path, monkeypatch) -> None:
    monkeypatch.setattr(
        subprocess, "run", lambda *args, **kwargs: completed(("b" * 40).encode())
    )
    with pytest.raises(ValueError, match="not the current HEAD"):
        require_clean_head(tmp_path, COMMIT)


def test_bundle_source_rejects_dirty_or_untracked_files(
    tmp_path, monkeypatch
) -> None:
    replies = iter((completed(COMMIT.encode()), completed(b"?? stray.txt\0")))
    monkeypatch.setattr(subprocess, "run", lambda *args, **kwargs: next(replies))
    with pytest.raises(ValueError, match="worktree is not clean"):
        require_clean_head(tmp_path, COMMIT)
