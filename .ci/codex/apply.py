from __future__ import annotations

import re
from pathlib import Path


path = Path("varcomp_kss/varcomp_kss.ado")
text = path.read_text(encoding="utf-8")


def replace_once(source: str, old: str, new: str, label: str) -> str:
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    print(f"replaced {label}")
    return source.replace(old, new, 1)


def sub_once(source: str, pattern: str, replacement: str, label: str) -> str:
    updated, count = re.subn(pattern, replacement, source, count=1, flags=re.S)
    if count != 1:
        raise RuntimeError(f"{label}: expected one regex match, found {count}")
    print(f"replaced {label}")
    return updated


start_marker = "program define _vckss_rust_generic, eclass"
end_marker = "program define _vckss_rust_exact, eclass"
before, remainder = text.split(start_marker, 1)
generic_tail, after = remainder.split(end_marker, 1)
generic = start_marker + generic_tail

# Extend the existing generic lifecycle rather than duplicating its sample,
# retained-mask, result, cleanup, and posting machinery.
generic = replace_once(
    generic,
    """        rustsupportflags physicallimit targetweightsupplied
""",
    """        rustsupportflags physicallimit targetweightsupplied plannedmode ///
        preconditionerrequested batchrequested wallsecondssupplied wallseconds
""",
    "generic argument surface",
)

generic = replace_once(
    generic,
    """    local deletion_source_code = cond("`deletionmode'"=="match",1,2)
    tempvar rust_sample rust_retained
""",
    """    local deletion_source_code = cond("`deletionmode'"=="match",1,2)
    local planned_mode = real("`plannedmode'")
    if missing(`planned_mode') local planned_mode = 0
    local preconditioner_requested = lower("`preconditionerrequested'")
    if "`preconditioner_requested'" == "" local preconditioner_requested "diagonal"
    local batch_request = lower("`batchrequested'")
    if "`batch_request'" == "" local batch_request "`batch'"
    local phase_batch_mode = cond("`batch_request'"=="auto","auto","explicit")
    local phase_batch_code = cond("`phase_batch_mode'"=="auto",0,1)
    local solve_batch = cond("`phase_batch_mode'"=="auto",0,`batch')
    local fallback_allowed = cond("`preconditioner_requested'"=="auto",1,0)
    local route_expected_code = cond("`preconditioner_requested'"=="auto",0, ///
        cond("`preconditioner_requested'"=="cmg",3,2))
    local wallseconds_supplied_code = real("`wallsecondssupplied'")
    if missing(`wallseconds_supplied_code') local wallseconds_supplied_code = 0
    local wallseconds_value = cond(`wallseconds_supplied_code',real("`wallseconds'"),0)
    if missing(`wallseconds_value') local wallseconds_value = 0
    tempvar rust_sample rust_retained
""",
    "planned generic request locals",
)

generic = sub_once(
    generic,
    r"    capture noisily _vckss_rust_public_call requestcapability,.*?        physicallimit\(`physicallimit'\)\n    if _rc \{",
    """    if `planned_mode' {
        capture noisily _vckss_rust_public_call requestcapability,       ///
            algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
            route(`preconditioner_requested') rngcontract(counter_v1)    ///
            controls(`control_count') frequencyused(`frequency_code')   ///
            engine(generic) batchmode(independent)                       ///
            leveragebatchmode(`phase_batch_mode')                        ///
            targetbatchmode(`phase_batch_mode') stayers(movers)         ///
            targetweightmode(`targetweightmode')                         ///
            deletionsource(`deletion_source') fallback(`fallback_allowed') ///
            probeorder(0) wallsecondssupplied(`wallseconds_supplied_code') ///
            wallseconds(`wallseconds_value') physicallimit(`physicallimit')
    }
    else {
        capture noisily _vckss_rust_public_call requestcapability,       ///
            algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
            route(diagonal) rngcontract(counter_v1) controls(`control_count') ///
            frequencyused(`frequency_code') engine(generic)             ///
            batchmode(explicit) stayers(movers)                          ///
            targetweightmode(`targetweightmode')                         ///
            deletionsource(`deletion_source') probeorder(0) wallseconds(0) ///
            physicallimit(`physicallimit')
    }
    if _rc {""",
    "planned capability request",
)

generic = sub_once(
    generic,
    r"    local cap_reason_code = r\(reason_code\).*?    if !`capability_ok' \{",
    """    local cap_reason_code = r(reason_code)
    local cap_profile_code = r(profile_code)
    local cap_request_signature_hi = r(request_signature_hi)
    local cap_request_signature_lo = r(request_signature_lo)
    local cap_leverage_batch_mode_code = cond(`planned_mode',       ///
        r(leverage_batch_mode_code),1)
    local cap_target_batch_mode_code = cond(`planned_mode',         ///
        r(target_batch_mode_code),1)
    local cap_fallback_allowed = cond(`planned_mode',               ///
        r(automatic_fallback_allowed),0)
    local cap_wallseconds_value = cond(`planned_mode',r(wallseconds),0)
    local cap_algorithm_deferred = cond(`planned_mode',             ///
        r(algorithm_resolution_deferred),0)
    local cap_engine_deferred = cond(`planned_mode',                ///
        r(engine_resolution_deferred),0)
    local cap_route_deferred = cond(`planned_mode',                 ///
        r(route_resolution_deferred),0)
    local cap_lev_batch_deferred = cond(`planned_mode',             ///
        r(leverage_batch_resolution_deferred),0)
    local cap_tgt_batch_deferred = cond(`planned_mode',             ///
        r(target_batch_resolution_deferred),0)
    local cap_wall_advisory = cond(`planned_mode',r(wall_advisory_only),0)
    local capability_rc = 0
    capture confirm scalar r(abi_version)
    if _rc local capability_rc = 1
    capture confirm scalar r(struct_size)
    if _rc local capability_rc = 1
    capture confirm scalar r(support_flags)
    if _rc local capability_rc = 1
    capture confirm scalar r(supported)
    if _rc local capability_rc = 1
    capture confirm scalar r(request_schema)
    if _rc local capability_rc = 1
    capture confirm scalar r(profile_code)
    if _rc local capability_rc = 1
    capture confirm scalar r(request_signature_hi)
    if _rc local capability_rc = 1
    capture confirm scalar r(request_signature_lo)
    if _rc local capability_rc = 1
    local capability_ok = 0
    if !`capability_rc' {
        if `planned_mode' {
            local capability_ok =                                   ///
                `cap_abi_version'==1 & `cap_struct_size'==160 &      ///
                `cap_support_flags'==`rustsupportflags' &            ///
                `cap_supported'==1 & `cap_request_schema'==3 &      ///
                `cap_algorithm_code'==2 &                           ///
                `cap_deletion_code'==`deletion_code' &              ///
                `cap_nuisance_code'==`nuisance_code' &              ///
                `cap_solver_code'==`route_expected_code' &          ///
                `cap_rng_code'==1 &                                 ///
                `cap_controls_count'==`control_count' &              ///
                `cap_frequency_code'==`frequency_code' &             ///
                `cap_engine_code'==2 & `cap_batch_mode_code'==2 &   ///
                `cap_stayers_code'==1 &                             ///
                `cap_target_mode_code'==`target_code' &             ///
                `cap_deletion_source_code'==`deletion_source_code' & ///
                `cap_probeorder_supplied'==0 &                       ///
                `cap_wallseconds_supplied'==`wallseconds_supplied_code' & ///
                `cap_physical_limit'==`physicallimit' &              ///
                `cap_profile_code'==4 &                              ///
                `cap_leverage_batch_mode_code'==`phase_batch_code' & ///
                `cap_target_batch_mode_code'==`phase_batch_code' &  ///
                `cap_fallback_allowed'==`fallback_allowed' &        ///
                `cap_wallseconds_value'==`wallseconds_value' &      ///
                `cap_wall_advisory'==1
        }
        else {
            local capability_ok =                                   ///
                `cap_abi_version'==1 & `cap_struct_size'==104 &      ///
                `cap_support_flags'==`rustsupportflags' &            ///
                `cap_supported'==1 & `cap_request_schema'==2 &      ///
                `cap_algorithm_code'==2 &                           ///
                `cap_deletion_code'==`deletion_code' &              ///
                `cap_nuisance_code'==`nuisance_code' &              ///
                `cap_solver_code'==2 & `cap_rng_code'==1 &          ///
                `cap_controls_count'==`control_count' &              ///
                `cap_frequency_code'==`frequency_code' &             ///
                `cap_engine_code'==2 & `cap_batch_mode_code'==1 &   ///
                `cap_stayers_code'==1 &                             ///
                `cap_target_mode_code'==`target_code' &             ///
                `cap_deletion_source_code'==`deletion_source_code' & ///
                `cap_probeorder_supplied'==0 &                       ///
                `cap_wallseconds_supplied'==0 &                      ///
                `cap_physical_limit'==`physicallimit' &              ///
                `cap_reason_code'==0 & `cap_profile_code'==3
        }
    }
    if missing(`cap_request_signature_hi') |                       ///
        missing(`cap_request_signature_lo') |                      ///
        `cap_request_signature_hi'<0 | `cap_request_signature_hi'>4294967295 | ///
        `cap_request_signature_lo'<0 | `cap_request_signature_lo'>4294967295 | ///
        `cap_request_signature_hi'!=floor(`cap_request_signature_hi') | ///
        `cap_request_signature_lo'!=floor(`cap_request_signature_lo')   ///
        local capability_ok = 0
    if !`capability_ok' {""",
    "planned capability reconciliation",
)

generic = sub_once(
    generic,
    r"    capture noisily _vckss_rust_public_call solve `handle',.*?        signaturehi\(`cap_request_signature_hi'\)                       ///\n        signaturelo\(`cap_request_signature_lo'\)\n    if _rc \{",
    """    if `planned_mode' {
        capture noisily _vckss_rust_public_call solve `handle',       ///
            algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
            route(`preconditioner_requested') seed(`seed') probes(`probes') ///
            leveragebatch(`solve_batch') targetbatch(`solve_batch')  ///
            tolerance(`tolerance') maxiter(`maxiter')                 ///
            ranktol(`ranktol') blocktol(`blocktol') engine(generic)   ///
            batchmode(independent) leveragebatchmode(`phase_batch_mode') ///
            targetbatchmode(`phase_batch_mode') stayers(movers)      ///
            targetweightmode(`targetweightmode')                      ///
            deletionsource(`deletion_source')                         ///
            physicallimit(`physicallimit')                            ///
            capabilityschema(`cap_request_schema')                    ///
            capabilityprofile(`cap_profile_code')                     ///
            frequencyused(`frequency_code')                           ///
            signaturehi(`cap_request_signature_hi')                   ///
            signaturelo(`cap_request_signature_lo')                   ///
            fallback(`fallback_allowed')                              ///
            wallsecondssupplied(`wallseconds_supplied_code')          ///
            wallseconds(`wallseconds_value')
    }
    else {
        capture noisily _vckss_rust_public_call solve `handle',       ///
            algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
            route(diagonal) seed(`seed') probes(`probes')             ///
            leveragebatch(`batch') targetbatch(`batch')               ///
            tolerance(`tolerance') maxiter(`maxiter')                 ///
            ranktol(`ranktol') blocktol(`blocktol') engine(generic)   ///
            batchmode(explicit) stayers(movers)                       ///
            targetweightmode(`targetweightmode')                      ///
            deletionsource(`deletion_source')                         ///
            physicallimit(`physicallimit')                            ///
            capabilityschema(`cap_request_schema')                    ///
            capabilityprofile(`cap_profile_code')                     ///
            frequencyused(`frequency_code')                           ///
            signaturehi(`cap_request_signature_hi')                   ///
            signaturelo(`cap_request_signature_lo')
    }
    if _rc {""",
    "planned solve V4 dispatch",
)

generic = replace_once(
    generic,
    """    local r_signature_hi = r(solve_signature_hi)
    local r_signature_lo = r(solve_signature_lo)
""",
    """    local r_signature_hi = r(solve_signature_hi)
    local r_signature_lo = r(solve_signature_lo)
    local r_lev_batch_mode = cond(`planned_mode',r(batch_lev_mode),1)
    local r_tgt_batch_mode = cond(`planned_mode',r(batch_tgt_mode),1)
    local r_plan_schema = cond(`planned_mode',r(plan_schema),0)
    local r_plan_route_schema = cond(`planned_mode',r(plan_route_schema),0)
    local r_plan_route_req = cond(`planned_mode',r(plan_route_req),2)
    local r_plan_route_sel = cond(`planned_mode',r(plan_route_sel),2)
    local r_wall_requested_value = cond(`planned_mode',r(wall_requested),0)
    local r_wall_forecast_value = cond(`planned_mode',r(wall_forecast),0)
    local r_wall_advisory_value = cond(`planned_mode',r(wall_advisory),0)
    local r_wall_margin_value = cond(`planned_mode',r(wall_margin),0)
    local r_plan_mem_command = cond(`planned_mode',r(mem_command),`r_solve_peak')
""",
    "planned result receipt capture",
)

# Solver route in every RHS row must match the actual selected route.
generic = generic.replace("`rhs_native'[`row',4]!=2", "`rhs_native'[`row',4]!=`r_sel_route'")

# Use phase-specific selected widths when reconstructing public batch starts.
generic = replace_once(
    generic,
    """            local batch_start = cond(`probe'<0,1,            ///
                floor(`probe'/`batch')*`batch'+1)
""",
    """            local active_batch = cond(`phase'==1,`r_lev_batch',`r_tgt_batch')
            local batch_start = cond(`probe'<0,1,            ///
                floor(`probe'/`active_batch')*`active_batch'+1)
""",
    "phase-specific batch starts",
)

# Compute the dynamic planned-route predicates once, then retain the existing
# detailed numerical and accounting checks.
generic = replace_once(
    generic,
    """    if `results_ok' {
        local results_ok =                                         ///
""",
    """    local route_result_ok = (`r_req_route'==2 & `r_sel_route'==2 & ///
        `r_fallback'==0 & `r_fallback_err'==0 & `r_full_route'==2)
    local batch_result_ok = (`r_lev_batch'==`batch' & `r_tgt_batch'==`batch')
    local capability_result_ok = (`r_cap_schema'==2 & `r_cap_profile'==3 & ///
        `r_batch_mode'==1 & `r_wallseconds'==0)
    local memory_result_ok =                                       ///
        `r_result_bytes'==`r_generic_result' &                     ///
        `r_result_bytes'>=`r_rhs_v2_copy' &                        ///
        `r_solver_setup'==max(`r_canon_peak',`r_fit_peak',`r_geometry_peak') & ///
        `r_generic_peak'==max(`r_canon_peak',`r_fit_peak',         ///
            `r_geometry_peak',`r_generic_lev_peak',                ///
            `r_generic_tgt_peak',`r_maker_peak',`r_generic_result') & ///
        `r_solve_peak'==`r_generic_peak' &                         ///
        `r_command_peak'==max(`r_prep_peak',`r_generic_peak') &   ///
        `r_command_peak'<=`r_mem_limit'
    if `planned_mode' {
        local route_result_ok = (`r_req_route'==`route_expected_code' & ///
            inlist(`r_sel_route',2,3) &                              ///
            (`route_expected_code'==0 | `r_sel_route'==`route_expected_code') & ///
            `r_full_route'==`r_sel_route' &                          ///
            inlist(`r_fallback',0,1) &                              ///
            (`fallback_allowed' | `r_fallback'==0) &                ///
            (`r_fallback' | `r_fallback_err'==0))
        local batch_result_ok = (`r_lev_batch'>=1 & `r_lev_batch'<=`probes' & ///
            `r_tgt_batch'>=1 & `r_tgt_batch'<=`probes' &             ///
            (`phase_batch_code'==0 |                               ///
                (`r_lev_batch'==`batch' & `r_tgt_batch'==`batch')))
        local capability_result_ok = (`r_cap_schema'==3 &           ///
            `r_cap_profile'==4 & `r_batch_mode'==2 &                ///
            `r_wallseconds'==`wallseconds_supplied_code' &          ///
            `r_lev_batch_mode'==`phase_batch_code' &                ///
            `r_tgt_batch_mode'==`phase_batch_code' &                ///
            `r_plan_schema'==1 & `r_plan_route_schema'==2 &         ///
            `r_plan_route_req'==`r_req_route' &                     ///
            `r_plan_route_sel'==`r_sel_route' &                     ///
            `r_wall_requested_value'==`wallseconds_value')
        local memory_result_ok = (`r_result_bytes'>=`r_rhs_v2_copy' & ///
            `r_solve_peak'==`r_plan_mem_command' &                  ///
            `r_solve_peak'<=`r_command_peak' &                      ///
            `r_command_peak'==max(`r_prep_peak',`r_solve_peak') &  ///
            `r_command_peak'<=`r_mem_limit')
    }
    if `results_ok' {
        local results_ok =                                         ///
""",
    "planned result predicates",
)

generic = replace_once(
    generic,
    """            `r_req_route'==2 & `r_sel_route'==2 &                  ///
            `r_fallback'==0 & `r_fallback_err'==0 &                ///
""",
    """            `route_result_ok' &                                   ///
""",
    "dynamic route predicate",
)
generic = replace_once(
    generic,
    """            `r_lev_batch'==`batch' & `r_tgt_batch'==`batch' &      ///
""",
    """            `batch_result_ok' &                                   ///
""",
    "dynamic batch predicate",
)
generic = replace_once(
    generic,
    """            `r_full_tol'==`expected_full_tol' & `r_full_route'==2 & ///
""",
    """            `r_full_tol'==`expected_full_tol' &                    ///
""",
    "dynamic full-route predicate",
)
generic = replace_once(
    generic,
    """            `r_cap_schema'==2 & `r_cap_profile'==3 &               ///
            `r_batch_mode'==1 & `r_stayers_mode'==1 &              ///
""",
    """            `capability_result_ok' & `r_stayers_mode'==1 &        ///
""",
    "dynamic result capability predicate",
)
generic = replace_once(
    generic,
    """            `r_probeorder'==0 & `r_wallseconds'==0 &               ///
""",
    """            `r_probeorder'==0 &                                   ///
""",
    "dynamic wall predicate",
)
generic = sub_once(
    generic,
    r"            `r_result_bytes'==`r_generic_result' &.*?            `r_command_peak'<=`r_mem_limit' &                      ///\n",
    """            `memory_result_ok' &                                 ///
""",
    "dynamic memory predicate",
)

# Dynamic public metadata and compact V7 summary fields.
generic = replace_once(
    generic,
    """    ereturn scalar batch = `batch'
""",
    """    ereturn scalar batch = cond(`planned_mode',max(`r_lev_batch',`r_tgt_batch'),`batch')
""",
    "public selected batch",
)
generic = replace_once(
    generic,
    """    ereturn local backend_routing_reason "RUST_GENERIC_EXPLICIT"
""",
    """    ereturn local backend_routing_reason = cond(`planned_mode', ///
        "RUST_GENERIC_PLANNED","RUST_GENERIC_EXPLICIT")
""",
    "public planned routing reason",
)
generic = replace_once(
    generic,
    """    ereturn local preconditioner_requested "diagonal"
    ereturn local preconditioner_selected "diagonal"
    ereturn local routing_reason "RUST_GENERIC_DIAGONAL_EXPLICIT"
    ereturn local fallback_status "NOT_ELIGIBLE"
""",
    """    ereturn local preconditioner_requested = cond(`planned_mode', ///
        "`preconditioner_requested'","diagonal")
    ereturn local preconditioner_selected = cond(`r_sel_route'==3,"cmg","diagonal")
    ereturn local routing_reason = cond(`planned_mode',               ///
        "RUST_GENERIC_NATIVE_PLAN","RUST_GENERIC_DIAGONAL_EXPLICIT")
    ereturn local fallback_status = cond(`r_fallback',"CMG_TO_DIAGONAL", ///
        cond(`planned_mode' & `fallback_allowed',"ELIGIBLE_NOT_USED","NOT_ELIGIBLE"))
""",
    "public planned preconditioner metadata",
)
generic = replace_once(
    generic,
    """    ereturn local batch_requested "`batch'"
    ereturn local batch_rule "USER_SUPPLIED"
""",
    """    ereturn local batch_requested = cond(`planned_mode',"`batch_request'","`batch'")
    ereturn local batch_rule = cond(`planned_mode' & "`phase_batch_mode'"=="auto", ///
        "NATIVE_AUTO","USER_SUPPLIED")
""",
    "public planned batch metadata",
)
generic = replace_once(
    generic,
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
    ereturn scalar rust_planned_mode = `planned_mode'
""",
    "public planned summary receipts",
)

generic = replace_once(
    generic,
    """    ereturn local cmd "varcomp_kss"
""",
    """    ereturn local cmd "varcomp_kss"
""",
    "generic section integrity fence",
)

text = before + generic + end_marker + after

# Public option support and routing in _vckss_impl.
text = replace_once(
    text,
    """    local rust_exact_supported = `algorithm_supplied' & "`algorithm'"=="exact" & ///
""",
    """    local rust_planned_generic_supported = `algorithm_supplied' &       ///
        "`algorithm'"=="jla" & `engine_supplied' & "`engine'"=="generic" & ///
        `preconditioner_supplied' & "`preconditioner'"=="auto" &       ///
        `rng_supplied' & "`rng'"=="counter_v1" &                     ///
        inlist("`deletion'","match","observation") &                 ///
        inlist("`nuisance'","joint","fixedoffset") &                ///
        "`stayers'"=="movers" & "`probeorder'"==""
    local rust_exact_supported = `algorithm_supplied' & "`algorithm'"=="exact" & ///
""",
    "planned generic support predicate",
)
text = replace_once(
    text,
    """    local rust_options_supported = `rust_exact_supported' |           ///
        `rust_generic_supported'
""",
    """    local rust_options_supported = `rust_exact_supported' |           ///
        `rust_generic_supported' | `rust_planned_generic_supported'
""",
    "planned options support",
)
text = replace_once(
    text,
    """    local rust_generic_requested = "`backend'"=="rust" &             ///
        `rust_generic_supported'
""",
    """    local rust_generic_requested = "`backend'"=="rust" &             ///
        `rust_generic_supported'
    local rust_planned_generic_requested = "`backend'"=="rust" &     ///
        `rust_planned_generic_supported'
""",
    "planned request predicate",
)
text = replace_once(
    text,
    """    local rust_requires_counter = `rust_generic_requested'
""",
    """    local rust_requires_counter = `rust_generic_requested' |          ///
        `rust_planned_generic_requested'
""",
    "planned Counter requirement",
)
text = replace_once(
    text,
    """    local rust_missing_batch = `rust_generic_requested' &             ///
        !`batch_supplied'
""",
    """    local rust_missing_batch = `rust_generic_requested' &             ///
        !`batch_supplied'
""",
    "legacy batch requirement integrity fence",
)
text = replace_once(
    text,
    """    local rust_unsupported_wall = `wallseconds_supplied' &             ///
        !`rust_exact_requested'
""",
    """    local rust_unsupported_wall = `wallseconds_supplied' &             ///
        !(`rust_exact_requested' | `rust_planned_generic_requested')
""",
    "planned wall support",
)
text = replace_once(
    text,
    """    if `rust_requested' & "`batch_requested'"=="auto" &              ///
        !`rust_exact_requested' {
""",
    """    if `rust_requested' & "`batch_requested'"=="auto" &              ///
        !(`rust_exact_requested' | `rust_planned_generic_requested') {
""",
    "planned automatic batch support",
)

# Pass the planned request metadata into the shared generic lifecycle.
old_call = """                `memorygib' `sample' `deletion_mode' `controls'  ///
                `nuisance' `exactlimit' `blocksize' `ranktol' `blocktol' ///
                `rust_support_flags' `physical_limit' `targetweightsupplied'
"""
new_call = """                `memorygib' `sample' `deletion_mode' `controls'  ///
                `nuisance' `exactlimit' `blocksize' `ranktol' `blocktol' ///
                `rust_support_flags' `physical_limit' `targetweightsupplied' ///
                `rust_planned_generic_requested' `preconditioner'       ///
                `batch_requested' `wallseconds_supplied' `wallseconds'
"""
text = replace_once(text, old_call, new_call, "planned generic call arguments")

path.write_text(text, encoding="utf-8")
