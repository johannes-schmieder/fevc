"""Active guidance must not contradict the qualified explicit match boundary."""
import re
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
# Minimal expectations from the qualified source; no Git history is needed.
INVENTORY = {
    "source_commit": "e9573ffe6346620318596c74461921b74cd5c231",
    "help_sha256": "c6c1474567f3b5c4f36c68a98edcf52116c8384c9c60e1bfb3bf2e0a9d3807c6",
    "manifest_sha256": "a972cd8a5b08e311946acb406b8beb5f51e7a463ce39ec904263dc76a638c48f",
    "examples": [
        "exact_controls",
        "jla_controls",
        "weights_targets",
        "component_inference",
        "projection_inference"
    ],
    "files": [
        "LICENSE",
        "THIRD_PARTY_NOTICES.txt",
        "fevc.ado",
        "fevc.mata",
        "fevc_numerical.mata",
        "fevc_inference.mata",
        "fevc_graph.mata",
        "fevc_cmg.mata",
        "fevc_solver.mata",
        "fevc_rng.mata",
        "fevc_scale.mata",
        "fevc_resource.mata",
        "fevc_scale_engine.mata",
        "fevc_scale_runtime.mata",
        "fevc__display.ado",
        "fevc__lifecycle.ado",
        "fevc_estat.ado",
        "fevc_run.ado",
        "fevc_rust.ado",
        "fevc__rust_plugin_call.ado",
        "fevc__rust_solve_v4.ado",
        "fevc__rust_solve_v5.ado",
        "fevc__rust_plan_receipt.ado",
        "fevc__rust_reconcile_comp_v7.ado",
        "fevc__rust_reconcile_exact_v7.ado",
        "fevc__rust_post_comp_v7.ado",
        "fevc__rust_post_exact_v7.ado",
        "fevc__rust_capture_stayers.ado",
        "fevc__rust_post_stayer_hybrid.ado",
        "fevc__rust_macos.ado",
        "fevc__rust_windows.ado",
        "fevc__rust_linux.ado",
        "fevc__rust_public_call.ado",
        "fevc__component_model_route.ado",
        "fevc__exact_inference_model_post.ado",
        "fevc__rust_component_attach.ado",
        "fevc__rust_component_fetch.ado",
        "fevc__rust_component_post.ado",
        "fevc__failure_guidance.ado",
        "fevc.sthlp"
    ]
}


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
    current = (ROOT / "fevc/fevc.sthlp").read_text()
    pattern = r"\{\* example_start - ([^}]+)\}\{\.\.\.\}(.*?)\{\* example_end\}\{\.\.\.\}"
    current_examples = dict(re.findall(pattern, current, re.S))
    assert len(current_examples) == 5
    assert set(INVENTORY["examples"]) == current_examples.keys()
    # Numerical fixture and truth checks run in Stata after moving generation
    # into the installed helper; example text is no longer byte-identical.
    for name, number in (("weights_targets", 3), ("component_inference", 5)):
        assert f"simulate_data(ex{number})" in current_examples[name]


def test_catalog_preserves_match_install_inventory():
    manifest = (ROOT / "fevc/fevc.pkg").read_text()
    additive_helpers = {
        "f fevc__numerical.ado",
        "f fevc__rust_numerical.ado",
        "f fevc__simulate_data.ado",
        "f fevc__progress.ado",
        "f fevc__memory_options.ado",
        "f fevc__native_threads.ado",
        "f fevc_timer.mata",
        "f fevc__timer.ado",
        "f fevc__control_failure_post.ado",
        "f fevc__observation_population.ado",
        "f fevc__hybrid_sample.ado",
        "f fevc__stayer_population_post.ado",
        "f fevc__rust_cmg_model.ado",
        "f fevc__rust_comp_batch_receipt.ado",
        "f fevc__rust_core_ready.ado",
    }
    assert additive_helpers <= set(manifest.splitlines())
    # Compare the historical inventory under the current helper filename convention.
    previous = ["f " + name for name in INVENTORY["files"]]
    assert previous == [
        x for x in manifest.splitlines() if x.startswith("f ") and x not in additive_helpers
    ]
    assert "fixed-offset match" in manifest
    assert "fixed-offset match" in (ROOT / "fevc/stata.toc").read_text()
