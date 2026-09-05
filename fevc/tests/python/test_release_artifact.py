from __future__ import annotations

import gzip
import importlib.util
import io
import json
import subprocess
import sys
import tarfile
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "fevc/tools/build_release_artifact.py"
SPEC = importlib.util.spec_from_file_location("build_release_artifact", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def test_release_artifact_is_deterministic_and_manifest_exact() -> None:
    first, files = MODULE.build_archive()
    second, second_files = MODULE.build_archive()
    assert first == second
    assert files == second_files

    with gzip.GzipFile(fileobj=io.BytesIO(first), mode="rb") as compressed:
        tar_payload = compressed.read()
    with tarfile.open(fileobj=io.BytesIO(tar_payload), mode="r:") as archive:
        members = archive.getmembers()
        expected = ["fevc", *(f"fevc/{item.relative}" for item in files)]
        assert [member.name for member in members] == expected
        assert members[0].isdir() and members[0].mode == 0o755
        assert all(member.uid == 0 and member.gid == 0 for member in members)
        assert all(member.uname == "" and member.gname == "" for member in members)
        assert all(member.mtime == 0 for member in members)
        assert all(member.isfile() and member.mode == 0o644 for member in members[1:])
        for item in files:
            assert archive.extractfile(f"fevc/{item.relative}").read() == item.data


def test_release_receipt_binds_source_archive_and_files() -> None:
    archive, files = MODULE.build_archive()
    source_commit = "a" * 40
    receipt = json.loads(
        MODULE.render_receipt(
            archive_name="fevc-0.5.0-rc.1.tar.gz",
            archive=archive,
            files=files,
            source_commit=source_commit,
        )
    )
    assert receipt["format"] == "FEVC-PORTABLE-SOURCE-ARTIFACT-V1"
    assert receipt["version"] == "0.5.0-rc.1"
    assert receipt["source_commit"] == source_commit
    assert receipt["archive_sha256"] == MODULE.sha256(archive)
    assert [row["path"] for row in receipt["files"]] == [
        f"fevc/{item.relative}" for item in files
    ]


def test_release_artifact_contains_only_catalog_manifest_and_manifest_files() -> None:
    files = MODULE.package_files(MODULE.PACKAGE_ROOT)
    manifest = (MODULE.PACKAGE_ROOT / "fevc.pkg").read_text(encoding="utf-8").splitlines()
    listed = sorted(line.removeprefix("f ").strip() for line in manifest if line.startswith("f "))
    assert [str(item.relative) for item in files] == sorted(["stata.toc", "fevc.pkg", *listed])
    assert {"LICENSE", "THIRD_PARTY_NOTICES.txt", "fevc.sthlp"} <= set(listed)
    assert not any(path.endswith(".plugin") for path in listed)


def test_release_artifact_rejects_unsafe_or_symlinked_manifest_paths(tmp_path: Path) -> None:
    package = tmp_path / "fevc"
    package.mkdir()
    (package / "stata.toc").write_text("v 0.5.0-rc.1\n", encoding="utf-8")
    (package / "fevc.pkg").write_text("v 3\nf ../escape\n", encoding="utf-8")
    with pytest.raises(ValueError, match="unsafe package path"):
        MODULE.package_files(package)

    (package / "payload").write_text("source\n", encoding="utf-8")
    (package / "linked").symlink_to(package / "payload")
    (package / "fevc.pkg").write_text("v 3\nf linked\n", encoding="utf-8")
    with pytest.raises(ValueError, match="symlink in package path"):
        MODULE.package_files(package)


def test_release_artifact_check_is_read_only(tmp_path: Path) -> None:
    completed = subprocess.run(
        [str(REPO_ROOT / ".venv/bin/python"), str(SCRIPT), "--check"],
        cwd=REPO_ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert completed.stderr == ""
    assert completed.stdout.startswith("FEVC_RELEASE_ARTIFACT_CHECK_PASS ")
    assert list(tmp_path.iterdir()) == []
