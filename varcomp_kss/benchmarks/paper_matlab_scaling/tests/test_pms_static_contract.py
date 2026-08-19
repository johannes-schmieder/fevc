from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HARNESS = ROOT / "varcomp_kss" / "benchmarks" / "paper_matlab_scaling"


def test_pair_is_same_host_scalar_and_resource_bound() -> None:
    source = (HARNESS / "run_pair.sge").read_text(encoding="utf-8")
    for token in (
        "#$ -pe omp 4",
        "#$ -l h_rt=08:00:00",
        "#$ -l mem_per_core=14G",
        "stata-mp -q do",
        "matlab -batch",
        "qsub",
    ):
        if token == "qsub":
            assert token not in source
        else:
            assert token in source
    assert "-t " not in source


def test_both_estimators_use_match_deletion_and_p200() -> None:
    stata = (HARNESS / "stata_run.do").read_text(encoding="utf-8")
    matlab = (HARNESS / "paper_matlab_scaling_run.m").read_text(encoding="utf-8")
    assert "deletion(match)" in stata
    assert "probes(`probes')" in stata
    assert "leave_out_level = 'matches'" in matlab
    assert "probes==200" in matlab
    assert "size(unique([worker firm],'rows'),1)==expected_rows" in matlab
    assert "numel(unique([worker firm],'rows'))" not in matlab
    assert "'schema','kss_matlab_scale_process_identity_v1'" in matlab


def test_source_bundle_contains_entire_production_harness() -> None:
    allowlist_path = ROOT / "varcomp_kss" / "benchmarks" / \
        "prep_bnd1_matlab" / "bundle_allowlist.txt"
    entries = allowlist_path.read_text(encoding="utf-8").splitlines()
    assert entries == sorted(entries)
    expected = {
        f"varcomp_kss/benchmarks/paper_matlab_scaling/{path.name}"
        for path in HARNESS.iterdir()
        if path.is_file()
    }
    assert expected <= set(entries)
