from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCC = ROOT / "vckss/benchmarks/scc"
ALLOWLIST = ROOT / "vckss/benchmarks/scale_bundle_allowlist.txt"


def test_hierarchy_scc_shell_entrypoints_are_valid_and_source_bound() -> None:
    runner = SCC / "run_cmg_hierarchy.sge"
    submitter = SCC / "submit_cmg_hierarchy.sh"
    subprocess.run(["bash", "-n", str(runner), str(submitter)], check=True)
    source = runner.read_text(encoding="utf-8")
    for token in (
        "sha256sum -c BUNDLE_FILES.sha256",
        "CMG_H_STATA_PROCESSORS",
        "degree_hierarchy_matrix.do",
        "hierarchy_benchmark.do",
        "CMG_HIERARCHY_WRAPPER_PASS",
    ):
        assert token in source
    submit = submitter.read_text(encoding="utf-8")
    assert "qsub -verify" in submit
    assert "qsub -terse" in submit
    assert "-P welfgr" in submit


def test_hierarchy_scc_files_and_robust_families_are_bundled() -> None:
    allowlist = set(ALLOWLIST.read_text(encoding="utf-8").splitlines())
    assert {
        "vckss/benchmarks/scc/run_cmg_hierarchy.sge",
        "vckss/benchmarks/scc/submit_cmg_hierarchy.sh",
        "vckss/benchmarks/scc/validate_cmg_hierarchy.py",
        "vckss/cmg/benchmarks/hierarchy_benchmark.do",
    } <= allowlist
    benchmark = (
        ROOT / "vckss/cmg/benchmarks/hierarchy_benchmark.do"
    ).read_text(encoding="utf-8")
    for family in (
        "path", "ring", "star", "irregular", "expander", "tied",
        "oneheavy", "logspread", "barbell", "lollipop",
        "cluster_chain", "disconnected", "singleton",
    ):
        assert f'"{family}"' in benchmark
