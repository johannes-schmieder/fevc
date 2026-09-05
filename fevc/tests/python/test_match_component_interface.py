"""Keep the explicit match boundary, qualification inventory and docs aligned."""
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]


def test_scope_does_not_waive_failed_observation_confirmation():
    scope = json.loads((ROOT / "fevc/docs/fixed_offset_match_interface_v1.json").read_text())
    result = json.loads((ROOT / "fevc/docs/rc_observation_inference_v1_result.json").read_text())
    assert result["status"] == "FAIL"
    assert scope["scientific_evidence"]["observation_status"] == "FAIL_UNCHANGED"
    assert scope["tuple"]["nuisance"] == "explicit fixedoffset"
    assert scope["tuple"]["population"] == "explicit movers"
    assert scope["tuple"]["engine"] == "explicit generic"


def test_native_and_clean_install_profiles_include_public_match_gate():
    test = "test_rust_match_component_inference.do"
    for path in ("fevc/tests/stata/run_all.do", "fevc/tests/stata/test_rust_public_install.do",
                 "rust/stata_backend/qualify_macos.sh"):
        assert test in (ROOT / path).read_text()
    qualifier = (ROOT / "rust/stata_backend/qualify_macos.sh").read_text()
    assert "run_stata_case arm64 public-match-component-inference" in qualifier
    assert "run_stata_case x86_64 public-match-component-inference" in qualifier


def test_match_unit_metadata_is_additive_to_statistical_v4():
    header = (ROOT / "rust/stata_backend/include/vckss_rust.h").read_text()
    assert "sizeof(VckssComponentInferenceResultReceiptV4) == 208" in header
    assert "sizeof(VckssComponentInferenceUnitReceiptV1) == 64" in header
    for name in ("vckss_rust_engine_augment_component_inference_interrupt_v1",
                 "vckss_rust_engine_augment_match_component_inference_interrupt_v1",
                 "vckss_rust_engine_component_inference_unit_receipt_v1"):
        assert name in header
    fetch = (ROOT / "fevc/_fevc_rust_component_fetch.ado").read_text()
    assert "`receipt'[1,1]==4" in fetch
    assert "`units'[1,3]==`expectedunits'" in fetch
    post = (ROOT / "fevc/_fevc_rust_component_post.ado").read_text()
    assert post.index("ereturn matrix component_unit_receipt") > post.index("ereturn scalar inference_smallest_maker")
    for field in ("inference_independent_units", "inference_nuisance_omitted",
                  "inference_effective_matches", "inference_largest_mass_share",
                  "inference_largest_leverage", "inference_smallest_maker"):
        assert field in post
        assert field in (ROOT / "fevc/docs/FAILURES_AND_RETURNS.md").read_text()
