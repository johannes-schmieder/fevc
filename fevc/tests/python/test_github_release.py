import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import sys
import zipfile

import pytest

TOOLS = Path(__file__).resolve().parents[2] / "tools"
sys.path.insert(0, str(TOOLS))
import build_github_release as release
import build_native_release as native
import build_release_artifact as portable


@pytest.fixture
def adopted(tmp_path):
    package = tmp_path / "fevc"
    package.mkdir()
    (package / "stata.toc").write_text("v 0.5.0\np fevc\n")
    (package / "fevc.pkg").write_text("v 3\nf fevc.ado\nf LICENSE\nf THIRD_PARTY_NOTICES.txt\n")
    (package / "fevc.ado").write_text("program define fevc\nend\n")
    for name in ("LICENSE", "THIRD_PARTY_NOTICES.txt"):
        (package / name).write_text("test fixture notice\n")
    files = list(portable.package_files(package))
    manifest = {"schema": "FEVC-POOLED-COMPONENT-ADOPTION-V1",
                "build_source_commit": "a" * 40, "binaries": [], "evidence": []}
    for name in native.BINARY_NAMES:
        data = ("test fixture " + name).encode()
        (package / name).write_bytes(data)
        files.append(portable.PackageFile(PurePosixPath(name), data))
        manifest["binaries"].append({"name": name, "sha256": portable.sha256(data),
            "bytes": len(data), "build_source_commit": "a" * 40,
            "runtime_qualification": "OWNER_MANUAL_TEST_PENDING" if "windows" in name else "PASS"})
    evidence = tmp_path / "adoption"
    evidence.mkdir()
    for key in ("corresponding_source", "windows_manual_adoption"):
        data = ("test fixture " + key).encode()
        (evidence / key).write_bytes(data)
        manifest[key] = {"path": key, "sha256": portable.sha256(data)}
    for i, item in enumerate(files):
        if str(item.relative) == "fevc.pkg":
            files[i] = portable.PackageFile(item.relative, native.render_native_manifest(item.data))
    for item in native.repository_files(tuple(files)):
        if str(item.relative) in ("fevc.pkg", "stata.toc"):
            (tmp_path / item.relative).write_bytes(item.data)
    path = evidence / "manifest.json"
    path.write_text(json.dumps(manifest))
    return tmp_path, path


def test_pending_windows_requires_explicit_authorization(adopted):
    with pytest.raises(ValueError, match="runtime qualification incomplete"):
        release.adopted_package(*adopted)
    files, manifest = release.adopted_package(*adopted, allow_pending_windows=True)
    assert manifest["binaries"][-1]["runtime_qualification"] == "OWNER_MANUAL_TEST_PENDING"
    first = release.zip_package(files)
    assert first == release.zip_package(files)
    with zipfile.ZipFile(io.BytesIO(first)) as archive:
        assert archive.namelist() == ["fevc/" + str(item.relative) for item in files]
        entries = archive.read("fevc/fevc.pkg").decode().splitlines()
        for line in entries:
            if line.startswith(("f ", "F ")):
                assert "fevc/" + line[2:] in archive.namelist()
        for item in files:
            assert archive.read("fevc/" + str(item.relative)) == item.data


@pytest.mark.parametrize("change", ["missing", "duplicate", "failed-windows", "pending-linux",
                                    "binary-hash", "source", "archive-hash", "unsafe", "catalog"])
def test_adopted_release_rejects_invalid_inputs(adopted, change):
    root, path = adopted
    manifest = json.loads(path.read_text())
    if change == "missing": manifest["binaries"].pop()
    elif change == "duplicate": manifest["binaries"][-1] = manifest["binaries"][0]
    elif change == "failed-windows": manifest["binaries"][-1]["runtime_qualification"] = "FAIL"
    elif change == "pending-linux": manifest["binaries"][-2]["runtime_qualification"] = "OWNER_MANUAL_TEST_PENDING"
    elif change == "binary-hash": manifest["binaries"][0]["sha256"] = "0" * 64
    elif change == "source": manifest["binaries"][0]["build_source_commit"] = "b" * 40
    elif change == "archive-hash": manifest["corresponding_source"]["sha256"] = "0" * 64
    elif change == "unsafe": manifest["corresponding_source"]["path"] = "../outside"
    elif change == "catalog": (root / "fevc.pkg").write_text("v 3\nf wrong.ado\n")
    path.write_text(json.dumps(manifest))
    with pytest.raises(ValueError):
        release.adopted_package(root, path, allow_pending_windows=True)


def test_tagged_repository_archive_keeps_installable_plugins():
    root = TOOLS.parents[1]
    paths = ["fevc/" + name for name in native.BINARY_NAMES]
    output = subprocess.check_output(["git", "check-attr", "export-ignore", "--", *paths],
                                     cwd=root, text=True)
    assert all(line.endswith(": unspecified") for line in output.splitlines())


def test_current_adopted_payload_and_catalogs_agree():
    root = TOOLS.parents[1]
    files, _ = release.adopted_package(root, root / "native/pooled-component-20261010/manifest.json",
                                     allow_pending_windows=True)
    assert len(files) == 63
