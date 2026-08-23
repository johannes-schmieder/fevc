from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


CI_DIR = Path(__file__).resolve().parents[1]
TESTED_SHA = "b" * 40


class FinalizeStataReceiptTests(unittest.TestCase):
    def run_finalizer(
        self,
        root: Path,
        *,
        receipt_tests: str = "success",
        licensed_profile: str = "success",
        rust_quick: str = "success",
        rust_required: bool = True,
    ) -> subprocess.CompletedProcess[str]:
        profile_config = root / "profiles.json"
        profile_config.write_text(
            json.dumps(
                {
                    "profiles": {
                        "quick": {
                            "suite": "varcomp_kss/tests/stata/run_all.do",
                            "required_outputs": ["output/required.txt"],
                        }
                    }
                }
            ),
            encoding="utf-8",
        )
        command = [
            sys.executable,
            str(CI_DIR / "finalize_stata_receipt.py"),
            "--receipt",
            str(root / "run" / "receipt.json"),
            "--profile-config",
            str(profile_config),
            "--profile",
            "quick",
            "--tested-sha",
            TESTED_SHA,
            "--repository",
            "johannes-schmieder/varcomp_kss",
            "--ref",
            "refs/heads/codex/test",
            "--run-id",
            "123",
            "--run-attempt",
            "1",
            "--runner-name",
            "macstudio-stata-mp18-varcomp-kss",
            "--stata-executable",
            "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp",
            "--resolve-profile-outcome",
            "success",
            "--receipt-tests-outcome",
            receipt_tests,
            "--licensed-profile-outcome",
            licensed_profile,
            "--rust-quick-outcome",
            rust_quick,
        ]
        if rust_required:
            command.append("--rust-quick-required")
        return subprocess.run(command, check=False, text=True, capture_output=True)

    @staticmethod
    def valid_receipt() -> dict[str, object]:
        return {
            "schema_version": 1,
            "tested_sha": TESTED_SHA,
            "profile": "quick",
            "status": "success",
            "process_rc": 0,
            "stata_rc": 0,
            "run_id": "123",
            "platform": "Unix; Mac (Apple Silicon)",
        }

    def test_pre_stata_failure_synthesizes_exact_sha_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            completed = self.run_finalizer(
                root,
                receipt_tests="failure",
                licensed_profile="skipped",
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            receipt = json.loads(
                (root / "run" / "receipt.json").read_text(encoding="utf-8")
            )
            self.assertEqual(receipt["tested_sha"], TESTED_SHA)
            self.assertEqual(receipt["profile"], "quick")
            self.assertEqual(receipt["status"], "failure")
            self.assertEqual(receipt["failure_kind"], "pre_stata_gate")
            self.assertIsNone(receipt["stata_rc"])
            self.assertIsNone(receipt["process_rc"])
            self.assertTrue(receipt["synthetic_receipt"])
            self.assertEqual(
                receipt["ci_step_outcomes"]["receipt_tests"], "failure"
            )
            self.assertEqual(
                receipt["required_outputs"],
                [{"path": "output/required.txt", "present": False}],
            )

    def test_existing_valid_receipt_is_preserved_byte_for_byte(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt_path = root / "run" / "receipt.json"
            receipt_path.parent.mkdir(parents=True)
            original = json.dumps(self.valid_receipt(), indent=1) + "\n"
            receipt_path.write_text(original, encoding="utf-8")
            completed = self.run_finalizer(root)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(receipt_path.read_text(encoding="utf-8"), original)
            self.assertIn("preserved", completed.stdout)

    def test_malformed_receipt_is_replaced_with_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt_path = root / "run" / "receipt.json"
            receipt_path.parent.mkdir(parents=True)
            receipt_path.write_text("not-json\n", encoding="utf-8")
            completed = self.run_finalizer(
                root,
                licensed_profile="failure",
                rust_quick="skipped",
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            self.assertEqual(receipt["status"], "failure")
            self.assertEqual(receipt["failure_kind"], "ci_harness_error")
            self.assertIn("receipt could not be read", receipt["failure_detail"])

    def test_manual_profile_does_not_require_rust_step(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            completed = self.run_finalizer(
                root,
                licensed_profile="skipped",
                rust_quick="skipped",
                rust_required=False,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            receipt = json.loads(
                (root / "run" / "receipt.json").read_text(encoding="utf-8")
            )
            self.assertEqual(receipt["failure_kind"], "licensed_profile_skipped")


if __name__ == "__main__":
    unittest.main()
