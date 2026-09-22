"""Active guidance must not contradict the qualified explicit match boundary."""
import re
import subprocess
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
QUALIFIED_SOURCE = "e9573ffe6346620318596c74461921b74cd5c231"  # Review-purged equivalent of 53f22a109effee87467b4ef0602b21d0b8ec1ca9


@pytest.mark.parametrize("path,obsolete", [
    ("fevc/fevc.sthlp", "Match inference remains internal"),
    ("fevc/fevc.sthlp", "Fresh confirmation is required"),
    ("fevc/docs/DECISIONS.md", "The next internal match-deletion"),
    ("fevc/docs/DECISIONS.md", "The grouped route remains internal"),
    ("rust/README.md", "automatic routing, and public invocation"),
    ("rust/TEST_PLAN.md", "Point estimates only are implemented"),
    ("fevc/docs/MATRIX_FREE_COMPONENT_INFERENCE.md", "without exposing a public route"),
    ("fevc/PLAN.md", "Keep grouped match inference internal"),
])
def test_active_guidance_has_no_superseded_match_restriction(path, obsolete):
    text = " ".join((ROOT / path).read_text().split())
    assert obsolete not in text


def test_help_retains_scope_warning_and_links_to_detailed_inference():
    text = " ".join((ROOT / "fevc/fevc.sthlp").read_text().split())
    reference = " ".join((ROOT / "fevc/docs/INFERENCE.md").read_text().split())
    assert "ignoring nuisance-control estimation uncertainty" in text
    assert "Observation-q1 calibration retains a documented limitation" in text
    assert "docs/INFERENCE.md" in text
    # Confirmation history belongs in the detailed reference, not the applied help.
    assert "one failed SE-ratio gate" in reference
    assert "Independent q0 and repaired eligible q1 confirmations pass" in reference


def test_help_preserves_all_runnable_example_names():
    old = subprocess.check_output(
        ["git", "show", f"{QUALIFIED_SOURCE}:fevc/fevc.sthlp"], cwd=ROOT, text=True
    )
    current = (ROOT / "fevc/fevc.sthlp").read_text()
    pattern = r"\{\* example_start - ([^}]+)\}\{\.\.\.\}(.*?)\{\* example_end\}\{\.\.\.\}"
    old_examples = dict(re.findall(pattern, old, re.S))
    current_examples = dict(re.findall(pattern, current, re.S))
    assert len(old_examples) == len(current_examples) == 5
    assert old_examples.keys() == current_examples.keys()
    # Numerical fixture and truth checks run in Stata after moving generation
    # into the installed helper; example text is no longer byte-identical.
    for name, number in (("weights_targets", 3), ("component_inference", 5)):
        assert f"simulate_data(ex{number})" in current_examples[name]


def test_catalog_preserves_match_install_inventory():
    manifest = (ROOT / "fevc/fevc.pkg").read_text()
    old = subprocess.check_output(
        ["git", "show", f"{QUALIFIED_SOURCE}:fevc/fevc.pkg"], cwd=ROOT, text=True
    )
    additive_helpers = {
        "f fevc__simulate_data.ado",
        "f fevc__progress.ado",
        "f fevc__memory_options.ado",
        "f fevc__observation_population.ado",
        "f fevc__stayer_population_post.ado",
        "f fevc__rust_cmg_model.ado",
        "f fevc__rust_comp_batch_receipt.ado",
        "f fevc__rust_core_ready.ado",
    }
    assert additive_helpers <= set(manifest.splitlines())
    # Compare the historical inventory under the current helper filename convention.
    previous = [x.replace("f _fevc", "f fevc_", 1)
                for x in old.splitlines() if x.startswith("f ")]
    assert previous == [
        x for x in manifest.splitlines() if x.startswith("f ") and x not in additive_helpers
    ]
    assert "fixed-offset match" in manifest
    assert "fixed-offset match" in (ROOT / "fevc/stata.toc").read_text()
