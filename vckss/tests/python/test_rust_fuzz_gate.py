from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
FUZZ_ROOT = ROOT / "rust/fuzz"
SCRIPT = ROOT / "rust/tools/run_fuzz_checks.sh"


def test_fuzz_gate_is_executable_and_shell_valid() -> None:
    subprocess.run(["bash", "-n", str(SCRIPT)], check=True)
    assert SCRIPT.stat().st_mode & 0o111


def test_fuzz_target_is_registered_and_bounded() -> None:
    manifest = (FUZZ_ROOT / "Cargo.toml").read_text(encoding="utf-8")
    target = (
        FUZZ_ROOT / "fuzz_targets/request_capability.rs"
    ).read_text(encoding="utf-8")
    assert 'name = "request_capability"' in manifest
    assert 'libfuzzer-sys = "0.4"' in manifest
    for required in (
        "vckss_rust_backend_request_capability_v1",
        "vckss_rust_backend_request_capability_v2",
        "vckss_rust_backend_request_capability_v3",
        "ptr::null()",
        "ptr::null_mut()",
        "capacity::<VckssBackendRequestCapabilityReceiptV3>()",
        "f64::from_bits",
    ):
        assert required in target


def test_fuzz_qualifier_is_source_bound_and_ephemeral() -> None:
    source = SCRIPT.read_text(encoding="utf-8")
    for required in (
        "VCKSS-FUZZ-QUALIFICATION-V1",
        "nightly-2026-08-23",
        "cargo-fuzz 0.12.0",
        "cargo-clippy",
        "--locked --all-targets -- -D warnings",
        "status --porcelain --untracked-files=all",
        "-max_total_time=",
        "-timeout=10",
        "-max_len=256",
        "-seed=20260825",
        "-verbosity=0",
        "stat::number_of_executed_units",
        "executed_units=%s",
        "average_exec_per_second=%s",
        "peak_rss_mb=%s",
        "artifact_count=0",
        "seed-corpus.sha256",
        "temporary corpus growth, build products, and artifacts are deleted",
    ):
        assert required in source


def test_fuzz_seed_corpus_is_tracked_and_nonempty() -> None:
    seeds = sorted((FUZZ_ROOT / "corpus/request_capability").iterdir())
    assert len(seeds) >= 3
    assert all(seed.is_file() and seed.stat().st_size > 0 for seed in seeds)
