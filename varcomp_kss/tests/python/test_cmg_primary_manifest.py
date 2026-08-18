import csv
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BUILDER = ROOT / "varcomp_kss/benchmarks/scc/build_cmg_primary_manifest.py"
SUBMITTER = ROOT / "varcomp_kss/benchmarks/scc/submit_cmg_primary_matrix.sh"
COMMIT = "1" * 40
BUNDLE = "2" * 64


def test_primary_manifest_is_complete_matched_grid(tmp_path: Path) -> None:
    output = tmp_path / "tasks.tsv"
    completed = subprocess.run(
        [
            sys.executable,
            str(BUILDER),
            "--output",
            str(output),
            "--source-commit",
            COMMIT,
            "--bundle",
            BUNDLE,
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    assert "rows=120" in completed.stdout
    with output.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    assert len(rows) == 120
    assert len({row["experiment_id"] for row in rows}) == 120
    assert {int(row["cells_per_worker"]) for row in rows} == set(range(2, 8))
    assert {int(row["rows_per_cell"]) for row in rows} == {1, 8}
    assert {int(row["firms"]) for row in rows} == {16, 32, 64, 256, 1024}
    assert {int(row["probes"]) for row in rows} == {20}
    assert {int(row["stata_processors"]) for row in rows} == {4}
    assert {int(row["stata_slots"]) for row in rows} == {14}
    assert {int(row["matlab_mem_per_core_gib"]) for row in rows} == {32}
    counts = {}
    for row in rows:
        firms = int(row["firms"])
        counts[firms] = counts.get(firms, 0) + 1
        assert int(row["workers"]) == 40 * firms
        assert row["source_commit"] == COMMIT
        assert row["bundle_sha256"] == BUNDLE
    assert counts == {1024: 36, 256: 36, 64: 24, 32: 12, 16: 12}


def test_primary_submitter_has_valid_shell_syntax() -> None:
    subprocess.run(["bash", "-n", str(SUBMITTER)], check=True)
