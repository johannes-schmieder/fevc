import json
import sys
from pathlib import Path

import pytest

TOOLS = Path(__file__).resolve().parents[2] / "tools"
sys.path.insert(0, str(TOOLS))
import build_native_release as native
import build_release_artifact as portable


@pytest.fixture
def inputs(tmp_path):
    manifest = {"schema": "FEVC-BINARY-INPUTS-V1", "source_commit": "a" * 40,
                "binaries": []}
    for name in native.BINARY_NAMES:
        payload = ("test fixture " + name).encode()
        (tmp_path / name).write_bytes(payload)
        evidence = name + ".json"
        data = json.dumps({"test_only": True, "sha256": portable.sha256(payload)}).encode()
        (tmp_path / evidence).write_bytes(data)
        manifest["binaries"].append(dict(name=name, sha256=portable.sha256(payload),
            evidence=evidence, evidence_sha256=portable.sha256(data),
            source_commit="a" * 40, status="PASS"))
    return tmp_path, manifest


def build(inputs):
    root, manifest = inputs
    return native.native_files(portable.PACKAGE_ROOT, root, manifest, "a" * 40)


def test_complete_payload_adds_every_platform_and_preserves_portable_source(inputs):
    before = portable.package_files(portable.PACKAGE_ROOT)
    files = build(inputs)
    assert len(files) == len(before) + 5
    assert {str(f.relative) for f in files if str(f.relative).endswith(".plugin")} == set(native.BINARY_NAMES)
    pkg = next(f.data.decode() for f in files if str(f.relative) == "fevc.pkg")
    assert all(pkg.count(f"f {name}\n") == 1 for name in native.BINARY_NAMES)
    assert portable.archive_files(files) == portable.archive_files(build(inputs))
    assert portable.package_files(portable.PACKAGE_ROOT) == before


@pytest.mark.parametrize("change", ["missing", "duplicate", "wrong-source", "failed",
                                    "binary-hash", "evidence-hash", "unsafe", "schema"])
def test_native_payload_fails_closed(inputs, change):
    root, manifest = inputs
    row = manifest["binaries"][0]
    if change == "missing": manifest["binaries"].pop()
    elif change == "duplicate": manifest["binaries"][1] = row.copy()
    elif change == "wrong-source": row["source_commit"] = "b" * 40
    elif change == "failed": row["status"] = "FAIL"
    elif change == "binary-hash": (root / row["name"]).write_bytes(b"wrong")
    elif change == "evidence-hash": row["evidence_sha256"] = "0" * 64
    elif change == "unsafe": row["evidence"] = "../outside"
    elif change == "schema": manifest["schema"] = "unknown"
    with pytest.raises(ValueError): build(inputs)


def test_native_payload_rejects_symlink(inputs):
    root, manifest = inputs
    name = manifest["binaries"][0]["name"]
    (root / name).unlink()
    (root / name).symlink_to(root / native.BINARY_NAMES[1])
    with pytest.raises(ValueError, match="symlink"):
        build(inputs)


@pytest.mark.parametrize("change", [None, "target", "build", "binary", "failed", "hash"])
def test_compatibility_review_binds_original_build_and_new_package(inputs, change):
    root, manifest = inputs
    row = manifest["binaries"][0]
    row["source_commit"] = "b" * 40
    review = {
        "schema": "FEVC-BINARY-COMPATIBILITY-V1", "status": "PASS",
        "build_source_commit": "b" * 40, "package_source_commit": "a" * 40,
        "binaries": {row["name"]: row["sha256"]},
        "unchanged_source_manifest_sha256": "c" * 64,
        "changed_paths": ["README.md"], "checks": ["test fixture only"],
        "limitations": ["No real qualification is represented by this fixture."],
    }
    if change == "target": review["package_source_commit"] = "d" * 40
    if change == "build": review["build_source_commit"] = "d" * 40
    if change == "binary": review["binaries"][row["name"]] = "0" * 64
    if change == "failed": review["status"] = "FAIL"
    data = json.dumps(review).encode()
    (root / "compatibility.json").write_bytes(data)
    row.update(compatibility_evidence="compatibility.json",
               compatibility_sha256=portable.sha256(data))
    if change == "hash": row["compatibility_sha256"] = "0" * 64
    if change is None:
        assert len(build(inputs)) == len(portable.package_files(portable.PACKAGE_ROOT)) + 5
        assert row["source_commit"] == "b" * 40
    else:
        with pytest.raises(ValueError, match="compatibility"):
            build(inputs)


def test_repository_install_manifest_resolves_to_the_complete_payload(inputs, tmp_path):
    files = build(inputs)
    destination = tmp_path / "repository"
    native.write_repository(destination, files)
    pkg = (destination / "fevc.pkg").read_text()
    listed = [line[2:] for line in pkg.splitlines() if line.startswith(("f ", "F "))]
    assert "F fevc/LICENSE\n" in pkg
    assert "F fevc/THIRD_PARTY_NOTICES.txt\n" in pkg
    assert len(listed) == len(set(listed))
    expected = {f"fevc/{item.relative}": item.data for item in files
                if str(item.relative) not in {"fevc.pkg", "stata.toc"}}
    assert set(listed) == set(expected)
    for relative in listed:
        assert (destination / relative).read_bytes() == expected[relative]
    assert all(f"fevc/{name}" in listed for name in native.BINARY_NAMES)
    assert (destination / "stata.toc").read_bytes() == (
        portable.PACKAGE_ROOT / "stata.toc").read_bytes()
    with pytest.raises(FileExistsError):
        native.write_repository(destination, files)
