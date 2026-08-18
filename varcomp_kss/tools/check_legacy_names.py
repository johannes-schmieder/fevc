#!/usr/bin/env python3
"""Verify frozen evidence and reject predecessor names in active files."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO_ROOT = Path(__file__).resolve().parents[2]
SELF_REL = "varcomp_kss/tools/check_legacy_names.py"
INVENTORY_REL = "docs/migration/frozen_legacy_inventory.json"
RENAME_NOTE_REL = "docs/migration/RENAME_2026-08-18.md"
PREDECESSOR_COMMIT = "59aeb133470c0a27a5f543ca16fb8cdad5cc1a89"

TOKENS = (
    "kss_bc",
    "kssbc",
    "KSSBC",
    "KSS_BC",
    "shared/cmg",
    "ppmltalo_cmg",
    "apply_ppml",
)

# Every exception is tied to one path, token, location, and maximum count.
# The inventory counts are filled after its deterministic creation.
EXCEPTIONS: dict[tuple[str, str, str], int] = {
    **{(SELF_REL, token, "content"): 3 for token in TOKENS},
    (SELF_REL, TOKENS[0], "content"): 6,
    (SELF_REL, TOKENS[1], "content"): 7,
    (SELF_REL, TOKENS[6], "content"): 5,
    (INVENTORY_REL, "kss_bc", "content"): 398,
    (INVENTORY_REL, "kssbc", "content"): 0,
    (INVENTORY_REL, "KSSBC", "content"): 0,
    (INVENTORY_REL, "KSS_BC", "content"): 1,
    (INVENTORY_REL, "shared/cmg", "content"): 0,
    (INVENTORY_REL, "ppmltalo_cmg", "content"): 0,
    (INVENTORY_REL, "apply_ppml", "content"): 0,
    (RENAME_NOTE_REL, "kss_bc", "content"): 3,
    (RENAME_NOTE_REL, "kssbc", "content"): 2,
    (RENAME_NOTE_REL, "KSSBC", "content"): 1,
    (RENAME_NOTE_REL, "KSS_BC", "content"): 1,
    (RENAME_NOTE_REL, "shared/cmg", "content"): 2,
    (RENAME_NOTE_REL, "ppmltalo_cmg", "content"): 2,
    (RENAME_NOTE_REL, "apply_ppml", "content"): 1,
    ("varcomp_kss/cmg/tools/assemble.py", "kssbc", "content"): 1,
    ("varcomp_kss/cmg/tools/assemble.py", "apply_ppml", "content"): 1,
    ("varcomp_kss/cmg/tests/test_assembly.py", "kssbc", "content"): 1,
    ("varcomp_kss/cmg/tests/test_assembly.py", "apply_ppml", "content"): 1,
    ("varcomp_kss/tests/python/test_package_layout.py", "kss_bc", "content"): 1,
    # The equivalence harness must name and load the immutable predecessor.
    ("varcomp_kss/tests/equivalence/equivalence_driver.do", "kss_bc", "content"): 1,
    ("varcomp_kss/tests/equivalence/equivalence_driver.do", "kssbc", "content"): 6,
    ("varcomp_kss/tools/run_rename_equivalence.py", "kss_bc", "content"): 10,
    ("varcomp_kss/tools/run_rename_equivalence.py", "kssbc", "content"): 3,
    # The raw baseline qualification evidence must preserve the predecessor
    # command and private namespace exactly.  Keep this exception tied to the
    # two canonical artifacts and their deterministic occurrence counts.
    (
        "varcomp_kss/qualification/rename_equivalence/baseline.raw.json",
        TOKENS[0],
        "content",
    ): 79,
    (
        "varcomp_kss/qualification/rename_equivalence/baseline.raw.json",
        TOKENS[1],
        "content",
    ): 14,
    (
        "varcomp_kss/qualification/rename_equivalence/baseline.raw.tsv",
        TOKENS[0],
        "content",
    ): 79,
    (
        "varcomp_kss/qualification/rename_equivalence/baseline.raw.tsv",
        TOKENS[1],
        "content",
    ): 14,
}

FROZEN_PREFIXES = (
    "reviews/",
    "varcomp_kss/benchmarks/reports/",
    "varcomp_kss/cmg/benchmarks/reports/",
    "docs/history/",
)
FROZEN_EXACT = {
    "docs/migration/README.md",
    "varcomp_kss/cmg/docs/UPSTREAM_SOURCE_MANIFEST.yaml",
}
OPTIONAL_FROZEN_EXACT = {"docs/migration/ppml-variance-commit-map.tsv"}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_candidates() -> list[str]:
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        check=True,
        stdout=subprocess.PIPE,
    )
    paths = {item.decode("utf-8") for item in result.stdout.split(b"\0") if item}
    return sorted(path for path in paths if (REPO_ROOT / path).is_file())


def in_frozen_scope(path: str) -> bool:
    return (
        path in FROZEN_EXACT
        or path in OPTIONAL_FROZEN_EXACT
        or any(path.startswith(prefix) for prefix in FROZEN_PREFIXES)
    )


def load_inventory() -> tuple[dict[str, object], dict[str, dict[str, object]]]:
    path = REPO_ROOT / INVENTORY_REL
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read frozen inventory: {exc}") from exc
    if document.get("schema_version") != 1 or document.get("algorithm") != "sha256":
        raise ValueError("unsupported frozen inventory schema or algorithm")
    if document.get("predecessor_commit") != PREDECESSOR_COMMIT:
        raise ValueError("frozen inventory has the wrong predecessor commit")
    expected_scope = {
        "prefixes": list(FROZEN_PREFIXES),
        "required_exact": sorted(FROZEN_EXACT),
        "optional_exact": sorted(OPTIONAL_FROZEN_EXACT),
    }
    if document.get("scope") != expected_scope:
        raise ValueError("frozen inventory scope does not match the audit policy")
    records = document.get("files")
    if not isinstance(records, list):
        raise ValueError("frozen inventory files must be a list")
    by_path: dict[str, dict[str, object]] = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("frozen inventory contains a non-object record")
        rel = record.get("path")
        if not isinstance(rel, str) or rel in by_path:
            raise ValueError(f"invalid or duplicate frozen path: {rel!r}")
        if PurePosixPath(rel).is_absolute() or ".." in PurePosixPath(rel).parts:
            raise ValueError(f"unsafe frozen path: {rel}")
        by_path[rel] = record
    if list(by_path) != sorted(by_path):
        raise ValueError("frozen inventory records are not path-sorted")
    return document, by_path


def verify_inventory(candidates: list[str]) -> tuple[set[str], list[str]]:
    errors: list[str] = []
    try:
        _, records = load_inventory()
    except ValueError as exc:
        return set(), [str(exc)]

    expected = {path for path in candidates if in_frozen_scope(path)}
    recorded = set(records)
    for path in sorted(expected - recorded):
        errors.append(f"frozen file missing from inventory: {path}")
    for path in sorted(recorded - expected):
        errors.append(f"inventory path is absent or outside frozen scope: {path}")

    for rel in sorted(expected & recorded):
        record = records[rel]
        path = REPO_ROOT / rel
        expected_size = record.get("size")
        expected_hash = record.get("sha256")
        if not isinstance(expected_size, int) or expected_size < 0:
            errors.append(f"invalid frozen size: {rel}")
            continue
        if not isinstance(expected_hash, str) or len(expected_hash) != 64:
            errors.append(f"invalid frozen SHA-256: {rel}")
            continue
        actual_size = path.stat().st_size
        if actual_size != expected_size:
            errors.append(f"frozen size mismatch: {rel}: {actual_size} != {expected_size}")
            continue
        actual_hash = sha256(path)
        if actual_hash != expected_hash:
            errors.append(f"frozen hash mismatch: {rel}: {actual_hash} != {expected_hash}")
    return expected, errors


def decode_text(path: Path) -> str | None:
    data = path.read_bytes()
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def check_occurrences(path: str, location: str, value: str, errors: list[str]) -> int:
    findings = 0
    for token in TOKENS:
        count = value.count(token)
        if not count:
            continue
        findings += count
        maximum = EXCEPTIONS.get((path, token, location), 0)
        if count > maximum:
            errors.append(
                f"legacy token {token!r} occurs {count} time(s) in {location} of {path}; "
                f"allowed maximum is {maximum}"
            )
    return findings


def main() -> int:
    try:
        candidates = git_candidates()
    except (OSError, subprocess.CalledProcessError, UnicodeDecodeError) as exc:
        print(f"legacy-name audit could not enumerate Git candidates: {exc}", file=sys.stderr)
        return 2

    frozen, errors = verify_inventory(candidates)
    text_files = 0
    findings = 0
    for rel in candidates:
        if rel in frozen:
            continue
        findings += check_occurrences(rel, "path", rel, errors)
        text = decode_text(REPO_ROOT / rel)
        if text is None:
            continue
        text_files += 1
        findings += check_occurrences(rel, "content", text, errors)

    if errors:
        print("LEGACY NAME AUDIT FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        "LEGACY NAME AUDIT PASS: "
        f"{len(frozen)} frozen files verified; {text_files} active text files scanned; "
        f"{findings} allowlisted migration/config occurrences"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
