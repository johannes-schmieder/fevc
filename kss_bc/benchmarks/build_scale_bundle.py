#!/usr/bin/env python3
"""Build the deterministic, allowlisted KSS-SCALE-1 SCC source bundle."""
from __future__ import annotations

import argparse
import ast
import gzip
import hashlib
import io
import json
import os
import re
import stat
import subprocess
import tarfile
from pathlib import Path, PurePosixPath

BUNDLE_FORMAT = "KSS-SCALE-SOURCE-BUNDLE-V1"
INTERNAL_MANIFEST = PurePosixPath("BUNDLE_FILES.sha256")
SAFE_PATH = re.compile(r"[A-Za-z0-9._/-]+")
HEX40 = re.compile(r"[0-9a-f]{40}")

REQUIRED_INFRASTRUCTURE = {
    PurePosixPath("kss_bc/benchmarks/build_scale_bundle.py"),
    PurePosixPath("kss_bc/benchmarks/kss_scale_fixtures.do"),
    PurePosixPath("kss_bc/benchmarks/kss_scale_fixtures.mata"),
    PurePosixPath("kss_bc/benchmarks/scale_bundle_allowlist.txt"),
    PurePosixPath("kss_bc/benchmarks/scc/deploy_scale_bundle.sh"),
    PurePosixPath("kss_bc/benchmarks/scc/kss_scale_driver.do"),
    PurePosixPath("kss_bc/benchmarks/scc/process_tree_rss.awk"),
    PurePosixPath("kss_bc/benchmarks/scc/rng_k1_driver.do"),
    PurePosixPath("kss_bc/benchmarks/scc/run_kss_scale.sge"),
    PurePosixPath("kss_bc/benchmarks/scc/run_rng_k1.sge"),
    PurePosixPath("kss_bc/benchmarks/scc/submit_kss_scale.sh"),
    PurePosixPath("kss_bc/benchmarks/scc/submit_rng_k1.sh"),
    PurePosixPath("kss_bc/benchmarks/scc/validate_kss_scale.py"),
    PurePosixPath("kss_bc/benchmarks/scc/validate_rng_k1.py"),
}
REQUIRED_MATLAB_BENCHMARK = {
    PurePosixPath("kss_bc/benchmarks/matlab_scale/build_prepare_receipt.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/build_reference_receipt.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/build_wrapper_receipt.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/common.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/make_case.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/matlab_scale_cold.m"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/matlab_scale_run.m"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/matlab_scale_warm.m"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/monitor_process_tree.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/prepare_fixed_sample.do"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/run_matlab_scale.sge"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/run_prepare_fixed_sample.sge"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/source_contract.json"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/submit_matlab_scale.sh"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/validate.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/validate_scc_job.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/verify_case.py"),
    PurePosixPath("kss_bc/benchmarks/matlab_scale/verify_submission.py"),
}
REQUIRED_PACKAGE_METADATA = {
    PurePosixPath("kss_bc/kss_bc.pkg"),
    PurePosixPath("kss_bc/kss_bc.sthlp"),
    PurePosixPath("kss_bc/stata.toc"),
}
REQUIRED_DOCUMENTATION = {
    PurePosixPath("kss_bc/AGENTS.md"),
    PurePosixPath("kss_bc/CHANGELOG.md"),
    PurePosixPath("kss_bc/PLAN.md"),
    PurePosixPath("kss_bc/README.md"),
    PurePosixPath("kss_bc/TESTING.md"),
    PurePosixPath("kss_bc/benchmarks/README.md"),
    PurePosixPath("kss_bc/docs/ESTIMATOR_CONTRACT.md"),
    PurePosixPath("kss_bc/docs/FAILURES_AND_RETURNS.md"),
    PurePosixPath("kss_bc/docs/JLA_FINITE_PROJECTION.md"),
    PurePosixPath("kss_bc/docs/NUMERICAL_ARCHITECTURE.md"),
    PurePosixPath("kss_bc/docs/SOURCE_PROVENANCE.md"),
    PurePosixPath("shared/cmg/docs/API_CONTRACT.md"),
    PurePosixPath("shared/cmg/docs/IMPLEMENTATION_STATUS_2026-08-15.md"),
    PurePosixPath("shared/cmg/docs/MATH_CONTRACT.md"),
    PurePosixPath("shared/cmg/docs/SOURCE_PROVENANCE.md"),
}
REQUIRED_CMG_SOURCE = {
    PurePosixPath("cmg_plan.md"),
    PurePosixPath("shared/cmg/AGENTS.md"),
    PurePosixPath("shared/cmg/README.md"),
    PurePosixPath("shared/cmg/generated/kssbc_cmg_core.mata"),
    PurePosixPath("shared/cmg/generated/manifest.json"),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_allowlist(path: Path) -> list[PurePosixPath]:
    """Read canonical repository-relative paths from an explicit allowlist."""
    rows: list[PurePosixPath] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        require(SAFE_PATH.fullmatch(line) is not None,
                f"unsafe allowlist spelling: {line}")
        item = PurePosixPath(line)
        require(not item.is_absolute(), f"absolute allowlist path: {line}")
        require(str(item) == line, f"noncanonical allowlist path: {line}")
        require(".." not in item.parts and "." not in item.parts,
                f"unsafe allowlist path: {line}")
        rows.append(item)
    require(rows == sorted(set(rows), key=str),
            "allowlist must be sorted and duplicate-free")
    required = (REQUIRED_INFRASTRUCTURE | REQUIRED_MATLAB_BENCHMARK |
                REQUIRED_PACKAGE_METADATA | REQUIRED_DOCUMENTATION |
                REQUIRED_CMG_SOURCE)
    missing = sorted(required.difference(rows), key=str)
    require(not missing,
            "allowlist omits required scale files: " +
            ", ".join(map(str, missing)))
    return rows


def read_regular_no_symlinks(
    root: Path, relative: PurePosixPath,
) -> tuple[bytes, bool]:
    """Read one regular file while rejecting symlinks in every path component."""
    current = root
    for index, part in enumerate(relative.parts):
        current = current / part
        try:
            metadata = current.lstat()
        except FileNotFoundError as exc:
            raise ValueError(f"missing bundle file: {relative}") from exc
        require(not stat.S_ISLNK(metadata.st_mode),
                f"symlink in bundle path: {relative}")
        if index + 1 < len(relative.parts):
            require(stat.S_ISDIR(metadata.st_mode),
                    f"non-directory bundle parent: {relative}")
        else:
            require(stat.S_ISREG(metadata.st_mode),
                    f"nonregular bundle file: {relative}")
    return current.read_bytes(), bool(metadata.st_mode & 0o111)


def package_runtime_paths(root: Path) -> set[PurePosixPath]:
    payload, _ = read_regular_no_symlinks(
        root, PurePosixPath("kss_bc/kss_bc.pkg"))
    shipped: set[PurePosixPath] = set()
    declarations = 0
    for line in payload.decode("utf-8").splitlines():
        if not line.startswith("f "):
            continue
        declarations += 1
        spelling = line.removeprefix("f ").strip()
        item = PurePosixPath(spelling)
        require(SAFE_PATH.fullmatch(spelling) is not None and
                str(item) == spelling and len(item.parts) == 1,
                f"unsafe package runtime path: {spelling}")
        shipped.add(PurePosixPath("kss_bc") / item)
    require(shipped, "package manifest contains no runtime files")
    require(len(shipped) == declarations,
            "package manifest has duplicate runtime files")
    return shipped


def require_tracked(root: Path, rows: list[PurePosixPath]) -> None:
    completed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
    )
    tracked = {
        PurePosixPath(item.decode("utf-8"))
        for item in completed.stdout.split(b"\0")
        if item
    }
    missing = sorted(set(rows).difference(tracked), key=str)
    require(not missing,
            "allowlisted files are not tracked: " +
            ", ".join(map(str, missing)))


def validate_python(relative: PurePosixPath, payload: bytes) -> None:
    if relative.suffix != ".py":
        return
    try:
        ast.parse(payload.decode("utf-8"), filename=str(relative))
    except (SyntaxError, UnicodeDecodeError) as exc:
        raise ValueError(f"invalid Python source: {relative}: {exc}") from exc


def validate_json(relative: PurePosixPath, payload: bytes) -> None:
    if relative.suffix != ".json":
        return
    try:
        json.loads(payload.decode("utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise ValueError(f"invalid JSON source: {relative}: {exc}") from exc


def build(
    root: Path,
    allowlist: Path,
    source_commit: str,
    *,
    require_git_tracked: bool = False,
) -> tuple[bytes, str]:
    root = root.absolute()
    require(root.is_dir() and not root.is_symlink() and
            root.resolve(strict=True) == root,
            f"source root must be a real directory: {root}")
    require(HEX40.fullmatch(source_commit) is not None,
            "source commit must be 40 lowercase hexadecimal characters")
    allowlist_relative = PurePosixPath(
        "kss_bc/benchmarks/scale_bundle_allowlist.txt")
    allowlist_absolute = allowlist.absolute()
    require(allowlist_absolute == root / Path(allowlist_relative),
            "builder requires the canonical scale allowlist")
    read_regular_no_symlinks(root, allowlist_relative)
    rows = read_allowlist(allowlist_absolute)
    runtime = package_runtime_paths(root)
    missing_runtime = sorted(runtime.difference(rows), key=str)
    require(not missing_runtime,
            "allowlist omits installed runtime modules: " +
            ", ".join(map(str, missing_runtime)))
    if require_git_tracked:
        require_tracked(root, rows)

    entries: list[tuple[PurePosixPath, bytes, bool]] = [
        (PurePosixPath("BUNDLE_FORMAT.txt"),
         f"{BUNDLE_FORMAT}\n".encode(), False),
        (PurePosixPath("SOURCE_COMMIT.txt"),
         f"{source_commit}\n".encode(), False),
    ]
    for relative in rows:
        payload, executable = read_regular_no_symlinks(root, relative)
        validate_python(relative, payload)
        validate_json(relative, payload)
        entries.append((relative, payload, executable))

    manifest = "\n".join(
        f"{digest(payload)}  {relative}"
        for relative, payload, _ in entries
    ) + "\n"
    archive_entries = [
        *entries,
        (INTERNAL_MANIFEST, manifest.encode(), False),
    ]
    tar_buffer = io.BytesIO()
    with tarfile.open(
        fileobj=tar_buffer, mode="w", format=tarfile.PAX_FORMAT,
    ) as archive:
        for relative, payload, executable in archive_entries:
            info = tarfile.TarInfo(str(relative))
            info.size = len(payload)
            info.mode = 0o755 if executable else 0o644
            info.mtime = info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(payload))

    compressed = io.BytesIO()
    with gzip.GzipFile(
        filename="", mode="wb", fileobj=compressed, mtime=0, compresslevel=9,
    ) as stream:
        stream.write(tar_buffer.getvalue())
    return compressed.getvalue(), manifest


def write_immutable(path: Path, payload: bytes) -> None:
    if path.exists():
        require(path.is_file() and not path.is_symlink(),
                f"invalid immutable bundle target: {path}")
        require(path.read_bytes() == payload,
                f"immutable bundle collision: {path}")
        return
    temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
    temporary.write_bytes(payload)
    os.replace(temporary, path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--allowlist", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--require-git-tracked", action="store_true")
    args = parser.parse_args()
    bundle, manifest = build(
        args.root,
        args.allowlist,
        args.source_commit,
        require_git_tracked=args.require_git_tracked,
    )
    bundle_sha = digest(bundle)
    if not args.check_only:
        require(args.output_dir is not None,
                "--output-dir is required unless --check-only is used")
        args.output_dir.mkdir(parents=True, exist_ok=True)
        write_immutable(args.output_dir / f"{bundle_sha}.tar.gz", bundle)
        write_immutable(
            args.output_dir / f"{bundle_sha}.files.sha256",
            manifest.encode(),
        )
    print(bundle_sha)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
