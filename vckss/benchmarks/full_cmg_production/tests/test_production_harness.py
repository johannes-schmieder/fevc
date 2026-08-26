from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
HARNESS = ROOT / "vckss" / "benchmarks" / "full_cmg_production"


def test_stata_driver_requires_the_production_identity_and_state_gates() -> None:
    source = (HARNESS / "stata_run.do").read_text(encoding="utf-8")
    for token in (
        'e(cmg_backend)',
        '"CMG_FULL_V2"',
        'e(full_cmg_receipt)',
        'e(cmg_source_commit)',
        'data_restored',
        'rng_restored',
        'sort_rng_restored',
        'VCKSS_FULL_CMG_PRODUCTION_STATA_PASS',
    ):
        assert token in source
    assert "VCKSS_PRIVATE_CMG" not in source


def test_runner_is_source_bound_and_position_balanced() -> None:
    source = (HARNESS / "run_local.py").read_text(encoding="utf-8")
    for token in (
        '"release: 1.85.1"',
        'CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"',
        'PRIVATE_WINNER_SECONDS = 81.145',
        'WARM_REPETITIONS = 5',
        '"faster_than_matlab_gate"',
        '"within_five_percent_private_winner_gate"',
        '"two_x_matlab_objective"',
    ):
        assert token in source
    assert source.count('(\"vckss\", \"matlab\")') == 3
    assert source.count('(\"matlab\", \"vckss\")') == 3
