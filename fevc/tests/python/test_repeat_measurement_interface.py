# SPDX-License-Identifier: MIT
"""Static interface checks for repeat-measurement standard errors.

The method is a variant of the leave-out residual pass: the deletion units of
one worker-firm cell are repeat measurements and receive their sample variance
(an eligible stayer's observations; under observation deletion also a mover's
match of two or more observations), the remaining units a two-step fitted
model, and the covariance is the high-rank form with a closed-form pair term
of the sets. These checks pin the identifiers that the Rust core, the plugin
boundary, the C shim and the Stata front end must agree on. They do not run
Stata.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def test_capability_bit_and_source_code_agree_across_layers():
    plugin = read("rust/crates/vckss-plugin/src/ffi_engine.rs")
    header = read("rust/stata_backend/include/vckss_rust.h")
    shim = read("rust/stata_backend/cshim/stata_entry.c")
    assert "pub const VCKSS_CORE_REPEAT_SE_V1_READY: u64 = 1 << 19;" in plugin
    assert "| VCKSS_CORE_REPEAT_SE_V1_READY;" in plugin
    assert "#define VCKSS_CORE_REPEAT_SE_V1_READY (UINT64_C(1) << 19)" in header
    assert "pub const VCKSS_COMPONENT_VARIANCE_REPEAT_MEASUREMENT: u32 = 4;" in plugin
    assert "#define VCKSS_COMPONENT_VARIANCE_REPEAT_MEASUREMENT 4u" in header
    assert 'strcmp(text, "repeat_measurement") == 0' in shim
    assert "__vckss_rust_comp_repeat_api" in shim
    assert "comp_repeat_api" in read("fevc/fevc_rust.ado")
    abi = read("rust/stata_backend/tests/abi_header_compat_test.c")
    assert "VCKSS_CORE_REPEAT_SE_V1_READY == UINT64_C(524288)" in abi
    assert "sizeof(VckssRepeatMeasurementDiagnosticV1) == 120" in abi


def test_the_variant_is_a_flag_on_the_residual_pass():
    core = read("rust/crates/vckss-core/src/component_inference.rs")
    assert "pub repeat_measurement: bool," in core
    assert "pub fn prepare_repeat_measurement_with_interrupt(" in core
    # It shares routing, admission and solve accounting with the residual pass.
    prepare = core[core.index("pub fn prepare_repeat_measurement_with_interrupt(") :]
    prepare = prepare[: prepare.index("\n}\n")]
    assert "prepare_residual_noise_with_interrupt(" in prepare
    # Both deletion units are admitted; observation units need unit frequencies,
    # which the shared preparation enforces.
    assert "ComponentInferenceUnit::Match" not in prepare
    # The registered variance-source enumeration stays as it is.
    body = core[core.index("pub enum ComponentVarianceSource") :]
    body = body[: body.index("}")]
    assert re.findall(r"^\s*(\w+) = \d+,", body, flags=re.M) == [
        "Oracle",
        "StructuredCommon",
        "StructuredLeverage",
    ]
    attachment = read("rust/crates/vckss-core/src/generic_jla/residual_noise_attachment.rs")
    assert "if prepared.repeat_measurement {" in attachment
    assert "finish_repeat_covariance(" in attachment
    engine = read("rust/crates/vckss-core/src/generic_jla.rs")
    assert "&& !prepared.repeat_measurement" in engine
    # Observation units get the fixed-offset pass under joint nuisance, and a
    # stayer's observations are rows like any other.
    offset = engine[engine.index("fn offset_pass_requested(") :]
    offset = offset[: offset.index("\n}\n")]
    assert "|| prepared.repeat_measurement)" in offset
    repeat = read("rust/crates/vckss-core/src/residual_noise/repeat.rs")
    assert "pub set: &'a [u64]," in repeat
    assert "pub mover: &'a [bool]," in repeat


def test_variance_rule_constants():
    repeat = read("rust/crates/vckss-core/src/residual_noise/repeat.rs")
    assert "pub const PROFILE_TERMS: usize = 6;" in repeat
    assert "pub const PROFILE_CAP: f64 = 3.0;" in repeat
    assert "pub const SCALE_FLOOR_SHARE: f64 = 0.05;" in repeat
    # The covariance is reported as computed.
    assert "signal_truncation: 0.0," in repeat
    mata = read("fevc/fevc_residual_se.mata")
    assert "real colvector vckss_rse__repeat(" in mata
    assert "scale_floor = 0.05*mean(product[fit])" in mata
    assert "void vckss_rse__sets(" in mata
    assert "fit = units >= 100 ? selectindex(single) : selectindex(unit_mover)" in mata
    assert "(count-2)*(count-1)/(count+1)" in mata
    assert "(count - 2.0) * (count - 1.0)" in repeat and "/ (count + 1.0)" in repeat


def test_diagnostic_export_and_shim_reconciliation():
    export = read("rust/crates/vckss-plugin/src/ffi_component_v5.rs")
    assert "pub struct VckssRepeatMeasurementDiagnosticV1 {" in export
    assert "const _: [(); 120] = [(); size_of::<VckssRepeatMeasurementDiagnosticV1>()];" in export
    assert "pub extern \"C\" fn vckss_rust_engine_repeat_measurement_diagnostic_v1(" in export
    assert "receipt.variance_fit = if noise.repeat.is_some() { 4 } else { 3 };" in export
    header = read("rust/stata_backend/include/vckss_rust.h")
    assert "} VckssRepeatMeasurementDiagnosticV1;" in header
    assert "int32_t vckss_rust_engine_repeat_measurement_diagnostic_v1(" in header
    shim = read("rust/stata_backend/cshim/stata_entry.c")
    assert "if (receipt5.variance_fit == 4u) {" in shim
    assert "repeat.stayer_units != units.stayer_observations" in shim
    assert "__vckss_rm_pair_term" in shim


def test_stata_front_end_admits_the_model_explicitly_for_both_deletion_modes():
    route = read("fevc/fevc__component_model_route.ado")
    assert '"repeat_measurement") {' in route
    assert "REPEAT_MEASUREMENT_Q1_UNSUPPORTED" not in route
    assert "REPEAT_MEASUREMENT_TUPLE_REQUIRED" in route
    assert "!`repeat' &" not in route
    assert """("`residual_deletion'"=="observation" &                       ///""" in route
    assert "(`repeat' & r(component_repeat_api)!=1)" in route
    assert 'global VCKSS_SE_MODEL = cond(`residual\',"`inferencemodel\'","")' in route
    # The default is unchanged: inference(auto) without a typed model selects
    # leaveout_residual; with inferencemodel(repeat_measurement) it keeps it.
    assert (
        """if `se_auto' & !inlist("`residual_model'","","leaveout_residual","repeat_measurement") {"""
        in route
    )
    assert '"repeat_measurement","leaveout_residual")' in route
    post = read("fevc/fevc__residual_se_post.ado")
    assert "program define _fevc_repeat_se_post, eclass" in post
    assert 'ereturn local inference_variance_fit "repeat_measurement"' in post
    assert 'ereturn local status "FEVC_REPEAT_MEASUREMENT_SE"' in post
    assert "estimated control coefficients are treated as known" in post
    display = read("fevc/fevc__display.ado")
    assert "repeat-measurement" in display
    assert "The standard error is as good as that" in display


def test_leading_directions_travel_through_their_own_export_and_one_stata_helper():
    core = read("rust/crates/vckss-core/src/generic_jla/leading_direction.rs")
    assert "pub(super) fn leading_direction(" in core
    assert "const BASIS: usize = 6;" in core and "const KEPT: usize = 3;" in core
    attachment = read("rust/crates/vckss-core/src/generic_jla/residual_noise_attachment.rs")
    # Requested by the q1 reference on the repeat-measurement source only.
    assert "prepared.options.reference_distribution == ComponentReferenceDistribution::Q1" in attachment
    assert "finish_repeat_q1_target(" in attachment
    repeat = read("rust/crates/vckss-core/src/residual_noise/repeat.rs")
    assert "pub fn remainder_pair_term(" in repeat
    assert "let leading_variance = fitted.finish().max(leading_variance_leave_out);" in repeat
    export = read("rust/crates/vckss-plugin/src/ffi_component_v5.rs")
    assert "pub extern \"C\" fn vckss_rust_engine_repeat_measurement_leading_v1(" in export
    assert "const _: [(); 872] = [(); size_of::<VckssRepeatMeasurementLeadingV1>()];" in export
    header = read("rust/stata_backend/include/vckss_rust.h")
    assert "} VckssRepeatMeasurementLeadingV1;" in header
    assert "int32_t vckss_rust_engine_repeat_measurement_leading_v1(" in header
    abi = read("rust/stata_backend/tests/abi_header_compat_test.c")
    assert "sizeof(VckssRepeatMeasurementLeadingV1) == 872" in abi
    shim = read("rust/stata_backend/cshim/stata_entry.c")
    assert "__vckss_rl_present" in shim
    assert "(uint64_t)base->probes + 3u + noise.offset_pass_solves + leading_solves" in shim
    route = read("fevc/fevc__component_model_route.ado")
    assert "global VCKSS_SE_LEADING = cond(" in route
    assert "global VCKSS_SE_LEADING_CAP 40" in route
    post = read("fevc/fevc__residual_se_post.ado")
    # The decision and the posting live in the existing helper, so that the
    # package file list does not change.
    assert "program define _fevc_se_regime_decide" in post
    assert "program define _fevc_se_regime_post, eclass" in post
    assert "local share_threshold .05" in post
    assert 'if "`1\'"=="regime_decide" {' in post
    fetch = read("fevc/fevc__rust_component_fetch.ado")
    assert "fevc__residual_se_post regime_decide `computed' `posted' `level'" in fetch
    assert "matrix `receipt'[1,24] = 3" in fetch
    command = read("fevc/fevc.ado")
    assert "fevc__residual_se_post regime_decide `inference_V' `corrected' `level'" in command
    mata = read("fevc/fevc_residual_se.mata")
    assert "real colvector vckss_rse__lead_solve(" in mata
    assert "real matrix vckss_rse__lead_finish(" in mata
    assert 'st_matrix("__fevc_se_leading",lead_table)' in mata
    display = read("fevc/fevc__display.ado")
    assert "program define _fevc_display_intervals" in display
    assert "e(V) is not posted: it is posted only when every component is normal." in display
