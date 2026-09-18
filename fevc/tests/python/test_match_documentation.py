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


def test_help_retains_scope_warning_and_both_confirmation_outcomes():
    text = " ".join((ROOT / "fevc/fevc.sthlp").read_text().split())
    for option in ("deletion(match)", "nuisance(fixedoffset)", "stayers(movers)",
                   "engine(generic)", "inference(q1)", "inference(highrank)"):
        assert option in text
    assert "ignoring nuisance-control estimation uncertainty" in text
    assert "corrected observation-q1 confirmation has one failed SE-ratio gate" in text
    assert "eligible q1 confirmations pass their registered gates" in text


def test_help_preserves_weighted_and_component_inference_examples():
    old = subprocess.check_output(
        ["git", "show", f"{QUALIFIED_SOURCE}:fevc/fevc.sthlp"], cwd=ROOT, text=True
    )
    current = (ROOT / "fevc/fevc.sthlp").read_text()
    pattern = r"\{\* example_start - ([^}]+)\}\{\.\.\.\}(.*?)\{\* example_end\}\{\.\.\.\}"
    old_examples = dict(re.findall(pattern, old, re.S))
    current_examples = dict(re.findall(pattern, current, re.S))
    assert len(old_examples) == len(current_examples) == 5
    assert old_examples.keys() == current_examples.keys()
    # Revised sorting and firm-size DGPs have numerical Stata regressions.
    for name in ("weights_targets", "component_inference"):
        assert old_examples[name] == current_examples[name]


def test_catalog_preserves_match_install_inventory():
    manifest = (ROOT / "fevc/fevc.pkg").read_text()
    old = subprocess.check_output(
        ["git", "show", f"{QUALIFIED_SOURCE}:fevc/fevc.pkg"], cwd=ROOT, text=True
    )
    additive_helpers = {
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
