#!/usr/bin/env python3
"""Refresh or verify an unmodified CMG snapshot with external provenance."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

REPOSITORY = "https://github.com/johannes-schmieder/CMG"
ROOT_FILES = {"Cargo.toml", "Cargo.lock", "LICENSE", "README.md", "CHANGELOG.md", "RELEASING.md"}
DIRECTORIES = {"src", "tests", "examples", "docs"}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def upstream(source: Path, revision: str) -> tuple[str, dict[str, bytes]]:
    def git(*args: str) -> bytes:
        return subprocess.check_output(["git", "-C", str(source), *args])

    commit = git("rev-parse", "--verify", revision + "^{commit}").decode().strip()
    paths = git("ls-tree", "-r", "--name-only", commit).decode().splitlines()
    selected = [p for p in paths if p in ROOT_FILES or p.split("/", 1)[0] in DIRECTORIES]
    if not ROOT_FILES.issubset(selected):
        raise ValueError("revision does not contain a complete CMG package")
    return commit, {p: git("show", f"{commit}:{p}") for p in selected}


def inventory(destination: Path) -> dict[str, str]:
    result = {}
    for path in sorted(destination.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"vendored symlink is not permitted: {path}")
        if path.is_file():
            result[path.relative_to(destination).as_posix()] = digest(path.read_bytes())
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["refresh", "check"])
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--record", type=Path, required=True)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--rev")
    args = parser.parse_args()
    destination, record = args.destination.resolve(), args.record.resolve()
    if destination == record or destination in record.parents:
        parser.error("provenance must be outside the unmodified source payload")
    if bool(args.source) != bool(args.rev):
        parser.error("--source and --rev must be provided together")
    if args.action == "refresh":
        if not args.source:
            parser.error("refresh requires --source and --rev")
        commit, files = upstream(args.source, args.rev)
        if destination == args.source.resolve() or destination in args.source.resolve().parents:
            parser.error("destination must not replace the upstream checkout")
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=destination.parent) as staging:
            staged = Path(staging) / "cmg"
            for name, data in files.items():
                target = staged / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
            hashes = inventory(staged)
            if destination.exists():
                shutil.rmtree(destination)
            staged.rename(destination)
        record.parent.mkdir(parents=True, exist_ok=True)
        record.write_text(json.dumps({"repository": REPOSITORY, "revision": commit,
                                      "files": hashes}, indent=2, sort_keys=True) + "\n")
    provenance = json.loads(record.read_text())
    if provenance["repository"] != REPOSITORY:
        raise ValueError("unexpected CMG repository")
    revision = provenance["revision"]
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision):
        raise ValueError("CMG revision must be a full commit SHA")
    expected = provenance["files"]
    if args.source:
        commit, files = upstream(args.source, args.rev)
        if commit != revision or expected != {p: digest(data) for p, data in files.items()}:
            raise ValueError("provenance differs from the selected upstream commit")
    actual = inventory(destination)
    if actual != expected:
        changed = sorted(p for p in actual.keys() | expected.keys() if actual.get(p) != expected.get(p))
        raise ValueError("vendored CMG differs: " + ", ".join(changed))
    print(f"CMG {revision}: {len(actual)} unmodified source files verified")


if __name__ == "__main__":
    main()
