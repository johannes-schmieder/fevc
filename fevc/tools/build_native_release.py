#!/usr/bin/env python3
"""Stage a complete, source-bound native RC; never publish or accept partial platforms."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path, PurePosixPath

import build_release_artifact as portable


BINARY_NAMES = (
    "fevc_rust_macos_arm64.plugin",
    "fevc_rust_macos_x86_64.plugin",
    "fevc_rust_macos.plugin",
    "fevc_rust_linux_x64.plugin",
    "fevc_rust_windows_x64.plugin",
)
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def native_files(package_root: Path, binary_root: Path, manifest: dict,
                 source_commit: str) -> tuple[portable.PackageFile, ...]:
    """Require reviewed evidence bindings; this does not create qualification evidence."""
    if not portable.COMMIT_RE.fullmatch(source_commit):
        raise ValueError("invalid source commit")
    if manifest.get("schema") != "FEVC-BINARY-INPUTS-V1":
        raise ValueError("invalid native input schema")
    if manifest.get("source_commit") != source_commit:
        raise ValueError("native source commit mismatch")
    rows = manifest.get("binaries")
    if not isinstance(rows, list) or len(rows) != len(BINARY_NAMES):
        raise ValueError("complete five-binary payload required")
    if {row.get("name") for row in rows} != set(BINARY_NAMES):
        raise ValueError("missing, duplicate or unexpected native binary")
    files = list(portable.package_files(package_root))
    if any(str(item.relative).endswith(".plugin") for item in files):
        raise ValueError("portable source manifest must not contain native binaries")
    for row in rows:
        name = row["name"]
        if row.get("status") != "PASS" or row.get("source_commit") != source_commit:
            raise ValueError(f"unqualified or wrong-source binary: {name}")
        payload = portable._read_regular(binary_root, PurePosixPath(name))
        if not payload or portable.sha256(payload) != row.get("sha256"):
            raise ValueError(f"native hash mismatch: {name}")
        evidence = portable._safe_relative(row.get("evidence", ""))
        evidence_data = portable._read_regular(binary_root, evidence)
        if (not evidence_data or not SHA256.fullmatch(row.get("evidence_sha256", ""))
                or portable.sha256(evidence_data) != row["evidence_sha256"]):
            raise ValueError(f"qualification evidence hash mismatch: {name}")
        files.append(portable.PackageFile(PurePosixPath(name), payload))
    for index, item in enumerate(files):
        if str(item.relative) == "fevc.pkg":
            data = item.data.rstrip(b"\n") + b"\n" + "".join(
                f"f {name}\n" for name in BINARY_NAMES
            ).encode()
            files[index] = portable.PackageFile(item.relative, data)
    return tuple(sorted(files, key=lambda item: str(item.relative)))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-dir", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true")
    group.add_argument("--output-dir", type=Path)
    args = parser.parse_args()
    source_commit = portable.clean_source_identity()
    manifest_data = args.manifest.read_bytes()
    manifest = json.loads(manifest_data)
    files = native_files(portable.PACKAGE_ROOT, args.binary_dir, manifest, source_commit)
    archive = portable.archive_files(files)
    if archive != portable.archive_files(files):
        raise RuntimeError("nondeterministic native archive")
    version = portable.package_version(portable.PACKAGE_ROOT)
    archive_name = f"fevc-{version}-native.tar.gz"
    receipt = json.loads(portable.render_receipt(
        archive_name=archive_name, archive=archive, files=files, source_commit=source_commit
    ))
    receipt.update(format="FEVC-BINARY-ARTIFACT-V1",
                   native_inputs_sha256=portable.sha256(manifest_data),
                   binaries=manifest["binaries"],
                   status="STAGED_NOT_RELEASED")
    if args.output_dir is not None:
        # New directory only: no mixing with or overwriting an earlier candidate.
        args.output_dir.mkdir(parents=True, exist_ok=False)
        (args.output_dir / archive_name).write_bytes(archive)
        (args.output_dir / f"fevc-{version}-native.receipt.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print(f"FEVC_NATIVE_ARTIFACT_CHECK_PASS version={version} files={len(files)} "
          f"sha256={portable.sha256(archive)} source_commit={source_commit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
