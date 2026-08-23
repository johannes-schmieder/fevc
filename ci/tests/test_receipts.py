from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


CI_DIR = Path(__file__).resolve().parents[1]
TESTED_SHA = "a" * 40


class ReceiptTests(unittest.TestCase):
    def build_receipt(
        self,
        *,
        process: dict[str, object],
        status: str | None,
        required_outputs: list[str] | None = None,
    ) -> tuple[subprocess.CompletedProcess[str], dict[str, object]]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            process_path = root / "process.json"
            process_path.write_text(json.dumps(process), encoding="utf-8")
            status_path = root / "stata.status"
            if status is not None:
                status_path.write_text(status, encoding="utf-8")
            config_path = root / "profiles.json"
            config_path.write_text(
                json.dumps(
                    {
                        "profiles": {
                            "smoke": {
                                "suite": "ci/stata_smoke.do",
                                "required_outputs": required_outputs or [],
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            receipt_path = root / "receipt.json"
            command = [
                sys.executable,
                str(CI_DIR / "make_stata_receipt.py"),
                "--profile-config",
                str(config_path),
                "--profile",
                "smoke",
                "--process-json",
                str(process_path),
                "--status-file",
                str(status_path),
                "--run-dir",
                str(root),
                "--receipt",
                str(receipt_path),
                "--tested-sha",
                TESTED_SHA,
                "--stata-executable",
                "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp",
            ]
            completed = subprocess.run(command, check=False, text=True, capture_output=True)
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            return completed, receipt

    @staticmethod
    def normal_process(**overrides: object) -> dict[str, object]:
        value: dict[str, object] = {
            "started_at": "2026-01-01T00:00:00Z",
            "completed_at": "2026-01-01T00:00:01Z",
            "duration_seconds": 1.0,
            "launch_error": None,
            "timed_out": False,
            "process_rc": 0,
        }
        value.update(overrides)
        return value

    @staticmethod
    def status(stata_rc: int) -> str:
        return (
            "schema_version=1\n"
            "profile=smoke\n"
            f"stata_rc={stata_rc}\n"
            "stata_version=18\n"
            "stata_edition=MP\n"
            "stata_os=Unix\n"
            "stata_machine_type=Mac (Apple Silicon)\n"
            "stata_processors=8\n"
            "completed=1\n"
        )

    def test_success(self) -> None:
        completed, receipt = self.build_receipt(
            process=self.normal_process(), status=self.status(0)
        )
        self.assertEqual(completed.returncode, 0)
        self.assertEqual(receipt["status"], "success")
        self.assertIsNone(receipt["failure_kind"])
        self.assertEqual(receipt["tested_sha"], TESTED_SHA)

    def test_stata_error_wins_over_zero_process_rc(self) -> None:
        _, receipt = self.build_receipt(
            process=self.normal_process(process_rc=0), status=self.status(9)
        )
        self.assertEqual(receipt["status"], "failure")
        self.assertEqual(receipt["failure_kind"], "stata_error")
        self.assertEqual(receipt["stata_rc"], 9)

    def test_launch_error(self) -> None:
        _, receipt = self.build_receipt(
            process=self.normal_process(
                process_rc=None, launch_error="FileNotFoundError: missing Stata"
            ),
            status=None,
        )
        self.assertEqual(receipt["failure_kind"], "launch_error")
        self.assertIsNone(receipt["stata_rc"])

    def test_timeout(self) -> None:
        _, receipt = self.build_receipt(
            process=self.normal_process(process_rc=-15, timed_out=True), status=None
        )
        self.assertEqual(receipt["failure_kind"], "timeout")

    def test_missing_status(self) -> None:
        _, receipt = self.build_receipt(process=self.normal_process(), status=None)
        self.assertEqual(receipt["failure_kind"], "missing_status")

    def test_crash_without_status(self) -> None:
        _, receipt = self.build_receipt(
            process=self.normal_process(process_rc=-9), status=None
        )
        self.assertEqual(receipt["failure_kind"], "crash")

    def test_missing_required_output(self) -> None:
        _, receipt = self.build_receipt(
            process=self.normal_process(),
            status=self.status(0),
            required_outputs=["output/result.txt"],
        )
        self.assertEqual(receipt["failure_kind"], "missing_result")


if __name__ == "__main__":
    unittest.main()
