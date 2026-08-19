from __future__ import annotations

import csv
import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "benchmarks/prep_bnd1"
COMMON = HARNESS / "common.py"


def module():
    spec = importlib.util.spec_from_file_location("prep_bnd1_common", COMMON)
    assert spec is not None and spec.loader is not None
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


def profile_rows(common, role: str, retained_calls: int, retained_rows: int):
    commit = "a" * 40 if role == "baseline" else "b" * 40
    rows = []
    for metric in common.PREP_RHS_METRICS:
        rows.append(
            {
                "source_label": role,
                "source_commit": commit,
                "run": "1",
                "matrix_name": "prep_profile",
                "schema": common.PREP_RHS_SCHEMA,
                "metric": metric,
                "value": "1",
            }
        )
    for metric in common.PREP_BND_METRICS:
        rows.append(
            {
                "source_label": role,
                "source_commit": commit,
                "run": "1",
                "matrix_name": "prep_boundary_profile",
                "schema": common.PREP_BND_SCHEMA,
                "metric": metric,
                "value": "1",
            }
        )
    counts = {metric: 1 for metric in common.PREP_BND_COUNT_METRICS}
    counts.update(
        {
            "retained_id_group_calls": retained_calls,
            "semantic_group_calls": 1 if role == "baseline" else 0,
            "stata_sort_calls": 2 if role == "baseline" else 1,
            "retained_map_columns": 2,
            "retained_map_rows": retained_rows,
            "compression_import_rows": retained_rows,
        }
    )
    for metric, value in counts.items():
        rows.append(
            {
                "source_label": role,
                "source_commit": commit,
                "run": "1",
                "matrix_name": "prep_boundary_counts",
                "schema": common.PREP_BND_COUNTS_SCHEMA,
                "metric": metric,
                "value": str(value),
            }
        )
    return rows


def test_causal_transition_is_fail_closed() -> None:
    common = module()
    baseline = profile_rows(common, "baseline", 2, 27)
    candidate = profile_rows(common, "candidate", 0, 27)
    result = common.validate_causal_transition(baseline, candidate, [{"n_retained": "27"}])
    assert result[0]["baseline_retained_id_group_calls"] == 2
    assert result[0]["candidate_retained_id_group_calls"] == 0
    assert result[0]["baseline_semantic_group_calls"] == 1
    assert result[0]["candidate_semantic_group_calls"] == 0
    assert result[0]["baseline_stata_sort_calls"] == 2
    assert result[0]["candidate_stata_sort_calls"] == 1
    changed = profile_rows(common, "candidate", 0, 27)
    next(row for row in changed if row["metric"] == "initial_id_group_calls")["value"] = "2"
    with pytest.raises(ValueError, match="causal count changed"):
        common.validate_causal_transition(baseline, changed, [{"n_retained": "27"}])
    changed = profile_rows(common, "candidate", 0, 27)
    next(row for row in changed if row["metric"] == "stata_sort_calls")["value"] = "2"
    with pytest.raises(ValueError, match="exposures are inconsistent"):
        common.validate_causal_transition(baseline, changed, [{"n_retained": "27"}])


def test_profile_reader_requires_registered_metrics(tmp_path: Path) -> None:
    common = module()
    path = tmp_path / "profiles.csv"
    rows = profile_rows(common, "candidate", 0, 27)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=rows[0])
        writer.writeheader()
        writer.writerows(rows)
    loaded = common.read_profiles(path, "candidate", "b" * 40, 1)
    assert len(loaded) == len(rows)
    rows.pop()
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=rows[0])
        writer.writeheader()
        writer.writerows(rows)
    with pytest.raises(ValueError, match="count metrics differ"):
        common.read_profiles(path, "candidate", "b" * 40, 1)


def test_qacct_requires_scheduler_success_and_slots(tmp_path: Path) -> None:
    common = module()
    path = tmp_path / "123.txt"
    path.write_text(
        "jobnumber 123\nfailed 0\nexit_status 0\nhostname scc-x1\nqname batch\n"
        "slots 4\nru_wallclock 10\ncpu 8.5\nmaxvmem 1.2G\n",
        encoding="utf-8",
    )
    assert common.parse_qacct(path, "123", 4)["hostname"] == "scc-x1"
    path.write_text(path.read_text().replace("exit_status 0", "exit_status 1"))
    with pytest.raises(ValueError, match="nonzero exit"):
        common.parse_qacct(path, "123", 4)


def test_wrappers_follow_scc_resource_and_source_policy() -> None:
    scale = (HARNESS / "run_pair.sge").read_text(encoding="utf-8")
    cz18 = (HARNESS / "run_cz18_pair.sge").read_text(encoding="utf-8")
    assert "#$ -P welfgr" in scale and "#$ -pe omp 4" in scale
    assert "#$ -P welfgr" in cz18 and "#$ -pe omp 14" in cz18
    assert "set processors 4" in (HARNESS / "local_driver.do").read_text(encoding="utf-8")
    assert "PREP_BND_BASELINE" in scale and "PREP_BND_CANDIDATE" in scale
    assert "72179fb" not in scale and "a83f902" not in scale
    assert "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575" in cz18
    assert 'KSS_PREP_BND1_SCC_PASS ${role} ${commit}' in scale
    assert 'KSS_PREP_BND1_CZ18_PASS ${role} ${commit}' in cz18
    assert '${case_name//-/ }' not in scale


def test_local_runner_is_four_process_archive_isolated() -> None:
    source = (HARNESS / "run_local.py").read_text(encoding="utf-8")
    assert "git\", \"archive" in source
    assert "TemporaryDirectory" in source
    assert 'ORDERS = ("ab", "ba")' in source
    assert '"fresh_stata_processes": 4' in source
    assert "benchmark tool worktree must be clean" in source


def test_driver_captures_command_timer_before_r_class_calls() -> None:
    source = (HARNESS / "local_driver.do").read_text(encoding="utf-8")
    capture = source.index("local command_seconds = r(t80)")
    later_r_class = source.index("quietly count if `in_sample'")
    assert capture < later_r_class
    assert "`command_seconds',e(sample_selection_seconds)" in source


def test_offline_analyzer_uses_unwrapped_application_prefix() -> None:
    source = (HARNESS / "analyze_scc.py").read_text(encoding="utf-8")
    assert 'marker = f"{MARKER_SCALE} {role} {commit}"' in source
    assert 'marker = f"{MARKER_CZ18} {role} {commit}"' in source


def test_scientific_contract_allows_only_registered_roundoff() -> None:
    common = module()
    baseline = {field: "same" for field in common.EXACT_FIELDS}
    baseline.update({field: "1" for field in common.SCIENTIFIC_FIELDS})
    candidate = dict(baseline)
    candidate["r12"] = "1.0000000000001"
    common.compare_scientific_contract([baseline], [candidate], "roundoff")
    candidate["r12"] = "1.00000001"
    with pytest.raises(ValueError, match="numerical mismatch r12"):
        common.compare_scientific_contract([baseline], [candidate], "regression")
