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
    # Three influence solves, four per draw, and the offset pass when the
    # point estimate keeps controls in its operator.
    assert "`receipt'[1,22]>=`simulations'+3" in fetch
    assert "mod(`receipt'[1,22]-`simulations'-3,2)==0" in fetch
    assert "return scalar plan_rhs = cond(`variance_fit'==3,`receipt'[1,22]," in fetch
    command = read("fevc/fevc.ado")
    assert "local component_plan_rhs = r(plan_rhs)" in command
    assert "`r_plan_rhs'==`expected_rhs_rows'+`component_plan_rhs'" in command
    assert "mod(`probes',4) != 0" in read("fevc/fevc_rust.ado")


def test_helpers_map_the_model_to_source_three_and_fit_three():
    attach = read("fevc/fevc__rust_component_attach.ado")
    fetch = read("fevc/fevc__rust_component_fetch.ado")
    mapping = """cond("`model'"=="leaveout_residual",3,2)"""
    for text in (attach, fetch):
        assert mapping in text
    assert "local expectedfit = cond(`modelcode'==3,3,2)" in fetch
    assert "fevc__residual_se_post" in read("fevc/fevc__rust_component_post.ado")
    batch = read("fevc/fevc__rust_comp_batch_receipt.ado")
    assert "`batch'[1,7]==(`gram_probes'>0)" in batch


def test_standard_errors_are_the_default_and_never_reroute_or_cost_the_estimate():
    route = read("fevc/fevc__component_model_route.ado")
    assert "local inference auto" in route
    assert """inlist("`inference'", "none", "auto", "highrank", "q1")""" in route
    # Suites written for the point-only default keep it without editing.
    assert "FEVC_DEFAULT_INFERENCE" in route
    assert 'environment.setdefault("FEVC_DEFAULT_INFERENCE", "none")' in read(
        "fevc/tools/run_checks.py"
    )
    fallback = route[route.index("if `se_auto' {\n            // Point estimates only") :]
    fallback = fallback[: fallback.index("else {")]
    assert "local inference none" in fallback
    # The default may only narrow to point estimates; it must not set a
    # backend, engine, algorithm, preconditioner or nuisance mode.
    for option in ("backend", "engine", "algorithm", "preconditioner", "nuisance"):
        assert not re.search(rf"c_local {option}\b", route)
    command = read("fevc/fevc.ado")
    assert "INFERENCEDraws(integer 8)" in command
    assert "if `inference_requested' & !`residual_se_requested' local algorithm exact" in command
    # A failed attempt with default standard errors reruns without them.
    entry = command[command.index("program define fevc, eclass") :]
    entry = entry[: entry.index("\nend\n")]
    assert entry.count("capture noisily _fevc_command `0'") == 2
    assert '"$VCKSS_SE_ATTACHED"=="1"' in entry and "`command_rc'!=1" in entry
    assert "global VCKSS_SE_SUPPRESS 1" in entry
    assert "global VCKSS_SE_ATTACHED = (`se_auto' & `residual')" in route
    assert '"$VCKSS_SE_SUPPRESS"=="1"' in route


def test_native_engine_selection_and_offset_pass_are_bound_to_the_flag():
    assert "compressed_can_carry_request" in read("rust/crates/vckss-plugin/src/ffi_engine.rs")
    engine = read("rust/crates/vckss-core/src/generic_jla.rs")
    assert "fn offset_pass_requested(" in engine
    assert "&& options.nuisance == NuisanceMode::Joint" in engine
    assert "diagnostic.offset_pass_solves = offset_receipts.len();" in engine


def test_corrected_centering_is_admitted_only_for_this_model():
    centering = read("fevc/fevc__centering.ado")
    assert """lower(strtrim("`inferencemodel'"))!="leaveout_residual\"""" in centering
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
    assert 'return("vckss-residual-se-api2")' in mata
    assert "void vckss_residual_se__stata(" in mata
    assert "void vckss_residual_se_jla__stata(" in mata
    assert "real colvector vckss_rse__finite_projection(" in mata
    command = read("fevc/fevc.ado")
    start = """if "`inferencemodel'"=="leaveout_residual" & "`inference'"!="none" {"""
    branch = command[command.index(start) :]
    branch = branch[: branch.index("else capture noisily mata: vckss_inference__stata(")]
    assert "capture noisily _fevc_residual_se_mata" in branch
    helper = command[command.index("program define _fevc_residual_se_mata") :]
    helper = helper[: helper.index("\nend\n")]
    assert "findfile fevc_residual_se.mata" in helper
    assert 'vckss_rse__build_id() == "vckss-residual-se-api2"' in helper
    assert "vckss_residual_se__stata(" in helper
    assert "vckss_residual_se_jla__stata(" in helper
    # The Mata routines report through locals of the helper, which must pass
    # them back to the estimator program.
    for name in ("inference_status", "inference_joint_available", "inference_nuisance_omitted"):
        assert f"c_local {name} " in helper
    # The exact structured runtime is not edited for this model.
    exact = read("fevc/fevc_inference.mata")
    assert "vckss_rse__" not in exact and '"leaveout_residual"' not in exact
    # A numpy transliteration checks both Mata routines against the oracle.
    mirror = read("fevc/benchmarks/auto_inference_prototype/check_mata_mirror.py")
    assert "def mata_mirror(" in mirror and "def mata_jla_mirror(" in mirror


def test_package_ships_the_new_files_and_help_documents_the_method():
    manifest = read("fevc/fevc.pkg").splitlines()
    assert "f fevc_residual_se.mata" in manifest
    assert "f fevc__residual_se_post.ado" in manifest
    help_text = read("fevc/fevc.sthlp")
    assert "{marker se}" in help_text
    assert "{cmd:inference(auto|none|highrank|q1)}" in help_text
    assert "{cmd:inferencedraws(}{it:#}{cmd:)}" in help_text
    assert "heteroskedasticity.}" in help_text
    assert "{cmd:inference(none)} gives point estimates only." in help_text
    assert (PACKAGE / "docs/standard_errors/standard_errors.tex").is_file()
    assert (PACKAGE / "tests/stata/test_leaveout_residual_se.do").is_file()


def _program_code_characters(source, name):
    """Characters of code in one Stata program: comments, blank lines and
    continuation markers removed."""
    total, inside, continued, block = 0, False, False, False
    for line in source.split("\n"):
        if re.match(rf"\s*program define {name}\b", line):
            inside = True
            continue
        if not inside:
            continue
        if re.match(r"\s*end\s*$", line) and not continued:
            break
        text = line.strip()
        if block:
            block = "*/" not in text
            continue
        if text.startswith("/*") and "*/" not in text:
            block = True
            continue
        if not continued and (
            text == ""
            or text.startswith(("//", "*"))
            or (text.startswith("/*") and text.endswith("*/"))
        ):
            continue
        total += len(re.sub(r"\s*///.*$", "", line).strip()) + 1
        continued = "///" in line
    return total


def test_largest_programs_stay_within_their_earlier_size():
    # Both programs sit just under Stata's compiled-program size limit; the
    # repository has exceeded it before. These bounds are their sizes, by this
    # measure, before leave-out residual standard errors were added.
    command = read("fevc/fevc.ado")
    assert _program_code_characters(command, "_vckss_impl") <= 131_933
    assert _program_code_characters(command, "_fevc_rust_generic_planned") <= 131_552
