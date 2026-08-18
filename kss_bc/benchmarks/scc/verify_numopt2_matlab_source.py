#!/usr/bin/env python3
"""Verify the checksum-bound maintained MATLAB tree without copying it."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def inventory_hash(root: Path, relative_paths: list[str]) -> str:
    digest = hashlib.sha256()
    for relative in sorted(relative_paths):
        path = root / relative
        require(path.is_file() and not path.is_symlink(), f"invalid source: {path}")
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(bytes.fromhex(sha256(path)))
    return digest.hexdigest()


def runtime_tree(root: Path, roots: list[str]) -> tuple[list[str], str]:
    paths: list[str] = []
    for relative_root in roots:
        directory = root / relative_root
        require(directory.is_dir() and not directory.is_symlink(),
                f"invalid runtime root: {directory}")
        for path in directory.rglob("*"):
            if path.is_file():
                require(not path.is_symlink(), f"runtime symlink: {path}")
                paths.append(path.relative_to(root).as_posix())
    paths.sort()
    return paths, inventory_hash(root, paths)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--matlab-root", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--contract-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(args.contract.is_file(), "missing source contract")
    require(sha256(args.contract) == args.contract_sha256,
            "source contract hash changed")
    contract = json.loads(args.contract.read_text(encoding="utf-8"))
    require(contract.get("schema") == "kss_matlab_scale_source_v1",
            "source contract schema changed")
    paths, tree_hash = runtime_tree(
        args.matlab_root, contract["runtime_tree"]["roots"]
    )
    require(len(paths) == contract["runtime_tree"]["file_count"],
            "maintained runtime file count changed")
    require(tree_hash == contract["runtime_tree"]["sha256"],
            "maintained runtime tree changed")
    core = contract["core"]["relative_path"]
    require(sha256(args.matlab_root / core) == contract["core"]["sha256"],
            "maintained MATLAB core changed")
    payload = {
        "schema": "KSS-NUMOPT-2-MATLAB-SOURCE-V1",
        "status": "PASS",
        "matlab_upstream_commit": contract["maintained_upstream_commit"],
        "matlab_runtime_tree_sha256": tree_hash,
        "matlab_core_sha256": contract["core"]["sha256"],
        "matlab_cmg_entry_sha256": sha256(
            args.matlab_root / contract["cmg_entry"]["relative_path"]
        ),
        "matlab_hierarchy_sha256": inventory_hash(
            args.matlab_root, contract["hierarchy_sources"]
        ),
        "matlab_solver_sha256": inventory_hash(
            args.matlab_root, contract["solver_sources"]
        ),
        "source_contract_sha256": args.contract_sha256,
        "runtime_file_count": len(paths),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print("KSS_NUMOPT2_MATLAB_SOURCE_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
