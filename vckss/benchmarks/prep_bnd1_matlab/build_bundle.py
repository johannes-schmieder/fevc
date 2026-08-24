#!/usr/bin/env python3
"""Build a deterministic, tracked PREP-BND-1 comparison source bundle."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import os
import re
import stat
import subprocess
import tarfile
from pathlib import Path, PurePosixPath

SAFE = re.compile(r"[A-Za-z0-9._/-]+")
HEX40 = re.compile(r"[0-9a-f]{40}")
ALLOWLIST = PurePosixPath(
    "vckss/benchmarks/prep_bnd1_matlab/bundle_allowlist.txt"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require_clean_head(root: Path, source_commit: str) -> None:
    """Bind worktree bytes to the exact clean commit named in the bundle."""
    head = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--verify", "HEAD^{commit}"],
        check=True,
        capture_output=True,
    ).stdout.decode().strip()
    require(head == source_commit, "source commit is not the current HEAD")
    status = subprocess.run(
        ["git", "-C", str(root), "status", "--porcelain=v1", "-z",
         "--untracked-files=all"],
        check=True,
        capture_output=True,
    ).stdout
    require(not status, "source worktree is not clean")


def read_paths(root: Path) -> list[PurePosixPath]:
    path = root / Path(ALLOWLIST)
    rows: list[PurePosixPath] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        item = PurePosixPath(line)
        require(SAFE.fullmatch(line) is not None and str(item) == line and
                not item.is_absolute() and ".." not in item.parts,
                f"unsafe allowlist path: {line}")
        rows.append(item)
    require(rows == sorted(set(rows), key=str),
            "allowlist must be sorted and unique")
    package = root / "vckss/vckss.pkg"
    runtime = {
        PurePosixPath("vckss") / line.removeprefix("f ").strip()
        for line in package.read_text(encoding="utf-8").splitlines()
        if line.startswith("f ")
    }
    require(runtime <= set(rows), "bundle omits an installed runtime file")
    required = {
        ALLOWLIST,
        PurePosixPath("vckss/benchmarks/prep_bnd1_matlab/build_bundle.py"),
        PurePosixPath("vckss/benchmarks/prep_bnd1_matlab/run_pair.sge"),
        PurePosixPath(
            "vckss/benchmarks/prep_bnd1_matlab/run_dense_oracle.sge"
        ),
        PurePosixPath("vckss/benchmarks/prep_bnd1_matlab/stata_run.do"),
        PurePosixPath("vckss/benchmarks/prep_bnd1_matlab/generate_input.do"),
        PurePosixPath("vckss/benchmarks/prep_bnd1_matlab/prep_bnd1_matlab_run.m"),
        PurePosixPath(
            "vckss/benchmarks/prep_bnd1_matlab/validate_dense_oracle.py"
        ),
        PurePosixPath(
            "vckss/benchmarks/prep_bnd1_matlab/validate_dense_oracle_scc.py"
        ),
        PurePosixPath("vckss/benchmarks/oracle/stata_oracle.do"),
        PurePosixPath("vckss/benchmarks/oracle/vckss_dense_oracle.m"),
        PurePosixPath("vckss/benchmarks/matlab_scale/source_contract.json"),
        PurePosixPath("vckss/benchmarks/matlab_scale/common.py"),
        PurePosixPath("vckss/benchmarks/matlab_scale/monitor_process_tree.py"),
        PurePosixPath("vckss/benchmarks/scc/verify_numopt2_matlab_source.py"),
    }
    require(required <= set(rows), "bundle omits required comparison source")
    tracked = {
        PurePosixPath(value.decode())
        for value in subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z"], check=True,
            capture_output=True,
        ).stdout.split(b"\0") if value
    }
    missing = sorted(set(rows) - tracked, key=str)
    require(not missing, "allowlisted files are not tracked: " +
            ", ".join(map(str, missing)))
    return rows


def regular_file(root: Path, relative: PurePosixPath) -> tuple[bytes, bool]:
    current = root
    for index, part in enumerate(relative.parts):
        current = current / part
        metadata = current.lstat()
        require(not stat.S_ISLNK(metadata.st_mode), f"symlink: {relative}")
        if index + 1 < len(relative.parts):
            require(stat.S_ISDIR(metadata.st_mode), f"invalid parent: {relative}")
        else:
            require(stat.S_ISREG(metadata.st_mode), f"nonregular file: {relative}")
    return current.read_bytes(), bool(metadata.st_mode & 0o111)


def build(root: Path, source_commit: str) -> tuple[bytes, str]:
    require(HEX40.fullmatch(source_commit) is not None, "invalid source commit")
    require_clean_head(root, source_commit)
    paths = read_paths(root)
    entries: list[tuple[PurePosixPath, bytes, bool]] = [
        (PurePosixPath("BUNDLE_FORMAT.txt"),
         b"PREP-BND-1-MATLAB-SOURCE-BUNDLE-V1\n", False),
        (PurePosixPath("SOURCE_COMMIT.txt"),
         f"{source_commit}\n".encode(), False),
    ]
    entries.extend((path, *regular_file(root, path)) for path in paths)
    manifest = "".join(
        f"{sha256_bytes(payload)}  {relative}\n"
        for relative, payload, _ in entries
    )
    entries.append((PurePosixPath("BUNDLE_FILES.sha256"), manifest.encode(), False))
    tar_bytes = io.BytesIO()
    with tarfile.open(fileobj=tar_bytes, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for relative, payload, executable in entries:
            info = tarfile.TarInfo(str(relative))
            info.size = len(payload)
            info.mode = 0o755 if executable else 0o644
            info.mtime = info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(payload))
    compressed = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=compressed,
                       mtime=0, compresslevel=9) as stream:
        stream.write(tar_bytes.getvalue())
    return compressed.getvalue(), manifest


def write_once(path: Path, payload: bytes) -> None:
    if path.exists():
        require(path.is_file() and not path.is_symlink() and
                path.read_bytes() == payload, f"bundle collision: {path}")
        return
    temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
    temporary.write_bytes(payload)
    os.replace(temporary, path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    archive, manifest = build(root, args.source_commit)
    digest = sha256_bytes(archive)
    if not args.check_only:
        require(args.output_dir is not None and args.output_dir.is_dir(),
                "existing output directory is required")
        write_once(args.output_dir / f"{digest}.tar.gz", archive)
        write_once(args.output_dir / f"{digest}.files.sha256", manifest.encode())
    print(digest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
