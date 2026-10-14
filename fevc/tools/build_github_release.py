#!/usr/bin/env python3
"""Package the adopted repository payload as a deterministic installable ZIP.

This preserves the adoption's qualification statuses. It does not qualify a
new binary or relax the separate fully-qualified native builder.
"""
from __future__ import annotations

import argparse
import io
import json
from pathlib import Path, PurePosixPath
import zipfile

import build_native_release as native
import build_release_artifact as portable


def adopted_package(repo: Path, manifest_path: Path, *, allow_pending_windows: bool = False):
    adoption = json.loads(manifest_path.read_bytes())
    if adoption.get("schema") != "FEVC-POOLED-COMPONENT-ADOPTION-V1":
        raise ValueError("unsupported adoption manifest")
    source = adoption.get("build_source_commit", "")
    if not portable.COMMIT_RE.fullmatch(source):
        raise ValueError("invalid build source")
    rows = adoption.get("binaries", [])
    if len(rows) != len(native.BINARY_NAMES) or {r.get("name") for r in rows} != set(native.BINARY_NAMES):
        raise ValueError("missing, duplicate or unexpected binary")
    files = list(portable.package_files(repo / "fevc"))
    for row in rows:
        status = row.get("runtime_qualification")
        pending = (row["name"] == "fevc_rust_windows_x64.plugin"
                   and status == "OWNER_MANUAL_TEST_PENDING" and allow_pending_windows)
        if status != "PASS" and not pending:
            raise ValueError("runtime qualification incomplete: " + row["name"])
        if row.get("build_source_commit") != source:
            raise ValueError("binary build source mismatch")
        payload = portable._read_regular(repo / "fevc", PurePosixPath(row["name"]))
        if portable.sha256(payload) != row.get("sha256") or len(payload) != row.get("bytes"):
            raise ValueError("binary hash or size mismatch: " + row["name"])
        files.append(portable.PackageFile(PurePosixPath(row["name"]), payload))
    for row in [*adoption.get("evidence", []), adoption["corresponding_source"],
                adoption["windows_manual_adoption"]]:
        payload = portable._read_regular(manifest_path.parent, portable._safe_relative(row["path"]))
        if portable.sha256(payload) != row["sha256"]:
            raise ValueError("adoption evidence hash mismatch: " + row["path"])
    for i, item in enumerate(files):
        if str(item.relative) == "fevc.pkg":
            files[i] = portable.PackageFile(item.relative, native.render_native_manifest(item.data))
    files = tuple(sorted(files, key=lambda item: str(item.relative)))
    for item in native.repository_files(files):
        if portable._read_regular(repo, item.relative) != item.data:
            raise ValueError("repository catalog/payload mismatch: " + str(item.relative))
    return files, adoption


def zip_package(files: tuple[portable.PackageFile, ...]) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for item in files:
            relative = portable._safe_relative(str(item.relative))
            info = zipfile.ZipInfo("fevc/" + str(relative), date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, item.data, compresslevel=9)
    return buffer.getvalue()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adoption-manifest", required=True, type=Path)
    parser.add_argument("--allow-pending-windows", action="store_true",
                        help="use only with explicit owner authorization; preserve pending status")
    choice = parser.add_mutually_exclusive_group(required=True)
    choice.add_argument("--check", action="store_true")
    choice.add_argument("--output-dir", type=Path)
    args = parser.parse_args()
    files, adoption = adopted_package(portable.REPO_ROOT, args.adoption_manifest,
                                     allow_pending_windows=args.allow_pending_windows)
    archive = zip_package(files)
    if archive != zip_package(files):
        raise RuntimeError("nondeterministic release ZIP")
    version = portable.package_version(portable.PACKAGE_ROOT)
    if args.check:
        print(f"FEVC_GITHUB_PACKAGE_CHECK_PASS version={version} files={len(files)} "
              f"sha256={portable.sha256(archive)}")
        return 0
    source_commit = portable.clean_source_identity()
    source_row = adoption["corresponding_source"]
    source_data = portable._read_regular(args.adoption_manifest.parent,
                                         portable._safe_relative(source_row["path"]))
    name = f"fevc-{version}.zip"
    receipt = {
        "schema": "FEVC-GITHUB-RELEASE-PACKAGE-V1", "version": version,
        "tag": "v" + version, "package_source_commit": source_commit,
        "build_source_commit": adoption["build_source_commit"],
        "archive": name, "archive_sha256": portable.sha256(archive),
        "adoption_manifest_sha256": portable.sha256(args.adoption_manifest.read_bytes()),
        "binaries": adoption["binaries"], "corresponding_source": source_row,
        "windows_runtime_pending_authorized": args.allow_pending_windows,
        "files": [{"path": "fevc/" + str(item.relative), "sha256": portable.sha256(item.data),
                   "size": len(item.data)} for item in files],
        "limitations": ["Windows Stata runtime testing remains pending.",
                        "Mac x86-64 runtime qualification uses Rosetta.",
                        "Package validation does not expand scientific or platform claims."],
    }
    output = args.output_dir
    output.mkdir(parents=True, exist_ok=False)
    assets = {name: archive, Path(source_row["path"]).name: source_data,
              "release-manifest.json": (json.dumps(receipt, indent=2) + "\n").encode()}
    for asset, data in assets.items():
        (output / asset).write_bytes(data)
    (output / "SHA256SUMS").write_text("".join(
        f"{portable.sha256(data)}  {asset}\n" for asset, data in sorted(assets.items())))
    print(f"FEVC_GITHUB_PACKAGE_PASS source={source_commit} archive={output / name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
