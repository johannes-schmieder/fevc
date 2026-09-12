from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess

import pytest

SCRIPT = Path(__file__).resolve().parents[2] / "tools/license_audit.py"
spec = importlib.util.spec_from_file_location("fevc_license_audit", SCRIPT)
assert spec is not None and spec.loader is not None
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def test_git_inventory_covers_tracked_and_new_manifests_not_ignored_snapshots(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    (tmp_path / ".git").mkdir()
    kept = ("rust/Cargo.toml", "rust/new-package/Cargo.toml")
    for relative in (*kept, "rust/experiments/ignored/Cargo.toml"):
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("[package]\n", encoding="utf-8")

    def git_inventory(command: list[str], **kwargs: object) -> subprocess.CompletedProcess:
        assert command == [
            "git", "-C", str(tmp_path), "ls-files", "-z", "--cached",
            "--others", "--exclude-standard", "--", "rust",
        ]
        assert kwargs["check"] is True
        return subprocess.CompletedProcess(command, 0, b"\0".join(
            item.encode() for item in (*kept, kept[0], "rust/deleted/Cargo.toml")
        ))

    monkeypatch.setattr(audit.subprocess, "run", git_inventory)
    assert audit.cargo_manifests(tmp_path) == tuple(tmp_path / p for p in kept)


def test_source_distribution_still_audits_every_cargo_manifest(tmp_path: Path) -> None:
    path = tmp_path / "rust/new-package/Cargo.toml"
    path.parent.mkdir(parents=True)
    path.write_text("[package]\n", encoding="utf-8")
    assert audit.cargo_manifests(tmp_path) == (path,)


def test_git_inventory_failure_is_not_silently_skipped(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    (tmp_path / ".git").mkdir()

    def fail(command: list[str], **kwargs: object) -> None:
        raise subprocess.CalledProcessError(1, command)

    monkeypatch.setattr(audit.subprocess, "run", fail)
    with pytest.raises(subprocess.CalledProcessError):
        audit.cargo_manifests(tmp_path)
