from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "rust/tools/run_safety_checks.sh"


def test_safety_gate_is_executable_and_shell_valid() -> None:
    subprocess.run(["bash", "-n", str(SCRIPT)], check=True)
    assert SCRIPT.stat().st_mode & 0o111


def test_safety_gate_keeps_bounded_miri_and_sanitizer_contracts() -> None:
    source = SCRIPT.read_text(encoding="utf-8")
    for required in (
        "VCKSS-SAFETY-QUALIFICATION-V1",
        "nightly-2026-08-23",
        "-Zmiri-strict-provenance",
        "-p vckss-plugin --lib",
        "-p vckss-plugin --test context_registry",
        "headers_and_every_output_capacity_fail_before_full_value_access_or_write",
        "v2_memory_admission_fails_before_column_descriptor_access",
        "invalid_and_unknown_callback_contracts_fail_closed",
        "prepare_user_breaks_leave_zero_generation_and_empty_registry",
        "interrupted_stayer_copy_leaves_generation_releasable",
        "-fsanitize=address,undefined",
        "cshim_interrupt_test.c",
        "cshim_error_transport_test.c",
        "abi_header_compat_test.c",
        "native Rust and Stata bitwise/numerical gates remain authoritative",
        "dense spectral solve is impractically slow under interpretation",
    ):
        assert required in source


def test_safety_gate_is_source_bound_and_does_not_overwrite_receipts() -> None:
    source = SCRIPT.read_text(encoding="utf-8")
    assert "symbolic-ref --short HEAD) == main" in source
    assert "status --porcelain --untracked-files=all" in source
    assert "refusing to overwrite safety receipt" in source
    assert "source_commit=$(git -C" in source
    assert "raw_logs=not retained" in source
