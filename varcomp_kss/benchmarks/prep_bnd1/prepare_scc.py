#!/usr/bin/env python3
"""Build immutable source and driver inputs for PREP-BND-1 SCC runs."""

from __future__ import annotations

import argparse
import json
import subprocess
from datetime import UTC, datetime
from pathlib import Path

from common import HEX40, require, sha256

DRIVERS = ("local_driver.do",)
WRAPPERS = ("run_pair.sge", "run_cz18_pair.sge")


def git(repo: Path, *args: str, stdout=None) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        text=stdout is None,
        capture_output=stdout is None,
        stdout=stdout,
        stderr=subprocess.PIPE,
    )
    return "" if stdout is not None else completed.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[3]
    baseline = git(repo, "rev-parse", f"{args.baseline}^{{commit}}")
    candidate = git(repo, "rev-parse", f"{args.candidate}^{{commit}}")
    require(HEX40.fullmatch(baseline) is not None, "invalid baseline commit")
    require(HEX40.fullmatch(candidate) is not None, "invalid candidate commit")
    require(not args.output_dir.exists() or not any(args.output_dir.iterdir()), "output must be empty")
    require(not git(repo, "status", "--porcelain"), "benchmark tool worktree must be clean")
    tool_commit = git(repo, "rev-parse", "HEAD^{commit}")
    tool_tree = git(repo, "rev-parse", "HEAD^{tree}")
    bundles = args.output_dir / "bundles"
    drivers = args.output_dir / "driver"
    wrappers = args.output_dir / "sge"
    bundles.mkdir(parents=True)
    drivers.mkdir()
    wrappers.mkdir()
    records: dict[str, object] = {}
    for role, commit in (("baseline", baseline), ("candidate", candidate)):
        bundle = bundles / f"{commit}.tar"
        with bundle.open("wb") as handle:
            git(repo, "archive", commit, stdout=handle)
        records[role] = {
            "commit": commit,
            "tree": git(repo, "rev-parse", f"{commit}^{{tree}}"),
            "bundle": str(bundle.relative_to(args.output_dir)),
            "bundle_sha256": sha256(bundle),
        }
    for name in (*DRIVERS, *WRAPPERS):
        relative = f"varcomp_kss/benchmarks/prep_bnd1/{name}"
        destination_root = drivers if name in DRIVERS else wrappers
        temporary = destination_root / name
        with temporary.open("wb") as handle:
            git(repo, "show", f"{tool_commit}:{relative}", stdout=handle)
        digest = sha256(temporary)
        destination = destination_root / (f"{digest}.do" if name.endswith(".do") else name)
        temporary.rename(destination)
        records[name] = {
            "source_path": relative,
            "path": str(destination.relative_to(args.output_dir)),
            "sha256": digest,
        }
    manifest = {
        "schema": "varcomp-kss-prep-bnd1-scc-input-v1",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "baseline": baseline,
        "candidate": candidate,
        "tool_commit": tool_commit,
        "tool_tree": tool_tree,
        "records": records,
        "synthetic_cases": [f"F{firms}-P256" for firms in (256, 1024, 4096, 8192)],
        "orders": ["ab", "ba"],
        "cz18": {
            "case": "CZ18-P20",
            "input_sha256": "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575",
            "data_not_in_bundle": True,
        },
        "stata_processors": 4,
    }
    manifest_path = args.output_dir / "input_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"PREP_BND1_SCC_INPUT_PASS manifest_sha256={sha256(manifest_path)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
