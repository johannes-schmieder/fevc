from __future__ import annotations

import ast
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCC = ROOT / "varcomp_kss" / "benchmarks" / "scc"


def test_cz18_matlab_harness_sources_parse() -> None:
    for name in ("run_cmg_cz18_matlab.sge",
                 "submit_cmg_cz18_matlab.sh"):
        subprocess.run(["bash", "-n", str(SCC / name)], check=True)
    ast.parse((SCC / "validate_cmg_cz18_matlab.py").read_text())


def test_cz18_matlab_task_is_matched_p20_four_core() -> None:
    submit = (SCC / "submit_cmg_cz18_matlab.sh").read_text()
    runner = (SCC / "run_cmg_cz18_matlab.sge").read_text()
    driver = (SCC / "cmg_cz18_matlab_run.m").read_text()
    assert 'test "$probes" = 20' in submit
    assert 'test "$seed" = 8675309' in submit
    assert "requested_slots=4" in submit
    assert "matlab_pool_workers\\t4" in submit
    assert 'test "$CMG_CZ_M_PROBES" = 20' in runner
    assert 'test "$CMG_CZ_M_REQUESTED_SLOTS" = 4' in runner
    assert "pool = parpool(cluster,4,'IdleTimeout',Inf);" in driver
    assert "probes == 20 && seed == 8675309" in driver
    assert "same_probe_count',true" in driver


def test_cz18_matlab_harness_binds_same_input_and_mata_boundary() -> None:
    prepare = (SCC / "cmg_cz18_matlab_prepare.do").read_text()
    runner = (SCC / "run_cmg_cz18_matlab.sge").read_text()
    submit = (SCC / "submit_cmg_cz18_matlab.sh").read_text()
    validator = (SCC / "validate_cmg_cz18_matlab.py").read_text()
    assert "8201888" in prepare and "311730" in prepare
    assert 'sha256sum "$CMG_CZ_M_INPUT_DTA"' in runner
    assert "verify_numopt2_matlab_source.py" in runner
    assert "compile official C MEX; Stata implementation remains Mata-only" in submit
    assert 'stata_summary["input_sha256"] == args.expected_input_sha256' in validator
    assert 'task["probes"] == "20"' in validator
    assert 'task["requested_slots"] == task["matlab_pool_workers"] == "4"' in validator


def test_cz18_matlab_harness_is_bundled() -> None:
    allowlist = (ROOT / "varcomp_kss" / "benchmarks" /
                 "scale_bundle_allowlist.txt").read_text().splitlines()
    expected = {
        "varcomp_kss/benchmarks/scc/cmg_cz18_matlab_prepare.do",
        "varcomp_kss/benchmarks/scc/cmg_cz18_matlab_run.m",
        "varcomp_kss/benchmarks/scc/run_cmg_cz18_matlab.sge",
        "varcomp_kss/benchmarks/scc/submit_cmg_cz18_matlab.sh",
        "varcomp_kss/benchmarks/scc/validate_cmg_cz18_matlab.py",
    }
    assert expected <= set(allowlist)
