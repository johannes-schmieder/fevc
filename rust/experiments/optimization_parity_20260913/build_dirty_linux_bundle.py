#!/usr/bin/env python3
"""Freeze the approved dirty FEVC source for isolated SCC qualification."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import subprocess
import tarfile
from pathlib import Path, PurePosixPath


def sha(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def git_paths(root: Path, *arguments: str) -> list[PurePosixPath]:
    completed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", *arguments],
        check=True, capture_output=True,
    )
    return [PurePosixPath(raw.decode()) for raw in completed.stdout.split(b"\0") if raw]


def permitted_untracked(path: PurePosixPath) -> bool:
    parts = path.parts
    if path == PurePosixPath("fevc/tests/python/test_macos_toolchain_identity.py"):
        return True
    if len(parts) == 2 and parts[0] == "fevc":
        return path.suffix in {".ado", ".mata", ".sthlp", ".pkg", ".toc"}
    if len(parts) >= 3 and parts[:2] == ("fevc", "docs"):
        return path.suffix in {".md", ".json"}
    if len(parts) >= 4 and parts[:3] == ("fevc", "tests", "stata"):
        return path.suffix == ".do"
    if len(parts) >= 5 and parts[:2] == ("rust", "crates"):
        return path.suffix == ".rs" and parts[2] in {"vckss-core", "vckss-plugin"}
    if len(parts) >= 4 and parts[:2] == ("rust", "experiments") and parts[2] in {
        "optimization_parity_20260913", "pipeline_optimization_20260915"
    }:
        return path.suffix in {".md", ".py", ".do", ".c", ".sh", ".sge"}
    return False


def source_rows(root: Path) -> list[tuple[PurePosixPath, bytes, int]]:
    tracked = git_paths(root, "--cached")
    untracked = git_paths(root, "--others", "--exclude-standard")
    paths = tracked + untracked
    if any(not path.parts or path.is_absolute() or ".." in path.parts for path in paths):
        raise ValueError("working tree has an unsafe source path")
    unexpected = [
        path for path in untracked
        if path.parts[0] != "KSS_Veneto_replication" and not permitted_untracked(path)
    ]
    if unexpected:
        raise ValueError(f"unexpected untracked source path: {unexpected[0]}")
    selected = sorted(set(tracked) | {
        path for path in untracked if path.parts[0] != "KSS_Veneto_replication"
    })
    if not selected:
        raise ValueError("working-tree source archive is empty")
    rows = []
    for relative in selected:
        if relative.parts[0] in {"KSS_Veneto_replication", ".local", ".git"}:
            raise ValueError(f"restricted source path: {relative}")
        source = root.joinpath(*relative.parts)
        if not source.is_file() or source.is_symlink():
            raise ValueError(f"source must be a regular file: {relative}")
        rows.append((relative, source.read_bytes(), source.stat().st_mode & 0o777))
    return rows


def build(root: Path, commit: str, snapshot: Path | None = None) -> tuple[bytes, str, int]:
    if snapshot is None:
        rows = source_rows(root)
    else:
        receipt=json.loads(snapshot.read_text())
        if receipt['git_head']!=commit: raise ValueError('snapshot Git identity mismatch')
        rows=[]
        for name,digest in sorted(receipt['source'].items()):
            relative=PurePosixPath(name)
            if relative.is_absolute() or '..' in relative.parts or relative.parts[0] not in {
                'rust','fevc','ci','tests','AGENTS.md','LICENSE','pyproject.toml','pytest.ini','rust-toolchain.toml'}:
                raise ValueError(f'unsafe snapshot input: {name}')
            path=snapshot.parent/'source'/name
            if path.is_symlink() or not path.is_file() or sha(path.read_bytes())!=digest:
                raise ValueError(f'snapshot input mismatch: {name}')
            rows.append((relative,path.read_bytes(),path.stat().st_mode & 0o777))
        rows.append((PurePosixPath('SOURCE_SNAPSHOT_SHA256.txt'),(sha(snapshot.read_bytes())+'\n').encode(),0o444))
    rows.extend([
        (PurePosixPath("SOURCE_COMMIT.txt"), f"{commit}\n".encode(), 0o444),
        (PurePosixPath("SOURCE_SNAPSHOT_KIND.txt"), b"DIRTY_WORKTREE_SNAPSHOT_V1\n", 0o444),
    ])
    rows.sort(key=lambda row: row[0])
    manifest = "".join(f"{sha(data)}  {path}\n" for path, data, _ in rows).encode()
    rows.append((PurePosixPath("SOURCE_FILES.sha256"), manifest, 0o444))
    rows.sort(key=lambda row: row[0])
    tar_buffer = io.BytesIO()
    with tarfile.open(fileobj=tar_buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for relative, data, mode in rows:
            info = tarfile.TarInfo(str(PurePosixPath("source") / relative))
            info.size = len(data)
            info.mode = mode
            info.uid = info.gid = info.mtime = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(data))
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
        compressed.write(tar_buffer.getvalue())
    payload = output.getvalue()
    return payload, sha(payload), len(rows)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--snapshot", type=Path)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    resolved = subprocess.run(
        ["git", "-C", str(root), "rev-parse", arguments.commit],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    if resolved != arguments.commit or len(resolved) != 40:
        parser.error("--commit must be one full resolved Git object ID")
    if arguments.output.exists():
        parser.error("--output must not already exist")
    payload, digest, count = build(root, resolved, arguments.snapshot)
    arguments.output.write_bytes(payload)
    print(
        f"VCKSS_SCC_LINUX_BUNDLE_READY source_commit={resolved} "
        f"bundle_sha256={digest} source_file_count={count} "
        "source_kind=dirty_worktree"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
