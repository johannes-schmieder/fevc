from __future__ import annotations

import re
from pathlib import Path


PATH = Path("varcomp_kss/varcomp_kss.ado")
SOURCE = PATH.read_text(encoding="utf-8")

LEGACY_START = "program define _vckss_rust_generic, eclass sortpreserve\n"
PLANNED_START = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
IMPL_START = "program define _vckss_impl, eclass sortpreserve\n"


def require_count(source: str, needle: str, count: int, label: str) -> None:
    observed = source.count(needle)
    if observed != count:
        raise RuntimeError(f"{label}: expected {count} match(es), found {observed}")


def replace_once(source: str, old: str, new: str, label: str) -> str:
    require_count(source, old, 1, label)
    print(f"replaced {label}")
    return source.replace(old, new, 1)


def sub_once(source: str, pattern: str, replacement: str, label: str) -> str:
    updated, count = re.subn(pattern, replacement, source, count=1, flags=re.S)
    if count != 1:
        raise RuntimeError(f"{label}: expected one regex match, found {count}")
    print(f"replaced {label}")
    return updated


require_count(SOURCE, LEGACY_START, 1, "legacy generic program")
require_count(SOURCE, IMPL_START, 1, "implementation program")
require_count(SOURCE, PLANNED_START, 0, "planned generic program absence")

before, remainder = SOURCE.split(LEGACY_START, 1)
legacy_tail, implementation = remainder.split(IMPL_START, 1)
legacy = LEGACY_START + legacy_tail
planned = PLANNED_START + legacy_tail

# The legacy V2 generic route is copied, never modified.
legacy_fingerprint = legacy

planned = replace_once(
    planned,
    """        targetweightsupplied frequencyused cmdline
""",
    """        targetweightsupplied frequencyused cmdline                  ///
        preconditionerrequested batchrequested wallsecondssupplied       ///
        wallseconds
""",
    "planned generic argument surface",
)

planned = replace_once(
    planned,
    """    else if `deletionidsupplied' {
        local deletion_source matchid
        local deletion_source_code = 2
    }

    capture quietly varcomp_kss_rust clear
""",
    """    else if `deletionidsupplied' {
        local deletion_source matchid
        local deletion_source_code = 2
    }
    local preconditioner_requested = lower(strtrim("`preconditionerrequested'"))
    local batch_request = lower(strtrim("`batchrequested'"))
    local phase_batch_mode = cond("`batch_request'"=="auto","auto","explicit")
    local phase_batch_code = cond("`phase_batch_mode'"=="auto",0,1)
    local solve_batch = cond("`phase_batch_mode'"=="auto",0,`batch')
    local fallback_allowed = cond("`preconditioner_requested'"=="auto",1,0)
    local route_expected_code = cond("`preconditioner_requested'"=="auto",0, ///
        cond("`preconditioner_requested'"=="cmg",3,2))
    local wallseconds_supplied_code = real("`wallsecondssupplied'")
    local wallseconds_value = cond(`wallseconds_supplied_code',real("`wallseconds'"),0)
    if !inlist("`preconditioner_requested'","auto","diagonal","cmg") | ///
        !inlist("`phase_batch_mode'","auto","explicit") |            ///
        !inlist(`wallseconds_supplied_code',0,1) |                   ///
        (`wallseconds_supplied_code' &                               ///
            (missing(`wallseconds_value') | `wallseconds_value'<=0)) | ///
        (!`wallseconds_supplied_code' & `wallseconds_value'!=0) {
        quietly _vckss_post_failure "INVALID_TUNING"                ///
            "The planned Rust route received an invalid route, batch, or wall tuple."
        exit 198
    }

    capture quietly varcomp_kss_rust clear
""",
    "planned generic request locals",
)

planned = sub_once(
    planned,
    r"""    capture quietly _vckss_rust_public_call requestcapability,.*?
    tempvar rust_keep
""",
    """    capture quietly _vckss_rust_public_call requestcapability,       ///
        algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
        route(`preconditioner_requested') rngcontract(counter_v1)    ///
        controls(`control_count') frequencyused(`frequency_code')    ///
        engine(generic) batchmode(`phase_batch_mode')                ///
        leveragebatchmode(`phase_batch_mode')                        ///
        targetbatchmode(`phase_batch_mode') stayers(movers)          ///
        targetweightmode(`target_mode') deletionsource(`deletion_source') ///
        probeordersupplied(0) wallsecondssupplied(`wallseconds_supplied_code') ///
        fallback(`fallback_allowed') wallseconds(`wallseconds_value') ///
        physicallimit(`physicallimit')
    if _rc {
        capture quietly varcomp_kss_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "planned generic-JLA route could not obtain a V3 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNAVAILABLE"      ///
            "The planned generic-JLA capability query was unavailable; no native preparation was attempted."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    foreach name in struct_size abi_version request_schema supported ///
        reason_code profile_code algorithm_code deletion_mode_code   ///
        nuisance_mode_code solver_route_code rng_contract_code       ///
        controls_count frequency_use_code engine_code batch_mode_code ///
        stayers_mode_code target_weight_mode_code deletion_source_code ///
        probeorder_supplied wallseconds_supplied physical_limit      ///
        request_signature_hi request_signature_lo                    ///
        leverage_batch_mode_code target_batch_mode_code              ///
        automatic_fallback_allowed wallseconds                       ///
        algorithm_resolution_deferred engine_resolution_deferred     ///
        route_resolution_deferred leverage_batch_deferred            ///
        target_batch_resolution_deferred wall_advisory_only {
        local cap_`name' = r(`name')
    }
    local cap_reason_name `"`r(reason)'"'
    local cap_profile_name `"`r(profile)'"'
    local capability_ok = 1
    foreach name in struct_size abi_version request_schema supported ///
        reason_code profile_code algorithm_code deletion_mode_code   ///
        nuisance_mode_code solver_route_code rng_contract_code       ///
        controls_count frequency_use_code engine_code batch_mode_code ///
        stayers_mode_code target_weight_mode_code deletion_source_code ///
        probeorder_supplied wallseconds_supplied physical_limit      ///
        request_signature_hi request_signature_lo                    ///
        leverage_batch_mode_code target_batch_mode_code              ///
        automatic_fallback_allowed algorithm_resolution_deferred     ///
        engine_resolution_deferred route_resolution_deferred         ///
        leverage_batch_deferred target_batch_resolution_deferred     ///
        wall_advisory_only {
        if missing(`cap_`name'') | `cap_`name'' < 0 |               ///
            `cap_`name'' != floor(`cap_`name'') local capability_ok = 0
    }
    if missing(`cap_wallseconds') | `cap_wallseconds'<0 local capability_ok = 0
    if `capability_ok' {
        local capability_ok =                                      ///
            `cap_struct_size'==160 & `cap_abi_version'==1 &        ///
            `cap_request_schema'==3 & `cap_supported'==1 &         ///
            `cap_reason_code'==0 & `cap_profile_code'==4 &         ///
            `cap_algorithm_code'==2 &                              ///
            `cap_deletion_mode_code'==`deletion_code' &            ///
            `cap_nuisance_mode_code'==`nuisance_code' &            ///
            `cap_solver_route_code'==`route_expected_code' &       ///
            `cap_rng_contract_code'==1 &                           ///
            `cap_controls_count'==`control_count' &                ///
            `cap_frequency_use_code'==`frequency_code' &           ///
            `cap_engine_code'==2 &                                 ///
            `cap_batch_mode_code'==`phase_batch_code' &            ///
            `cap_stayers_mode_code'==1 &                           ///
            `cap_target_weight_mode_code'==`target_code' &         ///
            `cap_deletion_source_code'==`deletion_source_code' &   ///
            `cap_probeorder_supplied'==0 &                         ///
            `cap_wallseconds_supplied'==`wallseconds_supplied_code' & ///
            `cap_physical_limit'==`physicallimit' &                ///
            `cap_leverage_batch_mode_code'==`phase_batch_code' &   ///
            `cap_target_batch_mode_code'==`phase_batch_code' &     ///
            `cap_automatic_fallback_allowed'==`fallback_allowed' & ///
            `cap_wallseconds'==`wallseconds_value' &               ///
            `cap_algorithm_resolution_deferred'==0 &               ///
            `cap_engine_resolution_deferred'==0 &                  ///
            `cap_route_resolution_deferred'==                      ///
                ("`preconditioner_requested'"=="auto") &           ///
            `cap_leverage_batch_deferred'==                        ///
                ("`phase_batch_mode'"=="auto") &                   ///
            `cap_target_batch_resolution_deferred'==               ///
                ("`phase_batch_mode'"=="auto") &                   ///
            `cap_wall_advisory_only'==1 &                          ///
            `cap_request_signature_hi'<=4294967295 &               ///
            `cap_request_signature_lo'<=4294967295
    }
    if !`capability_ok' {
        capture quietly varcomp_kss_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "planned generic-JLA route rejected an inconsistent V3 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNQUALIFIED"      ///
            "The V3 request-capability receipt did not reconcile with the materialized planned generic-JLA tuple."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    tempvar rust_keep
""",
    "planned capability lifecycle",
)

planned = sub_once(
    planned,
    r"""    capture noisily _vckss_rust_public_call solve `handle',.*?
    capture noisily _vckss_rust_public_call result `handle'
""",
    """    capture noisily _vckss_rust_public_call solve `handle',         ///
        seed(`seed') probes(`probes') leveragebatch(`solve_batch')   ///
        targetbatch(`solve_batch') route(`preconditioner_requested') ///
        tolerance(`tolerance') maxiter(`maxiter') algorithm(jla)    ///
        deletion(`deletionmode') nuisance(`nuisance')               ///
        exactlimit(`exactlimit') blocksizelimit(`blocksizelimit')   ///
        ranktolerance(`ranktol') blocktolerance(`blocktol')          ///
        engine(generic) batchmode(`phase_batch_mode')                ///
        leveragebatchmode(`phase_batch_mode')                        ///
        targetbatchmode(`phase_batch_mode') stayers(movers)          ///
        targetweightmode(`target_mode') deletionsource(`deletion_source') ///
        probeordersupplied(0) wallsecondssupplied(`wallseconds_supplied_code') ///
        physicallimit(`physicallimit') capabilityschema(3)          ///
        capabilityprofile(4) frequencyused(`frequency_code')        ///
        signaturehi(`cap_request_signature_hi')                     ///
        signaturelo(`cap_request_signature_lo')                     ///
        fallback(`fallback_allowed') wallseconds(`wallseconds_value')
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(solve)
        exit _rc
    }

    capture noisily _vckss_rust_public_call result `handle'
""",
    "planned solve V4 dispatch",
)

planned = replace_once(
    planned,
    """        request_signature_hi:r_signature_hi request_signature_lo:r_signature_lo {
""",
    """        request_signature_hi:r_signature_hi request_signature_lo:r_signature_lo ///
        leverage_batch_mode_code:r_lev_batch_mode                    ///
        target_batch_mode_code:r_tgt_batch_mode plan_schema:r_plan_schema ///
        plan_route_schema:r_plan_route_schema plan_route_req:r_plan_route_req ///
        plan_route_sel:r_plan_route_sel plan_route_fallback:r_plan_route_fallback ///
        plan_route_error:r_plan_route_error wall_requested:r_wall_requested_value ///
        wall_forecast:r_wall_forecast_value wall_advisory:r_wall_advisory_value ///
        wall_margin:r_wall_margin_value mem_command:r_plan_mem_command {
""",
    "planned V7 result capture",
)

planned = replace_once(
    planned,
    """            if `rhs_native'[`row',4]!=2 | `rhs_native'[`row',5]<0 | ///
""",
    """            if `rhs_native'[`row',4]!=`r_sel_route' |             ///
                `rhs_native'[`row',5]<0 |                              ///
""",
    "planned RHS route reconciliation",
)

planned = replace_once(
    planned,
    """        local batch_start = cond(`probe'<0,1,floor(`probe'/`batch')*`batch'+1)
""",
    """        local active_batch = cond(`phase'==1,`r_lev_batch',`r_tgt_batch')
        local batch_start = cond(`probe'<0,1,                       ///
            floor(`probe'/`active_batch')*`active_batch'+1)
""",
    "planned public batch starts",
)

planned = replace_once(
    planned,
    """        r_wallseconds r_frequency r_physical_limit r_signature_hi r_signature_lo
""",
    """        r_wallseconds r_frequency r_physical_limit r_signature_hi r_signature_lo ///
        r_lev_batch_mode r_tgt_batch_mode r_plan_schema             ///
        r_plan_route_schema r_plan_route_req r_plan_route_sel       ///
        r_plan_route_fallback r_plan_route_error                    ///
        r_wall_requested_value r_wall_forecast_value                ///
        r_wall_advisory_value r_wall_margin_value r_plan_mem_command
""",
    "planned receipt completeness list",
)

planned = replace_once(
    planned,
    """    foreach value of local receipt_numbers {
        if missing(``value'') local results_ok = 0
    }
    if `results_ok' {
""",
    """    foreach value of local receipt_numbers {
        if missing(``value'') local results_ok = 0
    }
    local route_result_ok =                                      ///
        `r_req_route'==`route_expected_code' &                   ///
        inlist(`r_sel_route',2,3) &                              ///
        (`route_expected_code'==0 | `r_sel_route'==`route_expected_code') & ///
        `r_full_route'==`r_sel_route' &                          ///
        inlist(`r_fallback',0,1) &                               ///
        (`fallback_allowed' | `r_fallback'==0) &                 ///
        (`r_fallback' | `r_fallback_err'==0) &                   ///
        `r_plan_route_req'==`r_req_route' &                      ///
        `r_plan_route_sel'==`r_sel_route' &                      ///
        `r_plan_route_fallback'==`r_fallback' &                  ///
        `r_plan_route_error'==`r_fallback_err'
    local batch_result_ok =                                      ///
        `r_lev_batch'>=1 & `r_lev_batch'<=`probes' &             ///
        `r_tgt_batch'>=1 & `r_tgt_batch'<=`probes' &             ///
        (`phase_batch_code'==0 |                                 ///
            (`r_lev_batch'==`batch' & `r_tgt_batch'==`batch')) & ///
        `r_lev_batch_mode'==`phase_batch_code' &                 ///
        `r_tgt_batch_mode'==`phase_batch_code'
    local capability_result_ok =                                 ///
        `r_cap_schema'==3 & `r_cap_profile'==4 &                 ///
        `r_batch_mode'==`phase_batch_code' &                     ///
        `r_stayers_mode'==1 & `r_target_mode'==`target_code' &   ///
        `r_deletion_source'==`deletion_source_code' &            ///
        `r_probeorder'==0 &                                      ///
        `r_wallseconds'==`wallseconds_supplied_code' &           ///
        `r_frequency'==`frequency_code' &                        ///
        `r_physical_limit'==`physicallimit' &                    ///
        `r_plan_schema'==1 & `r_plan_route_schema'==2 &          ///
        `r_wall_requested_value'==`wallseconds_value'
    local memory_result_ok =                                     ///
        `r_result_bytes'==`r_generic_result' &                   ///
        `r_result_bytes'>=`r_rhs_v2_copy' &                      ///
        `r_solver_setup'==max(`r_canon_peak',`r_fit_peak',`r_geometry_peak') & ///
        `r_generic_peak'==max(`r_canon_peak',`r_fit_peak',       ///
            `r_geometry_peak',`r_generic_lev_peak',              ///
            `r_generic_tgt_peak',`r_maker_peak',`r_generic_result') & ///
        `r_solve_peak'==`r_generic_peak' &                       ///
        `r_solve_peak'==`r_plan_mem_command' &                   ///
        `r_command_peak'==max(`r_prep_peak',`r_solve_peak') &    ///
        `r_command_peak'<=`r_mem_limit'
    if `results_ok' {
""",
    "planned dynamic result predicates",
)

planned = replace_once(
    planned,
    """            `r_req_route'==2 & `r_sel_route'==2 &                  ///
            `r_fallback'==0 & `r_fallback_err'==0 &                ///
""",
    """            `route_result_ok' &                                   ///
""",
    "planned route predicate",
)

planned = replace_once(
    planned,
    """            `r_lev_batch'==`batch' & `r_tgt_batch'==`batch' &      ///
""",
    """            `batch_result_ok' &                                   ///
""",
    "planned batch predicate",
)

planned = replace_once(
    planned,
    """            `r_full_tol'==`expected_full_tol' & `r_full_route'==2 & ///
""",
    """            `r_full_tol'==`expected_full_tol' &                    ///
""",
    "planned full-route predicate",
)

planned = replace_once(
    planned,
    """            `r_cap_schema'==2 & `r_cap_profile'==3 &               ///
            `r_batch_mode'==1 & `r_stayers_mode'==1 &              ///
            `r_target_mode'==`target_code' &                       ///
            `r_deletion_source'==`deletion_source_code' &          ///
            `r_probeorder'==0 & `r_wallseconds'==0 &               ///
            `r_frequency'==`frequency_code' &                      ///
            `r_physical_limit'==`physicallimit' &                  ///
""",
    """            `capability_result_ok' &                               ///
""",
    "planned capability predicate",
)

planned = sub_once(
    planned,
    r"""            `r_result_bytes'==`r_generic_result' &.*?
            `r_command_peak'<=`r_mem_limit' &                      ///
""",
    """            `memory_result_ok' &                                 ///
""",
    "planned memory predicate",
)

planned = replace_once(
    planned,
    """    ereturn scalar batch = `batch'
""",
    """    ereturn scalar batch = max(`r_lev_batch',`r_tgt_batch')
""",
    "planned selected batch",
)

planned = replace_once(
    planned,
    """    ereturn scalar rust_batch_mode_code = `r_batch_mode'
""",
    """    ereturn scalar rust_batch_mode_code = `r_batch_mode'
    ereturn scalar rust_leverage_batch_mode_code = `r_lev_batch_mode'
    ereturn scalar rust_target_batch_mode_code = `r_tgt_batch_mode'
    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested_value'
    ereturn scalar rust_wallseconds_forecast = `r_wall_forecast_value'
    ereturn scalar rust_wallseconds_advisory = `r_wall_advisory_value'
    ereturn scalar rust_wallseconds_margin = `r_wall_margin_value'
    ereturn scalar rust_plan_solve_peak_forecast_bytes = `r_plan_mem_command'
""",
    "planned public summary receipts",
)

planned = replace_once(
    planned,
    """    ereturn local backend_routing_reason "fully explicit public generic-JLA route"
""",
    """    ereturn local backend_routing_reason "explicit planned public generic-JLA route"
""",
    "planned backend routing reason",
)

planned = replace_once(
    planned,
    """    ereturn local preconditioner_requested "diagonal"
    ereturn local preconditioner_selected "diagonal"
    ereturn local routing_reason "explicit generic diagonal PCG route"
    ereturn local fallback_status "NOT_PERMITTED"
    ereturn local fallback_message "generic-JLA completed on the requested route without fallback"
    ereturn local batch_requested "`batch'"
    ereturn local batch_routing_reason "caller supplied the required explicit batch width"
""",
    """    ereturn local preconditioner_requested "`preconditioner_requested'"
    ereturn local preconditioner_selected = cond(`r_sel_route'==3,"cmg","diagonal")
    ereturn local routing_reason "native planned generic-JLA route"
    ereturn local fallback_status = cond(`r_fallback',"CMG_TO_DIAGONAL", ///
        cond(`fallback_allowed',"ELIGIBLE_NOT_USED","NOT_ELIGIBLE"))
    ereturn local fallback_message = cond(`r_fallback',              ///
        "CMG setup failed before RNG and the permitted diagonal fallback completed", ///
        "generic-JLA completed on the selected native route")
    ereturn local batch_requested "`batch_request'"
    ereturn local batch_routing_reason = cond("`phase_batch_mode'"=="auto", ///
        "native planner selected independent phase widths",          ///
        "caller supplied the shared explicit phase width")
""",
    "planned public routing metadata",
)

planned = replace_once(
    planned,
    """    ereturn local route_api "VCKSS-NATIVE-GENERIC-V3-V6"
    ereturn local rust_capability_profile "JLA_GENERIC_COUNTER_V1"
""",
    """    ereturn local route_api "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
    ereturn local rust_capability_profile "PLANNED_V1"
    ereturn local execution_plan_schema "VCKSS-EXECUTION-PLAN-V1"
""",
    "planned route API metadata",
)

# Reassemble with the untouched legacy program followed by the planned clone.
if legacy != legacy_fingerprint:
    raise RuntimeError("legacy generic program changed before reassembly")
updated = before + legacy + planned + IMPL_START + implementation

# Add the narrow public support predicate without changing permanent Mata defaults
# or the proven legacy generic/diagonal route.
updated = replace_once(
    updated,
    """        local rust_exact_supported =                           ///
""",
    """        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' & "`engine_requested'" == "generic" & ///
            `preconditioner_supplied' & "`preconditioner'" == "auto" & ///
            `batch_supplied' &                                     ///
            `rng_supplied' & "`rng_requested'" == "counter_v1" & ///
            inlist("`deletion'","match","observation") &          ///
            inlist("`nuisance'","joint","fixedoffset") &          ///
            "`stayers'" == "movers" & "`probeorder'" == ""
        local rust_exact_supported =                           ///
""",
    "planned public support predicate",
)

updated = replace_once(
    updated,
    """        local rust_options_supported =                         ///
            `rust_legacy_jla_supported' | `rust_generic_supported' | ///
            `rust_exact_supported'
""",
    """        local rust_options_supported =                         ///
            `rust_legacy_jla_supported' | `rust_generic_supported' | ///
            `rust_planned_generic_supported' | `rust_exact_supported'
""",
    "planned public options support",
)

updated = replace_once(
    updated,
    """        else if `rust_generic_requested' {
            capture noisily _vckss_rust_generic `depvar'          ///
""",
    """        else if `rust_planned_generic_supported' {
            local rust_planned_wallseconds = cond(`wallseconds_supplied', ///
                `wallseconds',0)
            capture noisily _vckss_rust_generic_planned `depvar'  ///
                `initial_worker' `initial_firm' `rust_deletion'   ///
                `frequency' `target' `touse' `N_scope' `N_complete' ///
                `N_stayers' `N_stayer_rows' `probes' `batch' `seed' ///
                `tolerance' `maxiter' `memory_gib'                ///
                `engine_requested' `backend_supplied' `rng_supplied' ///
                `deletionid_supplied' `engine_supplied'           ///
                `algorithm_supplied' `preconditioner_supplied'     ///
                `batch_supplied' `stayers_supplied'                ///
                `rust_core_flags' `rust_support_flags' `"`nodisplay'"' ///
                `deletion' `nuisance' `exact_limit'               ///
                `rank_tolerance' `block_tolerance'                ///
                `blocksize_limit' `physical_limit' `"`controlvars'"' ///
                `targetweight_supplied' `rust_frequency_used' `"`cmdline'"' ///
                `preconditioner' `batch_requested'                 ///
                `wallseconds_supplied' `rust_planned_wallseconds'
        }
        else if `rust_generic_requested' {
            capture noisily _vckss_rust_generic `depvar'          ///
""",
    "planned public dispatch",
)

updated = replace_once(
    updated,
    """                "The Rust route supports exact estimation, the frozen compressed JLA subset, or the fully explicit generic-JLA tuple."
""",
    """                "The Rust route supports exact estimation, the frozen compressed JLA subset, the explicit generic-diagonal tuple, or the planned generic-auto tuple."
""",
    "planned unsupported-route message",
)

# Fail closed if production structure moved unexpectedly or legacy behavior was altered.
require_count(updated, LEGACY_START, 1, "final legacy generic program")
require_count(updated, PLANNED_START, 1, "final planned generic program")
require_count(updated, IMPL_START, 1, "final implementation program")
final_before, final_remainder = updated.split(LEGACY_START, 1)
final_legacy_tail, _ = final_remainder.split(PLANNED_START, 1)
if LEGACY_START + final_legacy_tail != legacy_fingerprint:
    raise RuntimeError("legacy generic program changed during planned-route transformation")
if "backend() omitted; the permanent legacy default is Mata" not in updated:
    raise RuntimeError("permanent omitted-backend Mata default was not preserved")
if "backend(auto) permanently selected the Mata route" not in updated:
    raise RuntimeError("permanent backend(auto) Mata default was not preserved")

PATH.write_text(updated, encoding="utf-8")
print("planned generic Rust route transformation completed")
