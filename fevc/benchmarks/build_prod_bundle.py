#!/usr/bin/env python3
"""Build the deterministic, allowlisted KSS-PROD SCC source bundle."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import os
import tarfile
from pathlib import Path, PurePosixPath


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_allowlist(path: Path) -> list[PurePosixPath]:
    rows: list[PurePosixPath] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        item = PurePosixPath(line)
        require(not item.is_absolute(), f"absolute allowlist path: {line}")
        require(".." not in item.parts and "." not in item.parts, f"unsafe allowlist path: {line}")
        rows.append(item)
    require(rows == sorted(set(rows), key=str), "allowlist must be sorted and duplicate-free")
    return rows


def build(root: Path, allowlist: Path, source_commit: str) -> tuple[bytes, str]:
    rows = read_allowlist(allowlist)
    require(len(source_commit) == 40 and all(c in "0123456789abcdef" for c in source_commit),
            "source commit must be 40 lowercase hexadecimal characters")
    manifest: list[str] = []
    tar_buffer = io.BytesIO()
    with tarfile.open(fileobj=tar_buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        entries: list[tuple[PurePosixPath, bytes, bool]] = [
            (PurePosixPath("SOURCE_COMMIT.txt"), f"{source_commit}\n".encode(), False)
        ]
        for relative in rows:
            source = root / Path(relative)
            require(source.is_file() and not source.is_symlink(), f"missing/nonregular bundle file: {relative}")
            entries.append((relative, source.read_bytes(), os.access(source, os.X_OK)))
        for relative, payload, executable in entries:
            manifest.append(f"{digest(payload)}  {relative}")
            info = tarfile.TarInfo(str(relative))
            info.size = len(payload)
            info.mode = 0o755 if executable else 0o644
            info.mtime = info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(payload))
    compressed = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=compressed, mtime=0, compresslevel=9) as stream:
        stream.write(tar_buffer.getvalue())
    return compressed.getvalue(), "\n".join(manifest) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--allowlist", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    bundle, manifest = build(root, args.allowlist, args.source_commit)
    bundle_sha = digest(bundle)
    if not args.check_only:
        require(args.output_dir is not None, "--output-dir is required unless --check-only is used")
        args.output_dir.mkdir(parents=True, exist_ok=True)
        bundle_path = args.output_dir / f"{bundle_sha}.tar.gz"
        manifest_path = args.output_dir / f"{bundle_sha}.files.sha256"
        for path, payload in ((bundle_path, bundle), (manifest_path, manifest.encode())):
            if path.exists():
                require(path.read_bytes() == payload, f"immutable bundle collision: {path}")
            else:
                temporary = path.with_suffix(path.suffix + f".tmp.{os.getpid()}")
                temporary.write_bytes(payload)
                os.replace(temporary, path)
    print(bundle_sha)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
