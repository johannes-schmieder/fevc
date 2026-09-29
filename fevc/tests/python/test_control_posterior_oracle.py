"""Ensure shared certificate fixtures retain their independent Decimal source."""
import importlib.util
from pathlib import Path

def test_posterior_fixture_is_reproducible():
    path=Path(__file__).resolve().parents[1]/'oracles/control_posterior.py'
    spec=importlib.util.spec_from_file_location('posterior_oracle',path)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.FIXTURE.read_text()==module.fixture_text()

    assert (module.FIXTURE.parent/'control_span_residual.txt').read_text()==module.span_fixture_text()
