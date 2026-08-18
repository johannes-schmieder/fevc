from __future__ import annotations

import math
from pathlib import Path

from kss_bc.benchmarks.matlab_scale.common import sha256_inventory
from kss_bc.benchmarks.scc.verify_numopt2_matlab_source import inventory_hash
from kss_bc.benchmarks.summarize_numopt2_matlab import (
    ADMISSION_HEADROOM,
    HARD_MEMORY_BYTES,
    matlab_pcg_status,
    parse_memory,
)

ROOT = Path(__file__).resolve().parents[3]
SCC = ROOT / "kss_bc/benchmarks/scc"


def test_matlab_comparator_is_one_job_and_descriptive_only() -> None:
    submitter = (SCC / "submit_numopt2_matlab.sh").read_text(encoding="utf-8")
    driver = (SCC / "numopt2_matlab_run.m").read_text(encoding="utf-8")
    assert "qsub_args=(" in submitter
    assert "-t " not in submitter
    assert "hold_jid" not in submitter
    assert "NONE_DESCRIPTIVE_ONLY" in submitter
    assert "target_weight_semantics_comparable',false" in driver
    assert "rng_draws_comparable',false" in driver
    assert "solver_tolerance_comparable',false" in driver


def test_matlab_comparator_uses_same_synthetic_fixture_formula() -> None:
    stata = (SCC / "numopt2_generate.do").read_text(encoding="utf-8")
    matlab = (SCC / "numopt2_matlab_run.m").read_text(encoding="utf-8")
    for fragment in (
        "sin(worker/97)",
        "cos(firm/31)",
        "sin(deletion_unit/113)",
        "ceil(2*`firms'/3)",
    ):
        assert fragment in stata
    for fragment in (
        "sin(worker_chunk/97)",
        "cos(firm_chunk/31)",
        "sin(deletion/113)",
        "ceil(2*firms/3)",
    ):
        assert fragment in matlab
    assert "rows = cells*rows_per_cell" in matlab
    assert "cells = workers*density" in matlab


def _strong_offsets(firms: int, degree: int, layer: int) -> list[int]:
    band = firms // 4
    offsets = [0]
    if degree >= 2:
        offsets.append(1 + layer % (band - 1))
    if degree >= 3:
        offsets.append(math.ceil(firms / 3) + (97 * layer) % band)
    if degree >= 4:
        offsets.append(math.ceil(2 * firms / 3) + (193 * layer) % band)
    if degree >= 5:
        offsets.append(firms - 1)
    if degree >= 6:
        offsets.append(math.ceil(2 * firms / 3) - 1)
    if degree >= 7:
        offsets.append(math.ceil(firms / 3) - 1)
    return offsets


def test_primary_strong_fixture_has_distinct_cells_at_degrees_2_to_7() -> None:
    for firms in (16, 32, 64, 256, 1024):
        for degree in range(2, 8):
            for layer in range(40):
                offsets = _strong_offsets(firms, degree, layer)
                assert len(offsets) == degree
                assert len(set(offsets)) == degree
                assert min(offsets) >= 0
                assert max(offsets) < firms

    stata = (SCC / "numopt2_generate.do").read_text(encoding="utf-8")
    matlab = (SCC / "numopt2_matlab_run.m").read_text(encoding="utf-8")
    for fragment in (
        "ceil(2*`firms'/3)-1 if cell_slot == 6",
        "ceil(`firms'/3)-1 if cell_slot == 7",
    ):
        assert fragment in stata
    for fragment in (
        "ceil(2*firms/3)-1;",
        "ceil(firms/3)-1;",
    ):
        assert fragment in matlab


def test_matlab_comparator_files_are_in_scale_bundle() -> None:
    allowlist = set(
        (ROOT / "kss_bc/benchmarks/scale_bundle_allowlist.txt")
        .read_text(encoding="utf-8")
        .splitlines()
    )
    required = {
        "kss_bc/benchmarks/matlab_scale/common.py",
        "kss_bc/benchmarks/matlab_scale/monitor_process_tree.py",
        "kss_bc/benchmarks/matlab_scale/source_contract.json",
        "kss_bc/benchmarks/scc/numopt2_matlab_run.m",
        "kss_bc/benchmarks/scc/run_numopt2_matlab.sge",
        "kss_bc/benchmarks/scc/submit_numopt2_matlab.sh",
        "kss_bc/benchmarks/scc/validate_numopt2_matlab.py",
        "kss_bc/benchmarks/scc/verify_numopt2_matlab_source.py",
    }
    assert required <= allowlist


def test_matlab_summary_uses_binary_memory_units() -> None:
    assert parse_memory("1.5G") == round(1.5 * 1024**3)
    assert ADMISSION_HEADROOM == 0.20
    assert HARD_MEMORY_BYTES == 128 * 1024**3


def test_matlab_summary_distinguishes_pcg_convergence(tmp_path: Path) -> None:
    converged = tmp_path / "converged.txt"
    converged.write_text(
        "pcg converged at iteration 19 to a solution with relative residual "
        "7e-11.\n",
        encoding="utf-8",
    )
    stopped = tmp_path / "stopped.txt"
    stopped.write_text(
        "pcg stopped at iteration 1000 without converging to the desired "
        "tolerance 1e-10 because the maximum number of iterations was reached.\n"
        "The iterate returned (number 996) has relative residual 5.7e-07.\n",
        encoding="utf-8",
    )
    assert matlab_pcg_status(converged) == {
        "converged": True,
        "termination_iteration": 19,
        "returned_iteration": 19,
        "relative_residual": 7e-11,
    }
    assert matlab_pcg_status(stopped) == {
        "converged": False,
        "termination_iteration": 1000,
        "returned_iteration": 996,
        "relative_residual": 5.7e-7,
    }


def test_matlab_summary_has_no_corrected_estimate_equality_gate() -> None:
    source = (
        ROOT / "kss_bc/benchmarks/summarize_numopt2_matlab.py"
    ).read_text(encoding="utf-8")
    assert "NONE_DESCRIPTIVE_ONLY" in source
    assert "matlab_over_kss_command_ratio" in source
    assert "target_weight_semantics_comparable" in source
    assert "dimension_scale_from_reference" in source
    assert "corrected_total_abs_gap_descriptive" in source
    assert "comparison_source_commits" in source
    assert "matlab_numerical_result_not_accepted_experiments" in source


def test_source_verifier_matches_registered_inventory_framing(tmp_path: Path) -> None:
    (tmp_path / "nested").mkdir()
    (tmp_path / "a.txt").write_text("alpha\n", encoding="utf-8")
    (tmp_path / "nested/b.txt").write_text("beta\n", encoding="utf-8")
    paths = ["a.txt", "nested/b.txt"]
    assert inventory_hash(tmp_path, paths) == sha256_inventory(tmp_path, paths)
