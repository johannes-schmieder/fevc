"""Golden deleted-regression values are stable with doubled Decimal precision."""
import importlib.util
from pathlib import Path


def test_control_anchor_oracle_is_reproducible():
    path=Path(__file__).resolve().parents[1]/"oracles/control_anchor.py"
    spec=importlib.util.spec_from_file_location("anchor_oracle",path)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.FIXTURE.read_text()==module.fixture_text(120)==module.fixture_text(240)
