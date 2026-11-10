"""Static contract of the leave-out residual standard-error surface.

Stata is not run here. These checks bind the pieces that must agree across
the Rust ABI, the C header, the Stata helpers, the package manifest and the
help file, so that a change to one is noticed in the others.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKAGE = ROOT / "fevc"


def read(relative):
    return (ROOT / relative).read_text(encoding="utf-8")


def test_variance_source_code_and_capability_bit_agree_across_the_boundary():
    rust = read("rust/crates/vckss-plugin/src/ffi_engine.rs")
    header = read("rust/stata_backend/include/vckss_rust.h")
    shim = read("rust/stata_backend/cshim/stata_entry.c")
    assert "pub const VCKSS_COMPONENT_VARIANCE_LEAVEOUT_RESIDUAL: u32 = 3;" in rust
    assert "pub const VCKSS_CORE_RESIDUAL_SE_V1_READY: u64 = 1 << 18;" in rust
    assert "#define VCKSS_COMPONENT_VARIANCE_LEAVEOUT_RESIDUAL 3u" in header
    assert "#define VCKSS_CORE_RESIDUAL_SE_V1_READY (UINT64_C(1) << 18)" in header
    assert 'strcmp(text, "leaveout_residual") == 0' in shim
    assert "__vckss_rust_comp_residual_api" in shim
    assert "comp_residual_api" in read("fevc/fevc_rust.ado")


def test_registered_variance_source_enumeration_is_unchanged():
    # Registered campaign sources match this enumeration exhaustively, so the
    # new method is a flag on the prepared attachment, not a fourth variant.
    core = read("rust/crates/vckss-core/src/component_inference.rs")
    body = core[core.index("pub enum ComponentVarianceSource") :]
    body = body[: body.index("}")]
    assert re.findall(r"^\s*(\w+) = \d+,", body, flags=re.M) == [
        "Oracle",
        "StructuredCommon",
        "StructuredLeverage",
    ]
    assert "pub leave_out_residual: bool," in core


def test_probe_count_convention_is_four_solves_per_draw():
    core = read("rust/crates/vckss-core/src/component_inference.rs")
    assert "RESIDUAL_NOISE_SOLVES_PER_DRAW: u32 = 1 + PRIMITIVE_TARGETS as u32" in core
    route = read("fevc/fevc__component_model_route.ado")
    assert "local inferencesimulations = 4*`inferencedraws'" in route
    assert "local inferencegramprobes = 0" in route
    fetch = read("fevc/fevc__rust_component_fetch.ado")
    assert "`simulations'/4" in fetch
    assert "`receipt'[1,22]==`simulations'+3" in fetch
    wrapper = read("fevc/fevc_rust.ado")
    assert "mod(`probes',4) != 0" in wrapper


def test_helpers_map_the_model_to_source_three_and_fit_three():
    attach = read("fevc/fevc__rust_component_attach.ado")
    fetch = read("fevc/fevc__rust_component_fetch.ado")
    for text in (attach, fetch):
        assert 'cond("`model\'"=="leaveout_residual",3,2)' in text
    assert "local expectedfit = cond(`modelcode'==3,3,2)" in fetch
    post = read("fevc/fevc__rust_component_post.ado")
    assert "fevc__residual_se_post" in post
    batch = read("fevc/fevc__rust_comp_batch_receipt.ado")
    assert "`batch'[1,7]==(`gram_probes'>0)" in batch


def test_point_estimation_remains_the_default_and_auto_never_reroutes():
    route = read("fevc/fevc__component_model_route.ado")
    assert "if !`inference_supplied' local inference none" in route
    assert 'inlist("`inference\'", "none", "auto", "highrank", "q1")' in route
    fallback = route[route.index("if `se_auto' {\n            // Point estimates only") :]
    fallback = fallback[: fallback.index("else {")]
    assert "local inference none" in fallback
    # The automatic mode may only narrow to point estimates; it must not set
    # a backend, engine, algorithm or preconditioner.
    for option in ("backend", "engine", "algorithm", "preconditioner", "nuisance"):
        assert not re.search(rf"c_local {option}\b", route)
    command = read("fevc/fevc.ado")
    assert "INFERENCEDraws(integer 8)" in command
    assert "global VCKSS_SE_AUTO `se_auto'" in command


def test_corrected_centering_is_admitted_only_for_this_model():
    centering = read("fevc/fevc__centering.ado")
    assert 'lower(strtrim("`inferencemodel\'"))!="leaveout_residual"' in centering
    plugin = read("rust/crates/vckss-plugin/src/ffi_engine.rs")
    assert "fn centering_bound_component(" in plugin


def test_display_and_returns_state_the_assumption():
    post = read("fevc/fevc__residual_se_post.ado")
    assert "not robust to arbitrary heteroskedasticity" in post
    assert 'ereturn local inference_variance_fit "leaveout_residual"' in post
    assert "ereturn scalar inference_mean_omitted = 0" in post
    display = read("fevc/fevc__display.ado")
    assert "program define _fevc_display_residual_se" in display
    assert "Not robust to arbitrary heteroskedasticity." in display
    assert "normal confidence intervals can then under-cover" in display


def test_mata_runtime_is_separate_and_lazily_loaded():
    mata = read("fevc/fevc_residual_se.mata")
    assert "mata set matastrict on" in mata
    assert 'return("vckss-residual-se-api1")' in mata
    assert "void vckss_residual_se__stata(" in mata
    command = read("fevc/fevc.ado")
    branch = command[
        command.index('if "`inferencemodel\'"=="leaveout_residual" & "`inference\'"!="none" {') :
    ]
    branch = branch[: branch.index("else capture noisily mata: vckss_inference__stata(")]
    assert "findfile fevc_residual_se.mata" in branch
    assert "vckss_residual_se__stata(" in branch
    # The exact structured runtime is not edited for this model.
    exact = read("fevc/fevc_inference.mata")
    assert "vckss_rse__" not in exact and '"leaveout_residual"' not in exact


def test_package_ships_the_new_files_and_help_documents_the_method():
    manifest = read("fevc/fevc.pkg").splitlines()
    assert "f fevc_residual_se.mata" in manifest
    assert "f fevc__residual_se_post.ado" in manifest
    help_text = read("fevc/fevc.sthlp")
    assert "{marker se}" in help_text
    assert "{cmd:inference(none|auto|highrank|q1)}" in help_text
    assert "{cmd:inferencedraws(}{it:#}{cmd:)}" in help_text
    assert "heteroskedasticity.}" in help_text
    assert "Point estimation remains the default." in help_text
    assert (PACKAGE / "docs/standard_errors/standard_errors.tex").is_file()
