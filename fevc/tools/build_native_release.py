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
        if (row.get("status") != "PASS"
                or not portable.COMMIT_RE.fullmatch(row.get("source_commit", ""))):
            raise ValueError(f"unqualified binary: {name}")
        payload = portable._read_regular(binary_root, PurePosixPath(name))
        if not payload or portable.sha256(payload) != row.get("sha256"):
            raise ValueError(f"native hash mismatch: {name}")
        evidence = portable._safe_relative(row.get("evidence", ""))
        evidence_data = portable._read_regular(binary_root, evidence)
        if (not evidence_data or not SHA256.fullmatch(row.get("evidence_sha256", ""))
                or portable.sha256(evidence_data) != row["evidence_sha256"]):
            raise ValueError(f"qualification evidence hash mismatch: {name}")
        if row.get("source_commit") != source_commit:
            # Preserve the actual build SHA when a recorded compatibility
            # review carries qualification across packaging-only changes.
            review_path = portable._safe_relative(row.get("compatibility_evidence", ""))
            review_data = portable._read_regular(binary_root, review_path)
            if portable.sha256(review_data) != row.get("compatibility_sha256"):
                raise ValueError(f"compatibility evidence hash mismatch: {name}")
            review = json.loads(review_data)
            if (review.get("schema") != "FEVC-BINARY-COMPATIBILITY-V1"
                    or review.get("status") != "PASS"
                    or review.get("build_source_commit") != row.get("source_commit")
                    or review.get("package_source_commit") != source_commit
                    or review.get("binaries", {}).get(name) != row["sha256"]
                    or not SHA256.fullmatch(review.get("unchanged_source_manifest_sha256", ""))
                    or not review.get("changed_paths") or not review.get("checks")
                    or "limitations" not in review):
                raise ValueError(f"invalid compatibility review: {name}")
        files.append(portable.PackageFile(PurePosixPath(name), payload))
    for index, item in enumerate(files):
        if str(item.relative) == "fevc.pkg":
            # Stata otherwise treats these notices as optional ancillary files.
            lines = item.data.decode("utf-8").splitlines()
            lines = ["F " + line[2:] if line in {
                "f LICENSE", "f THIRD_PARTY_NOTICES.txt"
            } else line for line in lines]
            lines.extend(f"f {name}" for name in BINARY_NAMES)
            data = ("\n".join(lines) + "\n").encode("utf-8")
            files[index] = portable.PackageFile(item.relative, data)
    return tuple(sorted(files, key=lambda item: str(item.relative)))


def repository_files(files: tuple[portable.PackageFile, ...]) -> tuple[portable.PackageFile, ...]:
    """Expose one package at the repository root without duplicating runtime source."""
    output = []
    for item in files:
        name = str(item.relative)
        if name == "fevc.pkg":
            lines = item.data.decode("utf-8").splitlines()
            data = "\n".join(
                line[:2] + "fevc/" + line[2:] if line.startswith(("f ", "F ")) else line
                for line in lines
            ) + "\n"
            output.append(portable.PackageFile(item.relative, data.encode("utf-8")))
        elif name == "stata.toc":
            output.append(item)
        else:
            output.append(portable.PackageFile(PurePosixPath("fevc") / item.relative, item.data))
    return tuple(sorted(output, key=lambda item: str(item.relative)))


def write_repository(directory: Path, files: tuple[portable.PackageFile, ...]) -> None:
    """Write only to a new staging directory; never modify the live checkout."""
    directory.mkdir(parents=True, exist_ok=False)
    for item in repository_files(files):
        relative = portable._safe_relative(str(item.relative))
        target = directory / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(item.data)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-dir", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true")
    group.add_argument("--output-dir", type=Path)
    group.add_argument("--repository-dir", type=Path,
                       help="stage root fevc.pkg/stata.toc and complete fevc/ payload")
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
    if args.repository_dir is not None:
        write_repository(args.repository_dir, files)
        receipt["repository_files"] = [
            {"path": str(item.relative), "sha256": portable.sha256(item.data),
             "size": len(item.data)} for item in repository_files(files)
        ]
        (args.repository_dir / "native-package.receipt.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"FEVC_NATIVE_ARTIFACT_CHECK_PASS version={version} files={len(files)} "
          f"sha256={portable.sha256(archive)} source_commit={source_commit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
