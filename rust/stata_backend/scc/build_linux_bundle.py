#!/usr/bin/env python3
"""Build a deterministic exact-commit source bundle for SCC Linux qualification."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import subprocess
import tarfile
from pathlib import Path, PurePosixPath


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def git_archive(root: Path, commit: str) -> list[tuple[PurePosixPath, bytes, int]]:
    completed = subprocess.run(
        ["git", "-C", str(root), "archive", "--format=tar", commit],
        check=True,
        capture_output=True,
    )
    rows: list[tuple[PurePosixPath, bytes, int]] = []
    with tarfile.open(fileobj=io.BytesIO(completed.stdout), mode="r:") as source:
        for member in source.getmembers():
            if member.isdir():
                continue
            if not member.isfile():
                raise ValueError(f"source archive contains nonregular path: {member.name}")
            relative = PurePosixPath(member.name)
            if relative.is_absolute() or ".." in relative.parts:
                raise ValueError(f"unsafe source archive path: {relative}")
            # Repository installers include prior builds; qualification must
            # start from source and cannot consume those plugin candidates.
            if relative.parent == PurePosixPath("fevc") and relative.suffix == ".plugin":
                continue
            handle = source.extractfile(member)
            if handle is None:
                raise ValueError(f"could not read source archive path: {relative}")
            rows.append((relative, handle.read(), member.mode & 0o777))
    if not rows:
        raise ValueError("source archive is empty")
    paths = [row[0] for row in rows]
    if len(paths) != len(set(paths)):
        raise ValueError("git archive contains duplicate paths")
    rows.sort(key=lambda row: row[0])
    return rows


def build(root: Path, commit: str) -> tuple[bytes, str, int]:
    rows = git_archive(root, commit)
    rows.append((PurePosixPath("SOURCE_COMMIT.txt"), f"{commit}\n".encode(), 0o444))
    rows.sort(key=lambda row: row[0])
    manifest = "".join(
        f"{digest(payload)}  {relative}\n" for relative, payload, _ in rows
    ).encode()
    rows.append((PurePosixPath("SOURCE_FILES.sha256"), manifest, 0o444))
    rows.sort(key=lambda row: row[0])

    tar_buffer = io.BytesIO()
    with tarfile.open(fileobj=tar_buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for relative, payload, mode in rows:
            info = tarfile.TarInfo(str(PurePosixPath("source") / relative))
            info.size = len(payload)
            info.mode = mode
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mtime = 0
            archive.addfile(info, io.BytesIO(payload))
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
        compressed.write(tar_buffer.getvalue())
    payload = output.getvalue()
    return payload, digest(payload), len(rows)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    resolved = subprocess.run(
        ["git", "-C", str(root), "rev-parse", args.commit],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if resolved != args.commit or len(resolved) != 40:
        parser.error("--commit must be one full resolved Git object ID")
    payload, bundle_sha, count = build(root, resolved)
    if args.output.exists():
        parser.error("--output must not already exist")
    args.output.write_bytes(payload)
    print(
        f"VCKSS_SCC_LINUX_BUNDLE_READY source_commit={resolved} "
        f"bundle_sha256={bundle_sha} source_file_count={count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
