"""Reject incomplete or mismatched Windows qualification evidence."""
from copy import deepcopy
import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location(
    "windows_qualification", ROOT / "rust/stata_backend/windows_qualification.py"
)
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)


def record(group="cmg_point", arm="candidate", block=0):
    identity = {
        "production_commit": ("a" if arm == "candidate" else "b") * 40,
        "harness_sha256": "c" * 64,
        "runtime_profile": "smoke" if group == "smoke" else "full",
        "qualification_group": group,
        "campaign_sha256": "d" * 64,
        "performance_arm": arm,
        "performance_block": block,
    }
    completed = list(contract.COMMON + contract.GROUPS[group])
    scope = "HOSTED_CI_PASS"
    gates = {x: "PASS" for x in completed}
    gates.update({x: scope for x in ("rust_workspace", "rust_backend")})
    rows = []
    if group == "performance":
        rows = [[case, workers, 10.0, 1., 2., .1, 3.2,
                 .01, .01, .01, .01, 1e-12, 1e-9, 200]
                for case in range(6) for workers in (1, 2)]
    value = {**identity, "schema": "FEVC-WINDOWS-CHECKS-V4", "gates": gates,
             "stata_version": "19", "stata_flavor": "MP",
             "binary_sha256": ("e" if arm == "candidate" else "f") * 64,
             "status": "PASS", "completed_tests": completed, "measurements": rows}
    return identity, value


def validate(value, identity=None):
    if identity is None:
        identity = value
    return contract.validate_checks(value, identity, value["binary_sha256"], "HOSTED_CI_PASS")


@pytest.mark.parametrize("group", list(contract.GROUPS))
def test_valid_group_is_not_global_qualification(group):
    identity, value = record(group)
    assert validate(value, identity) == value
    with pytest.raises(ValueError, match="incomplete"):
        contract.aggregate([value])


@pytest.mark.parametrize("mutation", [
    lambda x: x["gates"].pop("registry_idle"),
    lambda x: x["gates"].update(cmg_boundary="SKIP"),
    lambda x: x["completed_tests"].pop(),
    lambda x: x["completed_tests"].append(x["completed_tests"][0]),
    lambda x: x.update(status="FAIL"),
    lambda x: x.update(schema="FEVC-WINDOWS-CHECKS-V3"),
    lambda x: x.update(binary_sha256="unknown"),
    lambda x: x.update(stata_version="18"),
    lambda x: x.update(log="unbounded diagnostic"),
])
def test_missing_skipped_duplicate_or_unbounded_checks_fail(mutation):
    identity, value = record()
    mutation(value)
    with pytest.raises(ValueError):
        validate(value, identity)


def test_source_campaign_and_binary_must_match():
    identity, value = record()
    for field in ("production_commit", "harness_sha256", "campaign_sha256"):
        bad = deepcopy(value)
        bad[field] = "0" * len(bad[field])
        with pytest.raises(ValueError):
            validate(bad, identity)
    with pytest.raises(ValueError):
        contract.validate_checks(value, identity, "0" * 64, "HOSTED_CI_PASS")


@pytest.mark.parametrize("mutation", [
    lambda rows: rows.pop(),
    lambda rows: rows.append(rows[0]),
    lambda rows: rows.__setitem__(1, rows[0]),
    lambda rows: rows[0].__setitem__(2, float("nan")),
    lambda rows: rows[0].__setitem__(2, -1),
    lambda rows: rows[0].__setitem__(7, -1),
    lambda rows: rows[0].__setitem__(11, 1e-3),
    lambda rows: rows[0].__setitem__(1, 4),
    lambda rows: rows[0].append(99),
])
def test_partial_duplicate_or_scientifically_failing_measurements_fail(mutation):
    identity, value = record("performance")
    mutation(value["measurements"])
    with pytest.raises(ValueError):
        validate(value, identity)


def campaign():
    return [record(group)[1] for group in contract.REQUIRED] + [
        record("performance", arm, block)[1]
        for arm in ("candidate", "incumbent") for block in range(6)
    ]


def test_aggregation_requires_every_group_and_fixed_candidate():
    rows = campaign()
    for value in rows:
        validate(value)
    result = contract.aggregate(rows)
    assert result["status"] == "PASS"
    assert len(result["cells"]) == 12
    for broken in (rows[:-1], rows + [rows[0]]):
        with pytest.raises(ValueError):
            contract.aggregate(broken)
    for field, value in (("binary_sha256", "0"*64),
                         ("production_commit", "0"*40),
                         ("campaign_sha256", "0"*64)):
        bad = deepcopy(rows)
        bad[0][field] = value
        with pytest.raises(ValueError):
            contract.aggregate(bad)


def test_strict_json_rejects_duplicates_and_nonfinite_values():
    for value in ('{"a":1,"a":2}', '{"a":NaN}', '{"a":Infinity}'):
        with pytest.raises(ValueError):
            contract.strict_json(value)


def test_comparison_preserves_adverse_scientific_and_timing_cells():
    rows = campaign()
    for value in rows:
        if value["qualification_group"] == "performance" and value["performance_arm"] == "candidate":
            value["measurements"][0][2] = 12.0
            value["measurements"][1][3] += 1.0
    result = contract.aggregate(rows)
    assert result["status"] == "FAIL"
    assert result["cells"][0]["performance"] == "FAIL"
    assert result["cells"][1]["science"] == "FAIL"
    assert len(result["cells"][0]["candidate_ms"]) == 6
    assert result["retries_or_exclusions"] == 0


def test_one_millisecond_allowance_is_a_complete_command_limit():
    rows = campaign()
    for value in rows:
        if value["qualification_group"] == "performance" and value["performance_arm"] == "candidate":
            for row in value["measurements"]:
                row[2] = 11.0
    assert contract.aggregate(rows)["status"] == "PASS"
