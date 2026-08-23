from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


CI_DIR = Path(__file__).resolve().parents[1]
TESTED_SHA = "b" * 40


class PluginReceiptTests(unittest.TestCase):
    def build(
        self,
        *,
        process_rc: int = 0,
        qualification: str | None = None,
        timed_out: bool = False,
    ) -> dict[str, object]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            process = root / "process.json"
            process.write_text(
                json.dumps(
                    {
                        "started_at": "2026-01-01T00:00:00Z",
                        "completed_at": "2026-01-01T00:00:01Z",
                        "duration_seconds": 1.0,
                        "process_rc": process_rc,
                        "timed_out": timed_out,
                        "launch_error": None,
                    }
                ),
                encoding="utf-8",
            )
            process_log = root / "process.log"
            process_log.write_text(
                "macOS Rust plugin qualification failed: synthetic failure\n",
                encoding="utf-8",
            )
            qualification_path = root / "qualification.txt"
            if qualification is not None:
                qualification_path.write_text(qualification, encoding="utf-8")
            receipt = root / "receipt.json"
            command = [
                sys.executable,
                str(CI_DIR / "make_plugin_receipt.py"),
                "--profile",
                "plugin-load",
                "--process-json",
                str(process),
                "--process-log",
                str(process_log),
                "--qualification",
                str(qualification_path),
                "--receipt",
                str(receipt),
                "--tested-sha",
                TESTED_SHA,
                "--stata-executable",
                "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp",
            ]
            completed = subprocess.run(command, check=False, capture_output=True, text=True)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            return json.loads(receipt.read_text(encoding="utf-8"))

    @staticmethod
    def successful_qualification(commit: str = TESTED_SHA) -> str:
        return (
            "VCKSS_MACOS_CANDIDATE_RECEIPT_V1\n"
            "classification=CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION\n"
            f"commit={commit}\n"
            "dirty_status_start=clean\n"
            "dirty_status_end=clean\n"
            "host_macos=26.5.2\n"
            "stata_arm64_environment=VCKSS_STATA_ENV version=18 edition=MP "
            "os=MacOSX machine=Mac (Apple Silicon)\n"
            "cargo_fmt=PASS\n"
            "cargo_clippy=PASS\n"
            "cargo_test=PASS\n"
            "cshim_interrupt_test=PASS\n"
            "cshim_error_transport_test=PASS\n"
            "abi_header_compat_test=PASS\n"
            "rosetta_status=AVAILABLE\n"
            "arm64_test_status=PASS_NATIVE\n"
            "artifact_universal_file=Mach-O universal binary\n"
            "universal continuation without equals\n"
        )

    def test_success(self) -> None:
        receipt = self.build(qualification=self.successful_qualification())
        self.assertEqual(receipt["status"], "success")
        self.assertEqual(receipt["tested_sha"], TESTED_SHA)
        self.assertEqual(receipt["stata_rc"], 0)
        self.assertEqual(receipt["qualification"]["cargo_clippy"], "PASS")

    def test_plugin_failure(self) -> None:
        receipt = self.build(process_rc=1)
        self.assertEqual(receipt["failure_kind"], "plugin_error")
        self.assertEqual(receipt["failure_detail"], "synthetic failure")

    def test_missing_qualification(self) -> None:
        receipt = self.build()
        self.assertEqual(receipt["failure_kind"], "missing_result")

    def test_identity_mismatch(self) -> None:
        receipt = self.build(qualification=self.successful_qualification("c" * 40))
        self.assertEqual(receipt["failure_kind"], "identity_mismatch")

    def test_missing_pass_evidence(self) -> None:
        qualification = self.successful_qualification().replace("cargo_test=PASS\n", "")
        receipt = self.build(qualification=qualification)
        self.assertEqual(receipt["failure_kind"], "invalid_result")

    def test_timeout_wins(self) -> None:
        receipt = self.build(process_rc=-15, timed_out=True)
        self.assertEqual(receipt["failure_kind"], "timeout")


if __name__ == "__main__":
    unittest.main()
