#!/usr/bin/env python3
"""Verify relocated evidence and reject predecessor names in active files."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO_ROOT = Path(__file__).resolve().parents[2]
SELF_REL = "vckss/tools/check_legacy_names.py"
V1_INVENTORY_REL = "docs/migration/frozen_legacy_inventory.json"
V2_INVENTORY_REL = "docs/migration/relocation_inventory_v2.json"
V1_RENAME_NOTE_REL = "docs/migration/RENAME_2026-08-18.md"
V2_RENAME_NOTE_REL = "docs/migration/RENAME_TO_VCKSS_2026-08-24.md"
BASELINE_FAILURE_REL = "vckss/docs/EXACT_V7_BASELINE_FAILURE_2026-08-24.md"
V2_EQ_DRIVER_REL = "vckss/tests/equivalence/vckss_equivalence_driver.do"
V2_EQ_RUNNER_REL = "vckss/tools/run_vckss_rename_equivalence.py"
V2_EQ_BASELINE_MANIFEST_REL = (
    "vckss/qualification/vckss_rename_equivalence/"
    "baseline.plugin-source-manifest.sha256"
)
V2_EQ_BASELINE_JSON_REL = (
    "vckss/qualification/vckss_rename_equivalence/baseline.raw.json"
)
V2_EQ_BASELINE_TSV_REL = (
    "vckss/qualification/vckss_rename_equivalence/baseline.raw.tsv"
)
V2_EQ_RECEIPT_REL = (
    "vckss/qualification/vckss_rename_equivalence/receipt.json"
)
SOURCE_COMMIT = "7fcf1b20105a546e993e0b3186c842f05f1f79a8"
RECEIPT_TIP_COMMIT = "f9fb00dc6254116a51853b1c2be7162b0a48368e"
PREDECESSOR_COMMIT = "59aeb133470c0a27a5f543ca16fb8cdad5cc1a89"

TOKENS = (
    "varcomp_kss",
    "varcomp-kss",
    "VARCOMP_KSS",
    "VARCOMP-KSS",
    "kss_bc",
    "kssbc",
    "KSSBC",
    "KSS_BC",
    "shared/cmg",
    "ppmltalo_cmg",
    "apply_ppml",
)

# Every exception is tied to one path, token, location, and maximum count.
# The exact generated-inventory and self counts are locked by unit tests.
EXCEPTIONS: dict[tuple[str, str, str], int] = {
    (SELF_REL, TOKENS[0], "content"): 4,
    **{(SELF_REL, token, "content"): 1 for token in TOKENS[1:]},
    (V2_INVENTORY_REL, TOKENS[0], "content"): 1_077,
    (V2_INVENTORY_REL, TOKENS[3], "content"): 30,
    (V2_INVENTORY_REL, TOKENS[4], "content"): 796,
    (V2_INVENTORY_REL, TOKENS[7], "content"): 2,
    ("vckss/cmg/tools/assemble.py", TOKENS[5], "content"): 1,
    ("vckss/cmg/tools/assemble.py", TOKENS[10], "content"): 1,
    ("vckss/cmg/tests/test_assembly.py", TOKENS[5], "content"): 1,
    ("vckss/cmg/tests/test_assembly.py", TOKENS[10], "content"): 1,
    ("vckss/tests/python/test_package_layout.py", TOKENS[4], "content"): 1,
    ("vckss/tests/stata/test_rust_public_install.do", TOKENS[0], "content"): 1,
    ("README.md", TOKENS[0], "content"): 1,
    (V2_RENAME_NOTE_REL, TOKENS[0], "content"): 7,
    (V2_RENAME_NOTE_REL, TOKENS[1], "content"): 1,
    (V2_RENAME_NOTE_REL, TOKENS[2], "content"): 1,
    ("vckss/CHANGELOG.md", TOKENS[0], "content"): 4,
    (BASELINE_FAILURE_REL, TOKENS[0], "content"): 1,
    (V2_EQ_DRIVER_REL, TOKENS[0], "content"): 1,
    (V2_EQ_RUNNER_REL, TOKENS[0], "content"): 13,
    (V2_EQ_RUNNER_REL, TOKENS[1], "content"): 1,
    (V2_EQ_BASELINE_MANIFEST_REL, TOKENS[0], "content"): 1_075,
    (V2_EQ_BASELINE_JSON_REL, TOKENS[0], "content"): 90,
    (V2_EQ_BASELINE_JSON_REL, TOKENS[1], "content"): 7,
    (V2_EQ_BASELINE_TSV_REL, TOKENS[0], "content"): 90,
    (V2_EQ_BASELINE_TSV_REL, TOKENS[1], "content"): 7,
    (V2_EQ_RECEIPT_REL, TOKENS[0], "content"): 2,
    (V2_EQ_RECEIPT_REL, TOKENS[1], "content"): 1,
}

V1_FROZEN_PREFIXES = (
    "reviews/",
    "varcomp_kss/benchmarks/reports/",
    "varcomp_kss/cmg/benchmarks/reports/",
    "docs/history/",
)
V1_FROZEN_EXACT = {
    "docs/migration/README.md",
    "varcomp_kss/cmg/docs/UPSTREAM_SOURCE_MANIFEST.yaml",
}
V1_OPTIONAL_FROZEN_EXACT = {"docs/migration/ppml-variance-commit-map.tsv"}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_candidates() -> list[str]:
    result = subprocess.run(
        [
            "git",
            "-C",
            str(REPO_ROOT),
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        check=True,
        stdout=subprocess.PIPE,
    )
    paths = {item.decode("utf-8") for item in result.stdout.split(b"\0") if item}
    return sorted(path for path in paths if (REPO_ROOT / path).is_file())


def git_tree_paths(commit: str) -> set[str]:
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "ls-tree", "-r", "--name-only", "-z", commit],
        check=True,
        stdout=subprocess.PIPE,
    )
    return {item.decode("utf-8") for item in result.stdout.split(b"\0") if item}


def _safe_relative(path: object, label: str) -> str:
    if not isinstance(path, str):
        raise ValueError(f"{label} is not a string: {path!r}")
    pure = PurePosixPath(path)
    if pure.is_absolute() or ".." in pure.parts:
        raise ValueError(f"unsafe {label}: {path}")
    return path


def _load_json(relative: str) -> dict[str, object]:
    path = REPO_ROOT / relative
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read {relative}: {exc}") from exc
    if not isinstance(document, dict):
        raise ValueError(f"{relative} is not a JSON object")
    return document


def load_v1_inventory() -> dict[str, dict[str, object]]:
    document = _load_json(V1_INVENTORY_REL)
    if document.get("schema_version") != 1 or document.get("algorithm") != "sha256":
        raise ValueError("unsupported frozen inventory schema or algorithm")
    if document.get("predecessor_commit") != PREDECESSOR_COMMIT:
        raise ValueError("frozen inventory has the wrong predecessor commit")
    expected_scope = {
        "prefixes": list(V1_FROZEN_PREFIXES),
        "required_exact": sorted(V1_FROZEN_EXACT),
        "optional_exact": sorted(V1_OPTIONAL_FROZEN_EXACT),
    }
    if document.get("scope") != expected_scope:
        raise ValueError("frozen inventory scope does not match its v1 policy")
    records = document.get("files")
    if not isinstance(records, list):
        raise ValueError("frozen inventory files must be a list")
    by_path: dict[str, dict[str, object]] = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("frozen inventory contains a non-object record")
        relative = _safe_relative(record.get("path"), "frozen path")
        if relative in by_path:
            raise ValueError(f"duplicate frozen path: {relative}")
        by_path[relative] = record
    if list(by_path) != sorted(by_path):
        raise ValueError("frozen inventory records are not path-sorted")
    return by_path


def load_v2_inventory() -> dict[str, dict[str, object]]:
    document = _load_json(V2_INVENTORY_REL)
    if document.get("schema_version") != 2 or document.get("algorithm") != "sha256":
        raise ValueError("unsupported relocation inventory schema or algorithm")
    if document.get("source_commit") != SOURCE_COMMIT:
        raise ValueError("relocation inventory has the wrong source commit")
    if document.get("receipt_tip_commit") != RECEIPT_TIP_COMMIT:
        raise ValueError("relocation inventory has the wrong receipt tip")
    records = document.get("files")
    if not isinstance(records, list):
        raise ValueError("relocation inventory files must be a list")
    by_original: dict[str, dict[str, object]] = {}
    new_paths: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("relocation inventory contains a non-object record")
        original = _safe_relative(record.get("original_path"), "original path")
        relocated = _safe_relative(record.get("new_path"), "relocated path")
        if original in by_original:
            raise ValueError(f"duplicate original path: {original}")
        if relocated in new_paths:
            raise ValueError(f"duplicate relocated path: {relocated}")
        size = record.get("size")
        digest = record.get("sha256")
        frozen = record.get("byte_identity_required")
        if not isinstance(size, int) or size < 0:
            raise ValueError(f"invalid relocation size: {original}")
        if not isinstance(digest, str) or len(digest) != 64:
            raise ValueError(f"invalid relocation SHA-256: {original}")
        if not isinstance(frozen, bool):
            raise ValueError(f"invalid byte-identity flag: {original}")
        by_original[original] = record
        new_paths.add(relocated)
    if list(by_original) != sorted(by_original):
        raise ValueError("relocation inventory records are not path-sorted")
    return by_original


def _in_v1_scope(path: str) -> bool:
    return (
        path in V1_FROZEN_EXACT
        or path in V1_OPTIONAL_FROZEN_EXACT
        or any(path.startswith(prefix) for prefix in V1_FROZEN_PREFIXES)
    )


def verify_inventory(candidates: list[str]) -> tuple[set[str], list[str]]:
    errors: list[str] = []
    try:
        v1 = load_v1_inventory()
        v2 = load_v2_inventory()
        tip_paths = git_tree_paths(RECEIPT_TIP_COMMIT)
    except (OSError, ValueError, subprocess.CalledProcessError, UnicodeDecodeError) as exc:
        return set(), [str(exc)]

    if set(v2) != tip_paths:
        for relative in sorted(tip_paths - set(v2)):
            errors.append(f"receipt-tip file missing from relocation inventory: {relative}")
        for relative in sorted(set(v2) - tip_paths):
            errors.append(f"relocation inventory path absent at receipt tip: {relative}")

    expected_v1 = {path for path in tip_paths if _in_v1_scope(path)}
    if set(v1) != expected_v1:
        for relative in sorted(expected_v1 - set(v1)):
            errors.append(f"frozen v1 file missing from inventory: {relative}")
        for relative in sorted(set(v1) - expected_v1):
            errors.append(f"frozen v1 path absent or outside scope: {relative}")

    candidate_set = set(candidates)
    frozen_current: set[str] = set()
    for original, record in v2.items():
        relocated = str(record["new_path"])
        if relocated not in candidate_set:
            errors.append(f"relocated path is absent: {original} -> {relocated}")
            continue
        if not bool(record["byte_identity_required"]):
            continue
        frozen_current.add(relocated)
        path = REPO_ROOT / relocated
        actual_size = path.stat().st_size
        expected_size = int(record["size"])
        if actual_size != expected_size:
            errors.append(
                f"relocated frozen size mismatch: {relocated}: "
                f"{actual_size} != {expected_size}"
            )
            continue
        actual_hash = sha256(path)
        expected_hash = str(record["sha256"])
        if actual_hash != expected_hash:
            errors.append(
                f"relocated frozen hash mismatch: {relocated}: "
                f"{actual_hash} != {expected_hash}"
            )

    for original, record in v1.items():
        relocation = v2.get(original)
        if relocation is None:
            errors.append(f"frozen v1 path lacks a relocation record: {original}")
            continue
        if not bool(relocation["byte_identity_required"]):
            errors.append(f"frozen v1 path is not byte-locked by v2: {original}")
        if record.get("size") != relocation.get("size"):
            errors.append(f"frozen v1/v2 size mismatch: {original}")
        if record.get("sha256") != relocation.get("sha256"):
            errors.append(f"frozen v1/v2 SHA-256 mismatch: {original}")

    return frozen_current, errors


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
    for relative in candidates:
        if relative in frozen:
            continue
        findings += check_occurrences(relative, "path", relative, errors)
        text = decode_text(REPO_ROOT / relative)
        if text is None:
            continue
        text_files += 1
        findings += check_occurrences(relative, "content", text, errors)

    if errors:
        print("LEGACY NAME AUDIT FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        "LEGACY NAME AUDIT PASS: "
        f"{len(frozen)} relocated frozen files verified; "
        f"{text_files} active text files scanned; "
        f"{findings} count-bounded migration/provenance occurrences"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
