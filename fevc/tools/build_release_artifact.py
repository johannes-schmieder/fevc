#!/usr/bin/env python3
"""Build a deterministic, non-publishing FEVC portable source artifact."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import re
import subprocess
import tarfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


REPO_ROOT = Path(__file__).resolve().parents[2]
PACKAGE_ROOT = REPO_ROOT / "fevc"
ARCHIVE_ROOT = "fevc"
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
VERSION_RE = re.compile(r"^[0-9A-Za-z][0-9A-Za-z.-]*$")


@dataclass(frozen=True)
class PackageFile:
    relative: PurePosixPath
    data: bytes


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _safe_relative(value: str) -> PurePosixPath:
    relative = PurePosixPath(value)
    if (
        not value
        or relative.is_absolute()
        or relative.as_posix() != value
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise ValueError(f"unsafe package path: {value!r}")
    return relative


def _read_regular(package_root: Path, relative: PurePosixPath) -> bytes:
    current = package_root
    if current.is_symlink():
        raise ValueError(f"package root is a symlink: {package_root}")
    for part in relative.parts:
        current = current / part
        if current.is_symlink():
            raise ValueError(f"symlink in package path: {relative}")
    if not current.is_file():
        raise ValueError(f"missing package file: {relative}")
    return current.read_bytes()


def package_version(package_root: Path) -> str:
    lines = (package_root / "stata.toc").read_text(encoding="utf-8").splitlines()
    if not lines or not lines[0].startswith("v "):
        raise ValueError("stata.toc does not begin with a version record")
    version = lines[0].removeprefix("v ").strip()
    if not VERSION_RE.fullmatch(version):
        raise ValueError(f"invalid package version: {version!r}")
    return version


def package_files(package_root: Path) -> tuple[PackageFile, ...]:
    manifest_path = package_root / "fevc.pkg"
    lines = manifest_path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "v 3":
        raise ValueError("fevc.pkg does not use the expected Stata package format")
    listed = [
        _safe_relative(line.removeprefix("f ").strip())
        for line in lines
        if line.startswith("f ")
    ]
    if not listed or len(listed) != len(set(listed)):
        raise ValueError("fevc.pkg has no files or contains duplicate file records")
    relatives = (
        PurePosixPath("stata.toc"),
        PurePosixPath("fevc.pkg"),
        *listed,
    )
    return tuple(
        PackageFile(relative, _read_regular(package_root, relative))
        for relative in sorted(relatives, key=str)
    )


def build_archive(package_root: Path = PACKAGE_ROOT) -> tuple[bytes, tuple[PackageFile, ...]]:
    files = package_files(package_root)
    return archive_files(files), files


def archive_files(files: tuple[PackageFile, ...]) -> bytes:
    """Serialize an already validated, sorted installation inventory."""
    tar_payload = io.BytesIO()
    with tarfile.open(fileobj=tar_payload, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        directory = tarfile.TarInfo(ARCHIVE_ROOT)
        directory.type = tarfile.DIRTYPE
        directory.mode = 0o755
        directory.uid = directory.gid = 0
        directory.uname = directory.gname = ""
        directory.mtime = 0
        archive.addfile(directory)
        for item in files:
            info = tarfile.TarInfo(f"{ARCHIVE_ROOT}/{item.relative}")
            info.mode = 0o644
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mtime = 0
            info.size = len(item.data)
            archive.addfile(info, io.BytesIO(item.data))
    compressed = io.BytesIO()
    with gzip.GzipFile(
        filename="", fileobj=compressed, mode="wb", compresslevel=9, mtime=0
    ) as output:
        output.write(tar_payload.getvalue())
    return compressed.getvalue()


def render_receipt(
    *, archive_name: str, archive: bytes, files: tuple[PackageFile, ...], source_commit: str
) -> str:
    if not COMMIT_RE.fullmatch(source_commit):
        raise ValueError("source commit must be a full lowercase Git SHA")
    receipt = {
        "archive": archive_name,
        "archive_sha256": sha256(archive),
        "files": [
            {
                "path": f"{ARCHIVE_ROOT}/{item.relative}",
                "sha256": sha256(item.data),
                "size": len(item.data),
            }
            for item in files
        ],
        "format": "FEVC-PORTABLE-SOURCE-ARTIFACT-V1",
        "source_commit": source_commit,
        "version": package_version(PACKAGE_ROOT),
    }
    return json.dumps(receipt, indent=2, sort_keys=True) + "\n"


def clean_source_identity(repo_root: Path = REPO_ROOT) -> str:
    status = subprocess.run(
        ["git", "-C", str(repo_root), "status", "--porcelain", "--untracked-files=all"],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout
    if status:
        raise RuntimeError("release artifact construction requires a clean committed checkout")
    commit = subprocess.run(
        ["git", "-C", str(repo_root), "rev-parse", "HEAD"],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()
    if not COMMIT_RE.fullmatch(commit):
        raise RuntimeError("Git did not return a full source commit")
    return commit


def write_artifact(output_dir: Path, *, source_commit: str) -> tuple[Path, Path, str]:
    archive, files = build_archive()
    version = package_version(PACKAGE_ROOT)
    archive_path = output_dir / f"fevc-{version}.tar.gz"
    receipt_path = output_dir / f"fevc-{version}.receipt.json"
    if archive_path.exists() or receipt_path.exists():
        raise FileExistsError("refusing to overwrite an existing release artifact or receipt")
    output_dir.mkdir(parents=True, exist_ok=True)
    archive_path.write_bytes(archive)
    receipt_path.write_text(
        render_receipt(
            archive_name=archive_path.name,
            archive=archive,
            files=files,
            source_commit=source_commit,
        ),
        encoding="utf-8",
    )
    return archive_path, receipt_path, sha256(archive)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify determinism without writing")
    parser.add_argument("--output-dir", type=Path, help="directory for the archive and receipt")
    args = parser.parse_args()
    if args.check == (args.output_dir is not None):
        parser.error("choose exactly one of --check or --output-dir")
    if args.check:
        first, files = build_archive()
        second, second_files = build_archive()
        if first != second or files != second_files:
            raise RuntimeError("portable release artifact construction is not deterministic")
        print(
            "FEVC_RELEASE_ARTIFACT_CHECK_PASS "
            f"version={package_version(PACKAGE_ROOT)} files={len(files)} sha256={sha256(first)}"
        )
        return 0
    source_commit = clean_source_identity()
    archive_path, receipt_path, digest = write_artifact(
        args.output_dir.resolve(), source_commit=source_commit
    )
    print(
        "FEVC_RELEASE_ARTIFACT_PASS "
        f"archive={archive_path} receipt={receipt_path} sha256={digest}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
