from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

from vckss.benchmarks.comparative_scaling.build_benchmark_ado import (
    THREAD_CONTRACT,
    build,
)


REPO = Path(__file__).resolve().parents[4]
COMPARISON = "427063bd3ba982d044f6f5b949cf8910ef67ec2d"


def comparison_ado(tmp_path: Path) -> Path:
    path = tmp_path / "comparison-vckss.ado"
    path.write_bytes(subprocess.run(
        ("git", "show", f"{COMPARISON}:vckss/vckss.ado"),
        cwd=REPO, check=True, stdout=subprocess.PIPE,
    ).stdout)
    return path


@pytest.mark.parametrize("source_kind", ("candidate", "comparison"))
def test_adapter_transforms_candidate_and_checkpoint_once(
    tmp_path: Path, source_kind: str,
) -> None:
    source = (REPO / "vckss" / "vckss.ado" if source_kind == "candidate"
              else comparison_ado(tmp_path))
    output = tmp_path / f"{source_kind}.benchmark.ado"
    receipt = tmp_path / f"{source_kind}.json"
    value = build(source, output, receipt)
    text = output.read_text(encoding="utf-8")
    assert value["thread_contract"] == THREAD_CONTRACT
    assert value["maximum_stata_processors"] == 4
    assert value["allowed_native_threads"] == [1, 2, 4, 8, 16]
    assert text.count("VCKSS_BENCHMARK_THREAD_CONTRACT") == 1
    assert text.count("threads(`full_cmg_threads')") == 1
    assert text.count("`cmg_threads_requested'==`full_cmg_threads'") == 1
    assert text.count("`cmg_threads_used'==`full_cmg_threads'") == 1
    assert "threads(`=c(processors)')" not in text
    assert json.loads(receipt.read_text(encoding="utf-8"))["status"] == "PASS"


def test_adapter_rejects_already_adapted_source(tmp_path: Path) -> None:
    first = tmp_path / "first.ado"
    build(REPO / "vckss" / "vckss.ado", first, tmp_path / "first.json")
    with pytest.raises(ValueError, match="expected exactly one"):
        build(first, tmp_path / "second.ado", tmp_path / "second.json")


def test_adapter_decouples_native_threads_in_stata(tmp_path: Path) -> None:
    stata = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
    if not stata.is_file():
        pytest.skip("local Stata/MP is unavailable")
    package_value = os.environ.get("VCKSS_BENCHMARK_TEST_PACKAGE")
    if not package_value:
        pytest.skip("qualified macOS benchmark-test package is unavailable")
    package = Path(package_value)
    if not (package / "vckss_rust_macos_arm64.plugin").is_file():
        pytest.fail("benchmark-test package omits the qualified arm64 plugin")
    adapted = tmp_path / "vckss.ado"
    build(REPO / "vckss" / "vckss.ado", adapted, tmp_path / "receipt.json")
    driver = Path(__file__).with_name("stata_benchmark_adapter.do")
    environment = os.environ.copy()
    environment.update({
        "VCKSS_BENCHMARK_THREAD_CONTRACT": THREAD_CONTRACT,
        "VCKSS_BENCHMARK_RUST_THREADS": "8",
        "VCKSS_BENCHMARK_ACTIVE_CORES": "8",
        "VCKSS_BENCHMARK_ASSIGNED_SLOTS": "16",
        "OMP_NUM_THREADS": "8",
        "RAYON_NUM_THREADS": "8",
        "CMG_THREADS": "8",
    })
    result = subprocess.run(
        (str(stata), "-q", "do", str(driver), str(package),
         str(adapted)),
        cwd=REPO, env=environment, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=180, check=False,
    )
    assert result.returncode == 0, result.stdout
    assert "VCKSS BENCHMARK ADAPTER STATA PASS" in result.stdout
