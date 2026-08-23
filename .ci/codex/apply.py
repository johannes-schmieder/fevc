from __future__ import annotations

import re
from pathlib import Path


PATH = Path("varcomp_kss/varcomp_kss.ado")
text = PATH.read_text(encoding="utf-8")


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


legacy_start = "program define _vckss_rust_generic, eclass sortpreserve\n"
impl_start = "program define _vckss_impl, eclass sortpreserve\n"
if text.count(legacy_start) != 1:
    raise RuntimeError("legacy generic program boundary is not unique")
if text.count(impl_start) != 1:
    raise RuntimeError("implementation program boundary is not unique")
if "_vckss_rust_generic_planned" in text:
    raise RuntimeError("planned generic program already exists")

before, remainder = text.split(legacy_start, 1)
legacy_tail, after = remainder.split(impl_start, 1)
legacy = legacy_start + legacy_tail
planned = legacy.replace(
    legacy_start,
    "program define _vckss_rust_generic_planned, eclass sortpreserve\n",
    1,
)

planned = replace_once(
    planned,
    """        targetweightsupplied frequencyused cmdline
""",
    """        targetweightsupplied frequencyused cmdline                 ///
        preconditionerrequested batchrequested wallsecondssupplied wallseconds
""",
    "planned argument surface",
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
    if "`preconditioner_requested'" != "auto" {
        quietly _vckss_post_failure "RUST_OPTION_UNSUPPORTED"       ///
            "The first public planned Rust route requires preconditioner(auto)."
        exit 498
    }
    local batch_request = lower(strtrim("`batchrequested'"))
    if "`batch_request'" == "" local batch_request auto
    local phase_batch_mode auto
    local phase_batch_code = 0
    local solve_batch = 0
    if "`batch_request'" != "auto" {
        capture confirm integer number `batch_request'
        if _rc | real("`batch_request'") <= 0 {
            quietly _vckss_post_failure "INVALID_TUNING"           ///
                "The planned Rust route requires batch(auto) or a positive integer."
            exit 198
        }
        local phase_batch_mode explicit
        local phase_batch_code = 1
        local solve_batch = real("`batch_request'")
    }
    local fallback_allowed = 1
    local wallseconds_supplied_code = real("`wallsecondssupplied'")
    if !inlist(`wallseconds_supplied_code',0,1) {
        quietly _vckss_post_failure "INVALID_WALL_ENVELOPE"
        exit 198
    }
    local wallseconds_value = 0
    if `wallseconds_supplied_code' {
        local wallseconds_value = real("`wallseconds'")
        if missing(`wallseconds_value') | `wallseconds_value' <= 0 {
            quietly _vckss_post_failure "INVALID_WALL_ENVELOPE"
            exit 198
        }
    }

    capture quietly varcomp_kss_rust clear
""",
    "planned request locals",
)

capability_block = r"""    capture quietly _vckss_rust_public_call requestcapability,       ///
        algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
        route(auto) rngcontract(counter_v1)                          ///
        controls(`control_count') frequencyused(`frequency_code')    ///
        engine(generic) batchmode(`phase_batch_mode')                 ///
        leveragebatchmode(`phase_batch_mode')                        ///
        targetbatchmode(`phase_batch_mode') stayers(movers)          ///
        targetweightmode(`target_mode') deletionsource(`deletion_source') ///
        probeordersupplied(0) wallsecondssupplied(`wallseconds_supplied_code') ///
        physicallimit(`physicallimit') fallback(`fallback_allowed')  ///
        wallseconds(`wallseconds_value')
    if _rc {
        capture quietly varcomp_kss_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "planned generic-JLA route could not obtain a V3 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNAVAILABLE"      ///
            "The planned generic-JLA request-capability query was unavailable; no native preparation was attempted."
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
        automatic_fallback_allowed algorithm_resolution_deferred     ///
        engine_resolution_deferred route_resolution_deferred         ///
        leverage_batch_deferred target_batch_resolution_deferred     ///
        wall_advisory_only {
        local cap_`name' = r(`name')
    }
    local cap_wallseconds = r(wallseconds)
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
    if missing(`cap_wallseconds') | `cap_wallseconds' < 0 {
        local capability_ok = 0
    }
    if `capability_ok' {
        local capability_ok =                                      ///
            `cap_struct_size' == 160 & `cap_abi_version' == 1 &    ///
            `cap_request_schema' == 3 & `cap_supported' == 1 &     ///
            `cap_reason_code' == 0 & `cap_profile_code' == 4 &     ///
            `cap_algorithm_code' == 2 &                            ///
            `cap_deletion_mode_code' == `deletion_code' &          ///
            `cap_nuisance_mode_code' == `nuisance_code' &          ///
            `cap_solver_route_code' == 0 & `cap_rng_contract_code' == 1 & ///
            `cap_controls_count' == `control_count' &              ///
            `cap_frequency_use_code' == `frequency_code' &         ///
            `cap_engine_code' == 2 &                               ///
            `cap_batch_mode_code' == `phase_batch_code' &          ///
            `cap_leverage_batch_mode_code' == `phase_batch_code' & ///
            `cap_target_batch_mode_code' == `phase_batch_code' &   ///
            `cap_stayers_mode_code' == 1 &                         ///
            `cap_target_weight_mode_code' == `target_code' &       ///
            `cap_deletion_source_code' == `deletion_source_code' & ///
            `cap_probeorder_supplied' == 0 &                       ///
            `cap_wallseconds_supplied' == `wallseconds_supplied_code' & ///
            `cap_physical_limit' == `physicallimit' &              ///
            `cap_automatic_fallback_allowed' == 1 &                ///
            `cap_wallseconds' == `wallseconds_value' &             ///
            `cap_wall_advisory_only' == 1 &                        ///
            `cap_request_signature_hi' <= 4294967295 &             ///
            `cap_request_signature_lo' <= 4294967295
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

    tempvar rust_keep"""

planned = sub_once(
    planned,
    r"    capture quietly _vckss_rust_public_call requestcapability,.*?\n    tempvar rust_keep",
    capability_block,
    "planned capability boundary",
)

solve_block = r"""    capture noisily _vckss_rust_public_call solve `handle',         ///
        seed(`seed') probes(`probes') leveragebatch(`solve_batch')   ///
        targetbatch(`solve_batch') route(auto) tolerance(`tolerance') ///
        maxiter(`maxiter') algorithm(jla) deletion(`deletionmode')  ///
        nuisance(`nuisance') exactlimit(`exactlimit')               ///
        blocksizelimit(`blocksizelimit') ranktolerance(`ranktol')   ///
        blocktolerance(`blocktol') engine(generic)                  ///
        batchmode(`phase_batch_mode')                               ///
        leveragebatchmode(`phase_batch_mode')                       ///
        targetbatchmode(`phase_batch_mode') stayers(movers)         ///
        targetweightmode(`target_mode')                             ///
        deletionsource(`deletion_source') probeordersupplied(0)     ///
        wallsecondssupplied(`wallseconds_supplied_code')            ///
        wallseconds(`wallseconds_value') physicallimit(`physicallimit') ///
        capabilityschema(3) capabilityprofile(4)                    ///
        frequencyused(`frequency_code')                             ///
        signaturehi(`cap_request_signature_hi')                     ///
        signaturelo(`cap_request_signature_lo')                     ///
        fallback(`fallback_allowed')
    if _rc {"""

planned = sub_once(
    planned,
    r"    capture noisily _vckss_rust_public_call solve `handle',.*?\n    if _rc \{",
    solve_block,
    "planned solve V4 dispatch",
)

plan_capture = r"""    tempname raw_results rhs_native plan_receipt
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
    local plan_fields plan_struct plan_schema plan_alg_schema plan_alg_req ///
        plan_alg_sel plan_alg_reason plan_eng_schema plan_eng_req          ///
        plan_eng_sel plan_eng_reason plan_comp_elig plan_resolved          ///
        plan_eng_fallback plan_complexity plan_exact_limit                 ///
        plan_route_schema plan_route_req plan_route_sel plan_route_fallback ///
        plan_route_error plan_full_setup plan_fe_setup plan_fe_reuse       ///
        plan_frozen plan_route_contract plan_threads_req plan_threads_used ///
        plan_parallel plan_applicability plan_rhs plan_auto_firms          ///
        plan_auto_rhs plan_full_dim plan_fe_dim batch_schema batch_determ  ///
        batch_invariant batch_arithmetic batch_admitted batch_app          ///
        batch_nonbatched batch_command wall_schema wall_model wall_status  ///
        wall_routing wall_req_app wall_fcst_app wall_adv_app wall_margin_app ///
        wall_requested wall_forecast wall_advisory wall_margin wall_prepare ///
        wall_setup wall_fit wall_leverage wall_target wall_export wall_total ///
        ctr_schema ctr_rng ctr_work_app ctr_complete mem_schema mem_app     ///
        mem_phase mem_hard mem_prepared mem_setup mem_fit mem_correction   ///
        mem_leverage mem_target mem_result mem_nonbatched mem_command      ///
        mem_shared_cmg mem_control mem_transient mem_cmg_workspace         ///
        mem_cmg_cells mem_cmg_groups mem_cmg_graph mem_nq mem_q2           ///
        plan_app_hi plan_app_lo plan_contract_hi plan_contract_lo          ///
        plan_sig_hi plan_sig_lo plan_res_rng_hi plan_res_rng_lo            ///
        plan_res_ctr_hi plan_res_ctr_lo plan_pre_atom_hi plan_pre_atom_lo  ///
        plan_pre_word_hi plan_pre_word_lo plan_pre_trial_hi plan_pre_trial_lo ///
        ctr_pre_atom_hi ctr_pre_atom_lo ctr_pre_word_hi ctr_pre_word_lo    ///
        ctr_pre_trial_hi ctr_pre_trial_lo
    foreach phase in lev tgt {
        local plan_fields `plan_fields' batch_`phase'_mode             ///
            batch_`phase'_reason batch_`phase'_app batch_`phase'_req   ///
            batch_`phase'_sel batch_`phase'_probe batch_`phase'_threads ///
            batch_`phase'_threadcap batch_`phase'_routecap             ///
            batch_`phase'_effcap batch_`phase'_hard                    ///
            batch_`phase'_onebytes batch_`phase'_selbytes
    }
    foreach phase in lev tgt all {
        foreach field in pla ala puw auw ppt apt pgw agw {
            local plan_fields `plan_fields' ctr_`phase'_`field'_hi    ///
                ctr_`phase'_`field'_lo
        }
    }
    local plan_count : word count `plan_fields'
    matrix `plan_receipt' = J(1,`plan_count',.)
    local plan_column = 0
    local plan_values_ok = 1
    foreach field of local plan_fields {
        local ++plan_column
        capture local plan_value = r(`field')
        if _rc | missing(`plan_value') local plan_values_ok = 0
        else matrix `plan_receipt'[1,`plan_column'] = `plan_value'
    }
    matrix colnames `plan_receipt' = `plan_fields'
    local plan_receipt_schema `"`r(receipt_schema)'"'
    local r_plan_struct = r(plan_struct)
    local r_plan_schema = r(plan_schema)
    local r_plan_route_schema = r(plan_route_schema)
    local r_plan_route_req = r(plan_route_req)
    local r_plan_route_sel = r(plan_route_sel)
    local r_plan_route_fallback = r(plan_route_fallback)
    local r_plan_route_error = r(plan_route_error)
    local r_plan_lev_mode = r(batch_lev_mode)
    local r_plan_tgt_mode = r(batch_tgt_mode)
    local r_plan_lev_sel = r(batch_lev_sel)
    local r_plan_tgt_sel = r(batch_tgt_sel)
    local r_plan_mem_command = r(mem_command)
    local r_plan_mem_hard = r(mem_hard)
    local r_plan_mem_prepared = r(mem_prepared)
    local r_plan_threads_req = r(plan_threads_req)
    local r_plan_threads_used = r(plan_threads_used)
    local r_plan_parallel = r(plan_parallel)
    local r_wall_requested_value = r(wall_requested)
    local r_wall_forecast_value = r(wall_forecast)
    local r_wall_advisory_value = r(wall_advisory)
    local r_wall_margin_value = r(wall_margin)
    local r_wall_total_value = r(wall_total)
    local r_wall_phase_sum = r(wall_prepare)+r(wall_setup)+r(wall_fit)+ ///
        r(wall_leverage)+r(wall_target)+r(wall_export)"""

planned = replace_once(
    planned,
    """    tempname raw_results rhs_native
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
""",
    plan_capture + "\n",
    "planned V7 capture",
)

planned = planned.replace(
    "`rhs_native'[`row',4]!=2",
    "`rhs_native'[`row',4]!=`r_sel_route'",
)
if "`rhs_native'[`row',4]!=2" in planned:
    raise RuntimeError("RHS route replacement was incomplete")
print("replaced planned RHS selected-route checks")

planned = replace_once(
    planned,
    """        local batch_start = cond(`probe'<0,1,floor(`probe'/`batch')*`batch'+1)
""",
    """        local active_batch = cond(`phase'==2,`r_lev_batch',       ///
            cond(`phase'==3,`r_tgt_batch',1))
        local batch_start = cond(`probe'<0,1,                       ///
            floor(`probe'/`active_batch')*`active_batch'+1)
""",
    "phase-specific public batch starts",
)

main_predicate = r"""    if `results_ok' {
        local results_ok =                                         ///
            `plan_values_ok' &                                    ///
            `"`plan_receipt_schema'"'=="VCKSS-EXECUTION-PLAN-V1" & ///
            `r_plan_struct'==1000 & `r_plan_schema'==1 &           ///
            `r_plan_route_schema'==2 &                             ///
            `r_seed'==`seed' & `r_probes'==`probes' &              ///
            `r_lev_acc'==`probes' & `r_tgt_acc'==`probes' &        ///
            `r_req_route'==0 & inlist(`r_sel_route',2,3) &          ///
            `r_plan_route_req'==`r_req_route' &                     ///
            `r_plan_route_sel'==`r_sel_route' &                     ///
            `r_plan_route_fallback'==`r_fallback' &                 ///
            `r_plan_route_error'==`r_fallback_err' &                ///
            inlist(`r_fallback',0,1) &                              ///
            (`r_fallback' | `r_fallback_err'==0) &                  ///
            `r_dimension'==`p_firms'+`control_count' &              ///
            `r_lev_batch'>=1 & `r_lev_batch'<=`probes' &            ///
            `r_tgt_batch'>=1 & `r_tgt_batch'<=`probes' &            ///
            `r_plan_lev_mode'==`phase_batch_code' &                 ///
            `r_plan_tgt_mode'==`phase_batch_code' &                 ///
            `r_plan_lev_sel'==`r_lev_batch' &                       ///
            `r_plan_tgt_sel'==`r_tgt_batch' &                       ///
            (`phase_batch_code'==0 |                               ///
                (`r_lev_batch'==`solve_batch' &                     ///
                 `r_tgt_batch'==`solve_batch')) &                   ///
            `r_rank_tol'==`ranktol' & `r_block_tol'==`blocktol' &  ///
            `r_full_tol'==`expected_full_tol' &                     ///
            `r_full_route'==`r_sel_route' &                         ///
            `r_full_iter'>=0 & `r_full_iter'<=`maxiter' &           ///
            `r_full_red'>=0 & `r_full_complete'>=0 &                ///
            `r_full_complete'<=`r_full_tol' & inlist(`r_full_zero',0,1) & ///
            `r_lev_rhs'==`probes' & `r_tgt_rhs'==2*`probes' &       ///
            `r_max_red'==`rhs_max_reduced' &                        ///
            `r_max_complete'==`rhs_max_complete' &                  ///
            `r_max_complete'<=`r_full_tol' &                        ///
            `r_max_lev'>=0 & `r_max_lev'<1 & `r_max_recip'>=0 &    ///
            `r_max_recip'<=`maker_gate' &                           ///
            `r_rng'==1 & `r_rhs_rows'==`expected_rhs_rows' &        ///
            `r_rhs_copy'==0 & `r_rhs_schema'==2 &                   ///
            `r_algorithm_req'==2 & `r_algorithm_sel'==2 &           ///
            `r_deletion'==`deletion_code' &                         ///
            `r_nuisance'==`nuisance_code' &                         ///
            `r_parameters'==`expected_parameters' &                 ///
            `r_full_parameters'==`expected_full_parameters' &       ///
            `r_correction_parameters'==`expected_parameters' &      ///
            `r_native_info'==0 & `r_native_inverse'==0 &            ///
            `r_exact_flags'==256 & `r_engine_req'==2 & `r_engine_sel'==2 & ///
            `r_generic_flags'==`expected_flags' &                    ///
            `r_generic_controls'==`control_count' &                 ///
            `r_control_rhs'==`control_count' &                       ///
            `r_full_joint'==`r_full_complete' &                     ///
            `r_working_fit'>=0 & `r_working_fit'<=`r_full_tol' &    ///
            `r_maker'==`r_max_recip' & `r_rank_gap'>0 &             ///
            `r_schur_rcond'>0 & `r_schur_rcond'<=1 &                ///
            `r_schur_relres'>=0 & `r_control_basis'>=0 &            ///
            `r_control_forward'>=0 &                                ///
            scalar(`native_cr_tol')==max(`ranktol',1e-12) &         ///
            scalar(`native_cr_pcg')==1e-13 &                       ///
            scalar(`native_cr_gate')==1e-11 &                      ///
            `r_cap_schema'==3 & `r_cap_profile'==4 &                ///
            `r_batch_mode'==`phase_batch_code' &                    ///
            `r_stayers_mode'==1 & `r_target_mode'==`target_code' &  ///
            `r_deletion_source'==`deletion_source_code' &           ///
            `r_probeorder'==0 &                                     ///
            `r_wallseconds'==`wallseconds_supplied_code' &          ///
            `r_frequency'==`frequency_code' &                       ///
            `r_physical_limit'==`physicallimit' &                   ///
            `r_signature_hi'==`cap_request_signature_hi' &          ///
            `r_signature_lo'==`cap_request_signature_lo' &          ///
            `r_mem_limit'==`p_mem_limit' &                          ///
            `r_input_copy'==`p_input_copy' &                        ///
            `r_prep_peak'==`p_prep_peak' &                          ///
            `r_resident'==`p_resident' &                            ///
            `r_rhs_v2_copy'==216*`expected_rhs_rows' &              ///
            `r_result_bytes'>=`r_rhs_v2_copy' &                     ///
            `r_plan_mem_hard'==`r_mem_limit' &                      ///
            `r_plan_mem_prepared'==`r_resident' &                   ///
            `r_plan_mem_command'==`r_solve_peak' &                  ///
            `r_command_peak'==max(`r_prep_peak',`r_solve_peak') &   ///
            `r_command_peak'<=`r_mem_limit' &                       ///
            `r_plan_threads_req'>=1 & `r_plan_threads_used'>=1 &    ///
            `r_plan_threads_used'<=`r_plan_threads_req' &           ///
            `r_plan_parallel'==(`r_plan_threads_used'>1) &          ///
            `r_wall_requested_value'==`wallseconds_value' &         ///
            `r_wall_total_value'==`r_wall_phase_sum' &              ///
            abs(`r_actual_accounting'-`accounting_truth')<=         ///
                `roundoff_gate'*max(1,abs(`accounting_truth')) &    ///
            abs(`r_accounting'-`r_actual_accounting')<=             ///
                `roundoff_gate'*max(1,abs(`r_actual_accounting'))
    }
    if `results_ok' & `control_count'>0 {"""

planned = sub_once(
    planned,
    r"    if `results_ok' \{\n        local results_ok =\s+///\n            `r_seed'==`seed'.*?\n    \}\n    if `results_ok' & `control_count'>0 \{",
    main_predicate,
    "planned result reconciliation",
)

planned = planned.replace(
    "Generic-JLA V6/RHS-V2 result receipts did not reconcile with the submitted request.",
    "Planned generic-JLA V7/RHS-V2 result receipts did not reconcile with the submitted request.",
)
planned = replace_once(
    planned,
    """    ereturn matrix rust_control_rank_receipt = `control_rank_receipt'
""",
    """    ereturn matrix rust_control_rank_receipt = `control_rank_receipt'
    ereturn matrix rust_execution_plan = `plan_receipt'
""",
    "public V7 execution-plan matrix",
)
planned = replace_once(
    planned,
    """    ereturn scalar batch = `batch'
""",
    """    ereturn scalar batch = max(`r_lev_batch',`r_tgt_batch')
""",
    "public selected planned batch",
)
planned = replace_once(
    planned,
    """    ereturn scalar rust_batch_mode_code = `r_batch_mode'
""",
    """    ereturn scalar rust_batch_mode_code = `r_batch_mode'
    ereturn scalar rust_leverage_batch_mode_code = `r_plan_lev_mode'
    ereturn scalar rust_target_batch_mode_code = `r_plan_tgt_mode'
    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_plan_threads_requested = `r_plan_threads_req'
    ereturn scalar rust_plan_threads_used = `r_plan_threads_used'
    ereturn scalar rust_plan_parallel = `r_plan_parallel'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested_value'
    ereturn scalar rust_wallseconds_forecast = `r_wall_forecast_value'
    ereturn scalar rust_wallseconds_advisory = `r_wall_advisory_value'
    ereturn scalar rust_wallseconds_margin = `r_wall_margin_value'
    ereturn scalar rust_plan_solve_peak_forecast_bytes = `r_plan_mem_command'
    ereturn scalar rust_planned_mode = 1
""",
    "public planned summary scalars",
)
planned = replace_once(
    planned,
    """    ereturn scalar rust_cap_signature_hi = `cap_request_signature_hi'
    ereturn scalar rust_cap_signature_lo = `cap_request_signature_lo'
""",
    """    ereturn scalar rust_cap_signature_hi = `cap_request_signature_hi'
    ereturn scalar rust_cap_signature_lo = `cap_request_signature_lo'
    ereturn scalar rust_cap_leverage_batch_mode_code =             ///
        `cap_leverage_batch_mode_code'
    ereturn scalar rust_cap_target_batch_mode_code =               ///
        `cap_target_batch_mode_code'
    ereturn scalar rust_cap_automatic_fallback_allowed =           ///
        `cap_automatic_fallback_allowed'
    ereturn scalar rust_cap_wallseconds = `cap_wallseconds'
    ereturn scalar rust_cap_algorithm_resolution_deferred =        ///
        `cap_algorithm_resolution_deferred'
    ereturn scalar rust_cap_engine_resolution_deferred =           ///
        `cap_engine_resolution_deferred'
    ereturn scalar rust_cap_route_resolution_deferred =            ///
        `cap_route_resolution_deferred'
    ereturn scalar rust_cap_leverage_batch_deferred =              ///
        `cap_leverage_batch_deferred'
    ereturn scalar rust_cap_target_batch_resolution_deferred =     ///
        `cap_target_batch_resolution_deferred'
    ereturn scalar rust_cap_wall_advisory_only = `cap_wall_advisory_only'
""",
    "public planned capability extras",
)
planned = replace_once(
    planned,
    """    ereturn local backend_routing_reason "fully explicit public generic-JLA route"
""",
    """    ereturn local backend_routing_reason                     ///
        "explicit Rust planned generic-JLA route"
""",
    "public planned backend reason",
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
    ereturn local routing_reason "native V7 planned generic-JLA route"
    ereturn local fallback_status = cond(`r_fallback',"CMG_TO_DIAGONAL", ///
        "ELIGIBLE_NOT_USED")
    ereturn local fallback_message = cond(`r_fallback',              ///
        "automatic CMG setup fell back to diagonal before Counter-V1 addressing", ///
        "automatic route completed without fallback")
    ereturn local batch_requested "`batch_request'"
    ereturn local batch_routing_reason = cond(`phase_batch_code'==0, ///
        "native automatic phase batching","caller supplied explicit phase width")
""",
    "public planned route metadata",
)
planned = replace_once(
    planned,
    """    ereturn local route_api "VCKSS-NATIVE-GENERIC-V3-V6"
    ereturn local rust_capability_profile "JLA_GENERIC_COUNTER_V1"
""",
    """    ereturn local route_api "VCKSS-NATIVE-PLANNED-V4-V7"
    ereturn local rust_capability_profile "PLANNED_V1"
""",
    "public planned route API",
)

# Insert the separate program immediately before the main implementation.
text = before + legacy + planned + impl_start + after

# Add a narrow public support predicate without changing the frozen legacy route.
text = replace_once(
    text,
    """        local rust_exact_supported =                           ///
            `algorithm_supplied' & "`algorithm'" == "exact" & ///
""",
    """        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &         ///
            `engine_supplied' & "`engine_requested'" == "generic" & ///
            `preconditioner_supplied' & "`preconditioner'" == "auto" & ///
            `rng_supplied' & "`rng_requested'" == "counter_v1" &   ///
            "`stayers'" == "movers" & "`probeorder'" == ""
        local rust_exact_supported =                           ///
            `algorithm_supplied' & "`algorithm'" == "exact" & ///
""",
    "planned public support predicate",
)
text = replace_once(
    text,
    """        local rust_options_supported =                         ///
            `rust_legacy_jla_supported' | `rust_generic_supported' | ///
            `rust_exact_supported'
""",
    """        local rust_options_supported =                         ///
            `rust_legacy_jla_supported' | `rust_generic_supported' | ///
            `rust_planned_generic_supported' | `rust_exact_supported'
""",
    "planned public support union",
)

dispatch_old = r"""        else if `rust_generic_requested' {
            capture noisily _vckss_rust_generic `depvar'          ///
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
                `targetweight_supplied' `rust_frequency_used' `"`cmdline'"'
        }"""

dispatch_new = r"""        else if `rust_planned_generic_supported' {
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
                `wallseconds_supplied' `wallseconds'
        }
        else if `rust_generic_requested' {
            capture noisily _vckss_rust_generic `depvar'          ///
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
                `targetweight_supplied' `rust_frequency_used' `"`cmdline'"'
        }"""

text = replace_once(text, dispatch_old, dispatch_new, "planned public dispatch")

PATH.write_text(text, encoding="utf-8")
print("planned generic Rust route staged successfully")
