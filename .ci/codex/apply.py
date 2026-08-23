from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(".")


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    source = path.read_text(encoding="utf-8")
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    path.write_text(source.replace(old, new, 1), encoding="utf-8")
    print(f"replaced {label}")


# Do not classify Stata's /// continuation marker as a standalone // comment.
replace_once(
    ROOT / "ci/tests/test_stata_planned_route_syntax.py",
    r"(?m)^\s*//",
    r"(?m)^[ \t]*//(?!/)",
    "standalone Stata comment regex",
)


finalizer = r'''#!/usr/bin/env python3
"""Synthesize an exact-SHA failure receipt when CI stops before Stata.

A receipt created by the Stata/plugin harness is authoritative and is never
overwritten.
"""

from __future__ import annotations

import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
RUN_DIR = Path(".ci/stata/run")
RECEIPT_PATH = RUN_DIR / "receipt.json"
PROFILE_PATH = Path(".ci/stata/requested-profile.txt")
PLUGIN_PROFILES = {"plugin-build", "plugin-load", "numerical-small", "differential"}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def requested_profile() -> str:
    event_path = os.environ.get("GITHUB_EVENT_PATH", "")
    if event_path:
        try:
            event = json.loads(Path(event_path).read_text(encoding="utf-8"))
            candidate = str((event.get("inputs") or {}).get("profile") or "").strip()
            if candidate:
                return candidate
        except (OSError, ValueError, TypeError):
            pass
    for name in ("VCKSS_CI_PROFILE", "STATA_CI_PROFILE", "PROFILE", "INPUT_PROFILE"):
        candidate = os.environ.get(name, "").strip()
        if candidate:
            return candidate
    if PROFILE_PATH.is_file():
        candidate = PROFILE_PATH.read_text(encoding="utf-8").strip()
        if candidate:
            return candidate
    return "quick"


def suite_for(profile: str) -> str:
    if profile in PLUGIN_PROFILES:
        return "rust/stata_backend/qualify_macos.sh"
    return "varcomp_kss/tests/stata/run_all.do"


def build_failure_receipt() -> dict[str, Any]:
    tested_sha = os.environ.get("GITHUB_SHA", "").strip().lower()
    if not SHA_RE.fullmatch(tested_sha):
        raise RuntimeError(
            f"GITHUB_SHA is not a full lowercase commit SHA: {tested_sha!r}"
        )
    profile = requested_profile()
    now = utc_now()
    job_status = os.environ.get("VCKSS_JOB_STATUS", "failure").strip() or "failure"
    return {
        "schema_version": 1,
        "repository": os.environ.get("GITHUB_REPOSITORY", ""),
        "ref": os.environ.get("GITHUB_REF", ""),
        "tested_sha": tested_sha,
        "profile": profile,
        "suite": suite_for(profile),
        "status": "failure",
        "failure_kind": "pre_stata_gate",
        "failure_detail": (
            "Licensed Stata CI completed without a harness receipt. "
            f"The job status at finalization was {job_status}; a pre-Stata gate, "
            "setup step, or harness launch failed. Stata was not certified as executed."
        ),
        "process_rc": 1,
        "stata_rc": None,
        "stata_executed": False,
        "tests_passed": 0,
        "tests_failed": 1,
        "required_outputs": [],
        "started_at": now,
        "completed_at": now,
        "duration_seconds": 0.0,
        "run_id": os.environ.get("GITHUB_RUN_ID", ""),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
        "runner_name": os.environ.get("RUNNER_NAME", ""),
        "platform": "; ".join(
            value
            for value in (
                os.environ.get("RUNNER_OS", ""),
                os.environ.get("RUNNER_ARCH", ""),
            )
            if value
        ),
        "stata_version": None,
        "stata_bundle_version": None,
        "stata_edition": None,
        "stata_executable": None,
        "stata_processors": None,
    }


def finalize() -> bool:
    if RECEIPT_PATH.is_file():
        print(f"preserving authoritative harness receipt at {RECEIPT_PATH}")
        return False
    payload = build_failure_receipt()
    RUN_DIR.mkdir(parents=True, exist_ok=True)
    temporary = RECEIPT_PATH.with_suffix(".json.tmp")
    temporary.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    temporary.replace(RECEIPT_PATH)
    print(
        "synthesized pre-Stata failure receipt for "
        f"{payload['tested_sha']} profile={payload['profile']}"
    )
    return True


if __name__ == "__main__":
    finalize()
'''
finalizer_path = ROOT / "ci/finalize_stata_receipt.py"
if finalizer_path.exists():
    raise RuntimeError("ci/finalize_stata_receipt.py already exists")
finalizer_path.write_text(finalizer, encoding="utf-8")
print("added pre-Stata receipt finalizer")


test_source = r'''from __future__ import annotations

import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "ci" / "finalize_stata_receipt.py"
SPEC = importlib.util.spec_from_file_location("finalize_stata_receipt", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FinalizeStataReceiptTests(unittest.TestCase):
    def base_environment(self) -> dict[str, str]:
        return {
            "GITHUB_SHA": "a" * 40,
            "GITHUB_REPOSITORY": "johannes-schmieder/varcomp_kss",
            "GITHUB_REF": "refs/heads/codex/test",
            "GITHUB_RUN_ID": "12345",
            "GITHUB_RUN_ATTEMPT": "2",
            "RUNNER_NAME": "synthetic-runner",
            "RUNNER_OS": "macOS",
            "RUNNER_ARCH": "ARM64",
            "VCKSS_JOB_STATUS": "failure",
            "VCKSS_CI_PROFILE": "quick",
        }

    def test_synthesizes_exact_sha_failure_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            run_dir = Path(temporary_directory) / "run"
            receipt_path = run_dir / "receipt.json"
            with patch.dict(os.environ, self.base_environment(), clear=True), patch.object(
                MODULE, "RUN_DIR", run_dir
            ), patch.object(MODULE, "RECEIPT_PATH", receipt_path), patch.object(
                MODULE,
                "PROFILE_PATH",
                Path(temporary_directory) / "missing-profile.txt",
            ):
                self.assertTrue(MODULE.finalize())
            payload = json.loads(receipt_path.read_text(encoding="utf-8"))
            self.assertEqual("a" * 40, payload["tested_sha"])
            self.assertEqual("quick", payload["profile"])
            self.assertEqual("failure", payload["status"])
            self.assertEqual("pre_stata_gate", payload["failure_kind"])
            self.assertIsNone(payload["stata_rc"])
            self.assertFalse(payload["stata_executed"])

    def test_preserves_authoritative_harness_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            run_dir = Path(temporary_directory) / "run"
            run_dir.mkdir(parents=True)
            receipt_path = run_dir / "receipt.json"
            original = {"tested_sha": "b" * 40, "status": "success"}
            receipt_path.write_text(json.dumps(original) + "\n", encoding="utf-8")
            with patch.object(MODULE, "RUN_DIR", run_dir), patch.object(
                MODULE, "RECEIPT_PATH", receipt_path
            ):
                self.assertFalse(MODULE.finalize())
            self.assertEqual(
                original,
                json.loads(receipt_path.read_text(encoding="utf-8")),
            )

    def test_rejects_noncanonical_source_sha(self) -> None:
        environment = self.base_environment()
        environment["GITHUB_SHA"] = "not-a-sha"
        with patch.dict(os.environ, environment, clear=True):
            with self.assertRaisesRegex(RuntimeError, "full lowercase commit SHA"):
                MODULE.build_failure_receipt()


if __name__ == "__main__":
    unittest.main()
'''
test_path = ROOT / "ci/tests/test_finalize_stata_receipt.py"
if test_path.exists():
    raise RuntimeError("ci/tests/test_finalize_stata_receipt.py already exists")
test_path.write_text(test_source, encoding="utf-8")
print("added pre-Stata receipt finalizer tests")


workflow_path = ROOT / ".github/workflows/stata-ci.yml"
workflow = workflow_path.read_text(encoding="utf-8")
if "Finalize exact-SHA receipt" in workflow:
    raise RuntimeError("workflow finalization step already exists")
lines = workflow.splitlines(keepends=True)
upload_indices = [
    index
    for index, line in enumerate(lines)
    if line.strip() == "uses: actions/upload-artifact@v4"
]
if len(upload_indices) != 1:
    raise RuntimeError(
        f"expected one actions/upload-artifact@v4 step, found {len(upload_indices)}"
    )
upload_index = upload_indices[0]
step_start = None
for index in range(upload_index, -1, -1):
    if re.match(r"^\s*-\s+(?:name|uses):", lines[index]):
        step_start = index
        break
if step_start is None:
    raise RuntimeError("could not locate the Stata artifact upload step boundary")
indent_match = re.match(r"^(\s*)-", lines[step_start])
assert indent_match is not None
indent = indent_match.group(1)
finalization_block = [
    f"{indent}- name: Finalize exact-SHA receipt\n",
    f"{indent}  if: always()\n",
    f"{indent}  shell: bash\n",
    f"{indent}  env:\n",
    f"{indent}    VCKSS_JOB_STATUS: ${{{{ job.status }}}}\n",
    f"{indent}  run: python3 ci/finalize_stata_receipt.py\n",
    "\n",
]
lines[step_start:step_start] = finalization_block

# The artifact upload must also execute after an earlier gate failure.
upload_index += len(finalization_block)
next_step = len(lines)
for index in range(upload_index + 1, len(lines)):
    if re.match(rf"^{re.escape(indent)}-\s+(?:name|uses):", lines[index]):
        next_step = index
        break
step_block = lines[upload_index:next_step]
if not any(line.strip().startswith("if:") for line in step_block):
    lines.insert(upload_index + 1, f"{indent}  if: always()\n")

workflow_path.write_text("".join(lines), encoding="utf-8")
print("added always-running exact-SHA receipt finalization before artifact upload")
