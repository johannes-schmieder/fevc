#!/usr/bin/env python3
"""Reject the former VCKSS public identity while allowing private internals."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

HISTORICAL_PREFIXES = (
    ".ci/stata/",
    "docs/history/",
    "docs/migration/",
    "fevc/qualification/fevc_rename_equivalence/",
    "reviews/",
    "rust/experiments/",
    "rust/qualification/evidence/",
)
HISTORICAL_EXACT = {
    "fevc/CHANGELOG.md",
    "fevc/PLAN.md",
    "fevc/tools/check_fevc_public_identity.py",
    "fevc/tests/python/test_fevc_public_identity.py",
    "rust/SOURCE_PROVENANCE.md",
    "rust/vendor/cmg/VENDOR.md",
    "run_all.log",
}
OPERATIONAL_EXACT = {
    ".github/workflows/stata-ci.yml",
    "STATA_CI_RUNNER.md",
    "rust/stata_backend/scc/deploy_linux_bundle.sh",
    "rust/stata_backend/scc/run_linux_qualifier.sge",
    "rust/stata_backend/scc/submit_linux_qualifier.sh",
}
EQUIVALENCE_EXACT = {
    "fevc/tests/equivalence/fevc_equivalence_driver.do",
    "fevc/tools/run_fevc_rename_equivalence.py",
}
PUBLIC_BOUNDARY_FILES = {
    "AGENTS.md",
    "README.md",
    "REUSE.toml",
    "THIRD_PARTY_NOTICES.md",
    "fevc/AGENTS.md",
    "fevc/README.md",
    "fevc/TESTING.md",
    "fevc/THIRD_PARTY_NOTICES.txt",
    "fevc/docs/INFERENCE.md",
    "fevc/fevc.ado",
    "fevc/fevc.sthlp",
}
FORBIDDEN_PUBLIC_BASENAMES = {
    "vckss.ado",
    "vckss.pkg",
    "vckss.sthlp",
    "vckss_run.ado",
    "vckss_rust.ado",
}
LEGACY_DISTRIBUTED_BASENAME = re.compile(
    r"^_?vckss.*\.(?:ado|mata|plugin)$", re.IGNORECASE
)
PUBLIC_PATTERNS = (
    re.compile(r"^\s*program\s+define\s+vckss(?:\s|,|$)"),
    re.compile(r"ereturn\s+local\s+cmd\s+[`\"']?vckss(?:[`\"']|\s|$)"),
    re.compile(
        r"^\s*(?:(?:capture|quietly|noisily)\s+)*vckss(?:\s|,|$)",
    ),
    re.compile(r"\b(?:help|which|viewsource)\s+vckss\b"),
    re.compile(r"github\.com/johannes-schmieder/vckss(?:\.git)?(?:\s|$)", re.IGNORECASE),
)
OLD_BRAND = re.compile(r"\bVCkss\b")
RENAMED_PRIVATE_PROTOCOL = re.compile(
    r"\bFEVC-(?:COUNTER-V1|NATIVE-|EXECUTION-PLAN-V1)"
)
RENAMED_PRIVATE_BUILD_ID = re.compile(
    r"\bfevc-(?:api21-stayer-hybrid|inference-api1-exact-observation)\b"
)


def candidates() -> list[str]:
    completed = subprocess.run(
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
    paths = {item.decode("utf-8") for item in completed.stdout.split(b"\0") if item}
    return sorted(path for path in paths if (REPO_ROOT / path).is_file())


def is_exempt(relative: str) -> bool:
    return (
        relative in HISTORICAL_EXACT
        or relative in OPERATIONAL_EXACT
        or relative in EQUIVALENCE_EXACT
        or any(relative.startswith(prefix) for prefix in HISTORICAL_PREFIXES)
    )


def audit(paths: list[str]) -> list[str]:
    errors: list[str] = []
    for relative in paths:
        if relative == "vckss" or relative.startswith("vckss/"):
            errors.append(f"{relative}: obsolete predecessor working-tree path")
            continue
        if is_exempt(relative):
            continue
        path = REPO_ROOT / relative
        if path.name.lower() in FORBIDDEN_PUBLIC_BASENAMES:
            errors.append(f"{relative}: former public artifact name")
        if (
            path.parent == REPO_ROOT / "fevc"
            and LEGACY_DISTRIBUTED_BASENAME.fullmatch(path.name)
        ):
            errors.append(f"{relative}: legacy distributed runtime filename")
        raw = path.read_bytes()
        if b"\0" in raw[:8192]:
            continue
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            if RENAMED_PRIVATE_BUILD_ID.search(line):
                errors.append(
                    f"{relative}:{line_number}: private VCKSS build ID was renamed"
                )
            if RENAMED_PRIVATE_PROTOCOL.search(line):
                errors.append(
                    f"{relative}:{line_number}: private VCKSS protocol was renamed"
                )
            if any(pattern.search(line) for pattern in PUBLIC_PATTERNS):
                errors.append(
                    f"{relative}:{line_number}: former public command or repository identity"
                )
            if relative in PUBLIC_BOUNDARY_FILES and OLD_BRAND.search(line):
                errors.append(f"{relative}:{line_number}: former public brand")
    return errors


def main() -> int:
    try:
        errors = audit(candidates())
    except (OSError, subprocess.CalledProcessError, UnicodeDecodeError) as exc:
        print(f"FEVC public-identity audit could not run: {exc}", file=sys.stderr)
        return 2
    if errors:
        print("FEVC PUBLIC-IDENTITY AUDIT FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("FEVC PUBLIC-IDENTITY AUDIT PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
