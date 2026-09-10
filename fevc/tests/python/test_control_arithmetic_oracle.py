"""The shared Rust/Mata endpoints must come from an independent exact oracle."""
import importlib.util
from pathlib import Path


def test_control_crossproduct_oracles_are_exact_binary_inputs():
    path = Path(__file__).resolve().parents[1] / 'oracles/control_crossproducts.py'
    spec = importlib.util.spec_from_file_location('control_oracle', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.FIXTURE.read_text() == module.fixture_text()
    assert module.SCORE_FIXTURE.read_text() == module.score_fixture_text()
