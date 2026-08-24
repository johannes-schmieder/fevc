from __future__ import annotations

import json
from pathlib import Path


QUALIFICATION = Path(".ci/stata/qualifications/planned-auto-exact-v7.json")
if not QUALIFICATION.is_file():
    raise SystemExit(f"missing prerequisite qualification {QUALIFICATION}")
qualification = json.loads(QUALIFICATION.read_text(encoding="utf-8"))
if (
    qualification.get("source_sha")
    != "3be7a1f3220547f04397848084b01023d9d96aef"
    or qualification.get("status") != "success"
    or qualification.get("process_rc") != 0
    or qualification.get("stata_rc") != 0
    or qualification.get("tests_failed") != 0
):
    raise SystemExit(f"auto-exact bridge qualification is not green: {qualification}")


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


ado_path = Path("varcomp_kss/varcomp_kss.ado")
ado = ado_path.read_text(encoding="utf-8")
rexact_start = ado.index("program define _vckss_rexact, eclass sortpreserve")
rexact_end = ado.index("\nend\n", rexact_start)
post_start = ado.index(
    "    tempname plugin correction corrected kss_return mcse decomposition\n",
    rexact_start,
)
post_block = ado[post_start:rexact_end]

cap_start = post_block.index("    matrix `capability_receipt' =")
cap_end = post_block.index("\n\n    ereturn clear", cap_start)
capability_block = r'''    matrix `capability_receipt' = (`capstruct',`capabi',       ///
        `capschema',`capsupported',`capreason',`capprofile',             ///
        `capalgorithm',`capdeletion',`capnuisance',`caproute',`caprng',  ///
        `capcontrols',`capfrequency',`capengine',`capbatch',             ///
        `capstayers',`captarget',`capdelsource',`capprobeorder',         ///
        `capwallsup',`capphysical',`capsignaturehi',`capsignaturelo')
    matrix colnames `capability_receipt' = struct_size abi_version schema ///
        supported reason profile algorithm deletion nuisance route rng   ///
        controls frequency engine batch stayers target deletion_source  ///
        probeorder wall physical_limit signature_hi signature_lo'''
post_block = post_block[:cap_start] + capability_block + post_block[cap_end:]

post_block = post_block.replace(
    "    ereturn scalar correction_parameters = `r_correction_parameters'\n",
    "    ereturn scalar correction_parameters = `r_correction_parameters'\n"
    "    ereturn scalar controls_count = `p_controls'\n",
    1,
)
post_block = post_block.replace(
    "    ereturn scalar route_planned_rhs = 0\n",
    r'''    ereturn scalar route_planned_rhs = 0
    ereturn scalar rust_requested_algorithm_code = `r_algorithm_req'
    ereturn scalar rust_selected_algorithm_code = `r_algorithm_sel'
    ereturn scalar rust_requested_engine_code = `r_engine_req'
    ereturn scalar rust_selected_engine_code = `r_engine_sel'
    ereturn scalar rust_plan_struct_size = `r_plan_struct'
    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_plan_resolved = `r_plan_resolved'
    ereturn scalar rust_plan_frozen = `r_plan_frozen'
    ereturn scalar rust_plan_applicability = `r_plan_applicability'
    ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
    ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
    ereturn scalar rust_plan_engine_requested = `r_plan_eng_req'
    ereturn scalar rust_plan_engine_selected = `r_plan_eng_sel'
    ereturn scalar rust_plan_route_requested = `r_plan_route_req'
    ereturn scalar rust_plan_route_selected = `r_plan_route_sel'
    ereturn scalar rust_plan_route_fallback = `r_plan_route_fallback'
    ereturn scalar rust_plan_route_error = `r_plan_route_error'
    ereturn scalar rust_plan_rhs = `r_plan_rhs'
    ereturn scalar rust_plan_full_dimension = `r_plan_full_dim'
    ereturn scalar rust_plan_leverage_batch = `r_plan_lev_batch'
    ereturn scalar rust_plan_target_batch = `r_plan_tgt_batch'
    ereturn scalar rust_plan_solve_peak_bytes = `r_plan_mem_command'
    ereturn scalar rust_counter_plan_complete = `r_ctr_complete'
    ereturn scalar rust_pre_rng_hi = `r_pre_rng_hi'
    ereturn scalar rust_pre_rng_lo = `r_pre_rng_lo'
    ereturn scalar rust_wall_request_applicable = `r_wall_req_app'
    ereturn scalar rust_wallseconds_supplied = `r_wallseconds'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested'
    ereturn scalar rust_wallseconds_forecast = `r_wall_forecast'
    ereturn scalar rust_wallseconds_advisory = `r_wall_advisory'
    ereturn scalar rust_wallseconds_margin = `r_wall_margin'
    ereturn scalar rust_batch_mode_code = `r_batch_mode'
    ereturn scalar rust_leverage_batch_mode_code = `r_lev_batch_mode'
    ereturn scalar rust_target_batch_mode_code = `r_tgt_batch_mode'
''',
    1,
)
post_block = post_block.replace(
    "    ereturn scalar rust_cap_signature_lo = `capsignaturelo'\n",
    r'''    ereturn scalar rust_cap_signature_lo = `capsignaturelo'
    ereturn scalar rust_cap_algorithm_deferred = `capalgdefer'
    ereturn scalar rust_cap_engine_deferred = `capengdefer'
    ereturn scalar rust_cap_route_deferred = `caproutedefer'
    ereturn scalar rust_cap_leverage_batch_deferred = `caplevdefer'
    ereturn scalar rust_cap_target_batch_deferred = `captgtdefer'
''',
    1,
)
post_block = post_block.replace(
    "    ereturn scalar rng_option_supplied = `rngsupplied'\n"
    "    ereturn scalar deletionid_option_supplied = `deletionidsupplied'\n",
    r'''    ereturn scalar rng_option_supplied = `rngsupplied'
    ereturn scalar algorithm_option_supplied = `algorithmsupplied'
    ereturn scalar engine_option_supplied = `enginesupplied'
    ereturn scalar preconditioner_option_supplied = `preconditionsupplied'
    ereturn scalar batch_option_supplied = `batchsupplied'
    ereturn scalar stayers_option_supplied = `stayerssupplied'
    ereturn scalar deletionid_option_supplied = `deletionidsupplied'
    ereturn scalar targetweight_option_supplied = `targetweightsupplied'
''',
    1,
)
post_block = post_block.replace(
    "    ereturn local cmd \"varcomp_kss\"\n",
    "    ereturn local cmd \"varcomp_kss\"\n"
    "    ereturn local cmdline `\"`cmdline'\"'\n",
    1,
)
post_block = post_block.replace(
    '        "explicit Rust exact route; RNG is not applicable"',
    '        "explicit planned Rust algorithm-auto route selected exact; RNG is not applicable"',
    1,
)
post_block = post_block.replace(
    "    ereturn local algorithm \"exact\"\n",
    "    ereturn local algorithm_requested \"`algorithmrequested'\"\n"
    "    ereturn local algorithm \"exact\"\n",
    1,
)
post_block = post_block.replace(
    '    ereturn local rust_capability_profile "EXACT_V1"\n',
    '    ereturn local rust_capability_profile "PLANNED_V1"\n'
    '    ereturn local execution_plan_schema "VCKSS-EXECUTION-PLAN-V1"\n'
    '    ereturn local route_api "VCKSS-NATIVE-EXACT-PLANNED-V4-V7"\n',
    1,
)

preamble = r'''*! version 0.3.0-dev 24aug2026
program define _vckss_rust_post_exact_v7, eclass sortpreserve
    version 18.0
    args handle depvar frequency target touse nscope ncomplete nstayers ///
        nstayerrows probesrequested batchnumeric seedrequested         ///
        tolerancerequested maxiterrequested memorygib algorithmrequested ///
        enginerequested backendsupplied rngsupplied rngrequested       ///
        deletionidsupplied enginesupplied algorithmsupplied            ///
        preconditionsupplied batchsupplied stayerssupplied             ///
        rustcoreflags rustsupportflags nodisplay deletionmode nuisance ///
        ranktol blocktol physicallimit preconditionerrequested         ///
        batchrequested targetweightsupplied cmdline wallsecondssupplied ///
        wallseconds prepctx graphctx capctx

    foreach input in `depvar' `frequency' `target' `touse' {
        confirm numeric variable `input'
    }
    tempname prep graph cap
    matrix `prep' = `prepctx'
    matrix `graph' = `graphctx'
    matrix `cap' = `capctx'
    local context_ok = rowsof(`prep')==1 & colsof(`prep')==13 &       ///
        rowsof(`graph')==1 & colsof(`graph')==18 &                    ///
        rowsof(`cap')==1 & colsof(`cap')==33
    if !`context_ok' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact poster received an invalid context shape."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_context"
        exit 498
    }

    local p_input       = `prep'[1,1]
    local p_retained    = `prep'[1,2]
    local p_workers     = `prep'[1,3]
    local p_firms       = `prep'[1,4]
    local p_cells       = `prep'[1,5]
    local p_units       = `prep'[1,6]
    local p_strata      = `prep'[1,7]
    local p_target      = `prep'[1,8]
    local p_controls    = `prep'[1,9]
    local p_mem_limit   = `prep'[1,10]
    local p_input_copy  = `prep'[1,11]
    local p_prep_peak   = `prep'[1,12]
    local p_resident    = `prep'[1,13]

    local g_input_rows      = `graph'[1,1]
    local g_keep_rows       = `graph'[1,2]
    local g_input_mass      = `graph'[1,3]
    local g_keep_mass       = `graph'[1,4]
    local g_init_comp       = `graph'[1,5]
    local g_max_comp        = `graph'[1,6]
    local g_init_rows       = `graph'[1,7]
    local g_mover_rows      = `graph'[1,8]
    local g_init_edges      = `graph'[1,9]
    local g_keep_edges      = `graph'[1,10]
    local g_degree_removed  = `graph'[1,11]
    local g_art_removed     = `graph'[1,12]
    local g_bridge_units    = `graph'[1,13]
    local g_bridge_rows     = `graph'[1,14]
    local g_degree_iters    = `graph'[1,15]
    local g_art_iters       = `graph'[1,16]
    local g_bridge_iters    = `graph'[1,17]
    local g_fixed_iters     = `graph'[1,18]

    local capstruct       = `cap'[1,1]
    local capabi          = `cap'[1,2]
    local capschema       = `cap'[1,3]
    local capsupported    = `cap'[1,4]
    local capreason       = `cap'[1,5]
    local capprofile      = `cap'[1,6]
    local capalgorithm    = `cap'[1,7]
    local capdeletion     = `cap'[1,8]
    local capnuisance     = `cap'[1,9]
    local caproute        = `cap'[1,10]
    local caprng          = `cap'[1,11]
    local capcontrols     = `cap'[1,12]
    local capfrequency    = `cap'[1,13]
    local capengine       = `cap'[1,14]
    local capbatch        = `cap'[1,15]
    local capstayers      = `cap'[1,16]
    local captarget       = `cap'[1,17]
    local capdelsource    = `cap'[1,18]
    local capprobeorder   = `cap'[1,19]
    local capwallsup      = `cap'[1,20]
    local capphysical     = `cap'[1,21]
    local capsignaturehi  = `cap'[1,22]
    local capsignaturelo  = `cap'[1,23]
    local caplevmode      = `cap'[1,24]
    local captgtmode      = `cap'[1,25]
    local capfallback     = `cap'[1,26]
    local capalgdefer     = `cap'[1,27]
    local capengdefer     = `cap'[1,28]
    local caproutedefer   = `cap'[1,29]
    local caplevdefer     = `cap'[1,30]
    local captgtdefer     = `cap'[1,31]
    local capwalladvisory = `cap'[1,32]
    local capwallseconds  = `cap'[1,33]

    local algorithmrequested = lower(strtrim("`algorithmrequested'"))
    local enginerequested = lower(strtrim("`enginerequested'"))
    local preconditionerrequested = lower(strtrim("`preconditionerrequested'"))
    local batchrequested = lower(strtrim("`batchrequested'"))
    local deletionmode = lower(strtrim("`deletionmode'"))
    local nuisance = lower(strtrim("`nuisance'"))
    local algreq = cond("`algorithmrequested'"=="auto",0,.)
    local engreq = cond("`enginerequested'"=="auto",0,               ///
        cond("`enginerequested'"=="generic",2,.))
    local delcode = cond("`deletionmode'"=="match",1,               ///
        cond("`deletionmode'"=="observation",2,.))
    local nuiscode = cond("`nuisance'"=="joint",1,                  ///
        cond("`nuisance'"=="fixedoffset",2,.))
    local wallsup = real("`wallsecondssupplied'")
    local wallvalue = real("`wallseconds'")
    local expected_batch_mode = cond("`batchrequested'"=="auto",0,1)
    local expected_input_copy = `p_input'*(6+`p_controls')*8
    local expected_prep_peak = `expected_input_copy'+`p_input'*768+ ///
        `p_input'*`p_controls'*32+4096
    local tuple_ok = !missing(`algreq') & !missing(`engreq') &       ///
        !missing(`delcode') & !missing(`nuiscode') &                 ///
        "`preconditionerrequested'"=="auto" &                       ///
        inlist("`batchrequested'","auto","`batchnumeric'") &      ///
        inlist(`wallsup',0,1) &                                     ///
        ((`wallsup'==0 & `wallvalue'==0) |                          ///
            (`wallsup'==1 & `wallvalue'>0))
    if !`tuple_ok' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact poster received an invalid expected tuple."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_tuple"
        exit 498
    }

    capture noisily _vckss_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(exact_result_export)
        exit _rc
    }
    capture quietly _vckss_rust_reconcile_exact_v7 `algreq' `engreq' ///
        `delcode' `nuiscode' `p_workers' `p_firms' `p_controls'      ///
        `ranktol' `blocktol' `tolerancerequested' `p_mem_limit'      ///
        `p_input_copy' `p_prep_peak' `p_resident' `capsignaturehi'   ///
        `capsignaturelo' `physicallimit' `wallsup' `wallvalue'       ///
        `captarget' `capdelsource' `capfrequency'
    local reconcile_rc = _rc
    if `reconcile_rc' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact V7 reconciler was unavailable."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_reconcile"
        exit 498
    }
    local reconcile_ok = r(ok)
    local reconcile_detail `"`r(detail)'"'
    tempname validated_results
    matrix `validated_results' = r(result)
    if `reconcile_ok'!=1 | `"`r(result_family)'"'!="exact" |        ///
        `"`r(execution_plan_schema)'"'!="VCKSS-EXECUTION-PLAN-V1" {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "Planned exact V7 reconciliation failed: `reconcile_detail'."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_reconcile"
        exit 498
    }

    capture noisily _vckss_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(exact_result_reexport)
        exit _rc
    }
    tempname raw_results
    matrix `raw_results' = r(result)
    if mreldif(`raw_results',`validated_results') != 0 {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The immutable exact generation changed between reconciliation and posting."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_reexport"
        exit 498
    }

    foreach pair in seed:r_seed probes:r_probes                       ///
        leverage_probes_accepted:r_lev_acc target_probes_accepted:r_tgt_acc ///
        requested_route:r_req_route selected_route:r_sel_route       ///
        solver_fallback:r_fallback solver_fallback_error:r_fallback_err ///
        solver_dimension:r_dimension leverage_batch_width:r_lev_batch ///
        target_batch_width:r_tgt_batch rank_tolerance:r_rank_tol      ///
        block_tolerance:r_block_tol full_residual_tolerance:r_full_tol ///
        full_fit_route:r_full_route full_fit_iterations:r_full_iter  ///
        full_fit_reduced_residual:r_full_red                         ///
        full_fit_complete_residual:r_full_complete                   ///
        full_fit_zero_rhs:r_full_zero leverage_rhs_count:r_lev_rhs  ///
        target_rhs_count:r_tgt_rhs max_reduced_residual:r_max_red   ///
        max_complete_residual:r_max_complete max_leverage:r_max_lev ///
        max_reciprocal_residual:r_max_recip accounting_residual:r_accounting ///
        topology_checksum_hi:r_top_hi topology_checksum_lo:r_top_lo ///
        rng_contract_code:r_rng rhs_receipt_rows:r_rhs_rows          ///
        caller_result_copy_bytes:r_rhs_copy                         ///
        requested_algorithm_code:r_algorithm_req                    ///
        selected_algorithm_code:r_algorithm_sel                     ///
        requested_engine_code:r_engine_req selected_engine_code:r_engine_sel ///
        deletion_mode_code:r_deletion nuisance_mode_code:r_nuisance ///
        parameters:r_parameters full_parameters:r_full_parameters   ///
        correction_parameters:r_correction_parameters              ///
        information_rcond:r_info_rcond inverse_relative_residual:r_inverse_relres ///
        exact_peak_forecast_bytes:r_exact_peak exact_diagnostic_flags:r_exact_flags ///
        working_fit_complete_residual:r_working_fit                 ///
        inverse_sqrt_relative_residual:r_inverse_sqrt               ///
        maker_relative_residual:r_maker                             ///
        control_basis_relative_residual:r_control_relres            ///
        control_basis_forward_error:r_control_forward               ///
        deletion_rank_gap:r_rank_gap firm_zero_sum_residual:r_firm_zero ///
        fit_peak_forecast_bytes:r_fit_peak                          ///
        correction_peak_forecast_bytes:r_correction_peak            ///
        actual_accounting_residual:r_actual_accounting weighted_rss:r_rss ///
        memory_limit_bytes:r_mem_limit caller_copy_bytes:r_input_copy ///
        preparation_peak_forecast_bytes:r_prep_peak                 ///
        prepared_resident_bytes:r_resident                          ///
        solver_setup_forecast_bytes:r_solver_setup                  ///
        leverage_phase_forecast_bytes:r_lev_phase                   ///
        target_phase_forecast_bytes:r_tgt_phase                     ///
        result_forecast_bytes:r_result_bytes                        ///
        solve_peak_forecast_bytes:r_solve_peak                      ///
        command_peak_forecast_bytes:r_command_peak                  ///
        capability_schema:r_cap_schema capability_profile:r_cap_profile ///
        batch_mode_code:r_batch_mode stayers_mode_code:r_stayers_mode ///
        target_weight_mode_code:r_target_mode deletion_source_code:r_deletion_source ///
        probeorder_supplied:r_probeorder wallseconds_supplied:r_wallseconds ///
        frequency_use_code:r_frequency physical_limit:r_physical_limit ///
        request_signature_hi:r_signature_hi request_signature_lo:r_signature_lo ///
        generic_controls_count:r_result_controls                    ///
        batch_lev_mode:r_lev_batch_mode batch_tgt_mode:r_tgt_batch_mode ///
        plan_struct:r_plan_struct plan_schema:r_plan_schema          ///
        plan_route_schema:r_plan_route_schema plan_resolved:r_plan_resolved ///
        plan_frozen:r_plan_frozen plan_applicability:r_plan_applicability ///
        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel      ///
        plan_route_req:r_plan_route_req plan_route_sel:r_plan_route_sel ///
        plan_route_fallback:r_plan_route_fallback                    ///
        plan_route_error:r_plan_route_error plan_rhs:r_plan_rhs      ///
        plan_full_dim:r_plan_full_dim batch_lev_sel:r_plan_lev_batch ///
        batch_tgt_sel:r_plan_tgt_batch mem_command:r_plan_mem_command ///
        ctr_complete:r_ctr_complete plan_res_rng_hi:r_pre_rng_hi     ///
        plan_res_rng_lo:r_pre_rng_lo wall_req_app:r_wall_req_app     ///
        wall_requested:r_wall_requested wall_forecast:r_wall_forecast ///
        wall_advisory:r_wall_advisory wall_margin:r_wall_margin {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }

    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)
    local context_ok = `p_input'>0 & `p_retained'>0 & `p_workers'>0 & ///
        `p_firms'>1 & `p_cells'>0 & `p_units'>0 & `p_strata'>0 &    ///
        `p_controls'>=0 & `p_input'==`ncomplete' &                  ///
        `p_retained'==`retained_count' &                            ///
        `p_input_copy'==`expected_input_copy' &                      ///
        `p_prep_peak'==`expected_prep_peak' & `p_resident'>0 &      ///
        `p_target'>0 & `retained_target'>0 &                        ///
        abs(`p_target'-`retained_target')<=1e-10*max(1,abs(`p_target')) & ///
        `g_input_rows'==`ncomplete' & `g_keep_rows'==`retained_count' & ///
        `g_keep_mass'==`retained_physical' & `g_input_mass'>=`g_keep_mass' & ///
        `g_init_comp'>0 & `g_max_comp'>=`g_init_comp' &             ///
        `g_init_rows'>=`g_mover_rows' & `g_mover_rows'>=`g_keep_rows' & ///
        `g_init_edges'>=`g_keep_edges' &                            ///
        `g_degree_iters'<=`g_degree_removed' &                      ///
        `g_art_iters'<=`g_art_removed' & `g_bridge_iters'<=`g_bridge_units' & ///
        `g_fixed_iters'==`g_degree_iters'+`g_art_iters'+`g_bridge_iters' & ///
        `capstruct'==160 & `capabi'==1 & `capschema'==3 &            ///
        `capsupported'==1 & `capreason'==0 & `capprofile'==4 &       ///
        `capalgorithm'==`algreq' & `capdeletion'==`delcode' &       ///
        `capnuisance'==`nuiscode' & `caproute'==0 & `caprng'==1 &   ///
        `capcontrols'==`p_controls' & `capfrequency'==`r_frequency' & ///
        `capengine'==`engreq' & `capbatch'==`expected_batch_mode' &  ///
        `capstayers'==1 & `captarget'==`r_target_mode' &             ///
        `capdelsource'==`r_deletion_source' & `capprobeorder'==0 &   ///
        `capwallsup'==`wallsup' & `capphysical'==`physicallimit' &   ///
        `capsignaturehi'==`r_signature_hi' &                         ///
        `capsignaturelo'==`r_signature_lo' &                         ///
        `caplevmode'==`expected_batch_mode' &                        ///
        `captgtmode'==`expected_batch_mode' & `capfallback'==1 &     ///
        `capalgdefer'==1 & `capengdefer'==1 & `caproutedefer'==1 &  ///
        `caplevdefer'==("`batchrequested'"=="auto") &               ///
        `captgtdefer'==("`batchrequested'"=="auto") &               ///
        `capwalladvisory'==1 & `capwallseconds'==`wallvalue' &       ///
        `r_cap_schema'==3 & `r_cap_profile'==4 &                    ///
        `r_algorithm_req'==0 & `r_algorithm_sel'==1 &               ///
        `r_engine_req'==`engreq' & `r_engine_sel'==3 &              ///
        `r_result_controls'==`p_controls' & `r_rhs_rows'==0 &       ///
        `r_rhs_copy'==0 & `r_solve_peak'==`r_plan_mem_command' &    ///
        `r_plan_applicability'==1 & `r_plan_resolved'==1 &          ///
        `r_plan_frozen'==1 & `r_ctr_complete'==1 &                  ///
        `r_pre_rng_hi'==0 & `r_pre_rng_lo'==0 &                     ///
        `nscope'>=`ncomplete' & `ncomplete'>=`retained_count' &     ///
        `nstayers'>=0 & `nstayerrows'>=0
    if `context_ok' & `delcode'==1 {
        local context_ok = `g_keep_edges'==`p_units' &               ///
            (`g_bridge_iters'==0)==(`g_bridge_units'==0) &           ///
            (`g_bridge_iters'==0)==(`g_bridge_rows'==0)
    }
    if `context_ok' & `delcode'==2 {
        local context_ok = `p_units'==`retained_physical' &          ///
            `g_bridge_units'==0 & `g_bridge_rows'==0 & `g_bridge_iters'==0
    }
    if !`context_ok' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact public context did not reconcile with the validated V7 result."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_context_reconcile"
        exit 498
    }

    capture quietly _vckss_rust_public_call release `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(release) norelease
        exit _rc
    }
    capture quietly varcomp_kss_rust snapshot
    if _rc | r(state)!=0 | r(handle)!=0 {
        capture quietly varcomp_kss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact release did not return the native engine to idle state."
        exit 498
    }

'''
helper_path = Path("varcomp_kss/_vckss_rust_post_exact_v7.ado")
if helper_path.exists():
    raise SystemExit(f"refusing to overwrite existing helper {helper_path}")
helper_path.write_text(preamble + post_block + "\nend\n", encoding="utf-8")

manifest_path = Path("varcomp_kss/varcomp_kss.pkg")
replace_once(
    manifest_path,
    "f _vckss_rust_post_comp_v7.ado\n",
    "f _vckss_rust_post_comp_v7.ado\nf _vckss_rust_post_exact_v7.ado\n",
    "package exact poster",
)

layout_path = Path("varcomp_kss/tests/python/test_package_layout.py")
replace_once(
    layout_path,
    '        "_vckss_rust_plan_receipt.ado",\n'
    '        "_vckss_rust_macos.ado",\n',
    '        "_vckss_rust_plan_receipt.ado",\n'
    '        "_vckss_rust_reconcile_comp_v7.ado",\n'
    '        "_vckss_rust_reconcile_exact_v7.ado",\n'
    '        "_vckss_rust_post_comp_v7.ado",\n'
    '        "_vckss_rust_post_exact_v7.ado",\n'
    '        "_vckss_rust_macos.ado",\n',
    "package layout V7 helpers",
)

install_path = Path("varcomp_kss/tests/stata/test_rust_public_install.do")
replace_once(
    install_path,
    "    _vckss_rust_reconcile_comp_v7.ado                    ///\n"
    "    _vckss_rust_post_comp_v7.ado _vckss_rust_macos.ado   ///\n",
    "    _vckss_rust_reconcile_comp_v7.ado                    ///\n"
    "    _vckss_rust_reconcile_exact_v7.ado                   ///\n"
    "    _vckss_rust_post_comp_v7.ado                         ///\n"
    "    _vckss_rust_post_exact_v7.ado _vckss_rust_macos.ado  ///\n",
    "clean-install exact helpers",
)

qualifier_path = Path("rust/stata_backend/qualify_macos.sh")
replace_once(
    qualifier_path,
    '  "${package_dir}/_vckss_rust_reconcile_comp_v7.ado"\n'
    '  "${package_dir}/_vckss_rust_post_comp_v7.ado"\n',
    '  "${package_dir}/_vckss_rust_reconcile_comp_v7.ado"\n'
    '  "${package_dir}/_vckss_rust_reconcile_exact_v7.ado"\n'
    '  "${package_dir}/_vckss_rust_post_comp_v7.ado"\n'
    '  "${package_dir}/_vckss_rust_post_exact_v7.ado"\n',
    "qualifier exact helpers",
)

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
test = test_path.read_text(encoding="utf-8")
probe_anchor = "quietly varcomp_kss_rust clear\nquietly varcomp_kss_rust requestcapability, algorithm(auto) deletion(match) ///\n"
probe_replacement = r'''quietly varcomp_kss_rust clear
quietly varcomp_kss_rust probe
local xcore = r(core_ready_flags)
local xsupport = r(support_flags)
quietly varcomp_kss_rust requestcapability, algorithm(auto) deletion(match) ///
'''
if test.count(probe_anchor) != 1:
    raise SystemExit("auto-exact probe anchor changed")
test = test.replace(probe_anchor, probe_replacement, 1)

cap_anchor = """assert r(target_batch_resolution_deferred) == 1
local xsighi = r(request_signature_hi)
local xsiglo = r(request_signature_lo)
"""
cap_replacement = r'''assert r(target_batch_resolution_deferred) == 1
tempname xcapctx
matrix `xcapctx' = (r(struct_size),r(abi_version),r(request_schema), ///
    r(supported),r(reason_code),r(profile_code),r(algorithm_code), ///
    r(deletion_mode_code),r(nuisance_mode_code),r(solver_route_code), ///
    r(rng_contract_code),r(controls_count),r(frequency_use_code),  ///
    r(engine_code),r(batch_mode_code),r(stayers_mode_code),        ///
    r(target_weight_mode_code),r(deletion_source_code),            ///
    r(probeorder_supplied),r(wallseconds_supplied),r(physical_limit), ///
    r(request_signature_hi),r(request_signature_lo),               ///
    r(leverage_batch_mode_code),r(target_batch_mode_code),         ///
    r(automatic_fallback_allowed),r(algorithm_resolution_deferred), ///
    r(engine_resolution_deferred),r(route_resolution_deferred),    ///
    r(leverage_batch_deferred),r(target_batch_resolution_deferred), ///
    r(wall_advisory_only),r(wallseconds))
local xsighi = r(request_signature_hi)
local xsiglo = r(request_signature_lo)
'''
if test.count(cap_anchor) != 1:
    raise SystemExit("auto-exact capability context anchor changed")
test = test.replace(cap_anchor, cap_replacement, 1)

prep_anchor = """local xresident = r(prepared_resident_bytes)
assert `xworkers'+`xfirms'-1 == 15
"""
prep_replacement = r'''local xresident = r(prepared_resident_bytes)
tempname xprepctx xgraphctx
matrix `xprepctx' = (r(input_rows),r(retained_rows),r(workers),r(firms), ///
    r(cells),r(deletion_units),r(target_strata),r(target_weight_sum), ///
    r(controls_count),r(memory_limit_bytes),r(caller_copy_bytes),    ///
    r(preparation_peak_forecast_bytes),r(prepared_resident_bytes))
matrix `xgraphctx' = (r(graph_input_rows),r(graph_retained_rows),     ///
    r(graph_input_physical_mass),r(graph_retained_physical_mass),    ///
    r(graph_initial_components),r(graph_maximum_components),         ///
    r(graph_initial_component_rows),r(graph_mover_input_rows),       ///
    r(graph_initial_deletion_edges),r(graph_retained_deletion_edges), ///
    r(graph_degree_workers_removed),r(graph_artic_workers_removed),  ///
    r(graph_bridge_units_removed),r(graph_bridge_rows_removed),      ///
    r(graph_degree_iterations),r(graph_articulation_iterations),     ///
    r(graph_bridge_iterations),r(graph_fixed_point_iterations))
assert `xworkers'+`xfirms'-1 == 15
'''
if test.count(prep_anchor) != 1:
    raise SystemExit("auto-exact preparation context anchor changed")
test = test.replace(prep_anchor, prep_replacement, 1)

release_anchor = r'''quietly varcomp_kss_rust release `xhandle'
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`exact_rng'"'
'''
poster_block = r'''quietly _vckss_rust_post_exact_v7 `xhandle' outcome frequency ///
    target_weight `xkeep' 96 96 0 0 7 8 81227 1e-12 10000 1 auto auto ///
    1 1 counter_v1 1 1 1 1 1 0 `xcore' `xsupport' "nodisplay" match ///
    joint 1e-10 1e-10 50000000 auto auto 1                         ///
    "varcomp_kss outcome [fw=frequency], backend(rust) algorithm(auto) engine(auto)" ///
    0 0 `xprepctx' `xgraphctx' `xcapctx'
assert `"`e(algorithm_requested)'"' == "auto"
assert `"`e(algorithm)'"' == "exact"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "NOT_APPLICABLE"
assert `"`e(result_family)'"' == "exact"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 1
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 3
assert e(rust_plan_applicability) == 1
assert e(rust_plan_resolved) == 1 & e(rust_plan_frozen) == 1
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 1
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 3
assert e(rust_plan_route_requested) == 4
assert e(rust_plan_route_selected) == 4
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(probes) == 0 & e(numerical_mcse_available) == 0
assert mreldif(e(results),`xresult') == 0
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`exact_rng'"'
'''
if test.count(release_anchor) != 1:
    raise SystemExit("auto-exact release anchor changed")
test = test.replace(release_anchor, poster_block, 1)
test_path.write_text(test, encoding="utf-8")
