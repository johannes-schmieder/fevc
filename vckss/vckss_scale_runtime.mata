*! vckss compressed-command runtime bridge 0.4.0-dev 18aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum off

/*
The lightweight prepared-state owner and constructor live in vckss_scale.mata
so eligibility diagnostics and numerical work share one canonical compression
pass.  This bridge only consumes that cached state after Stata has cleared the
raw dataset.  Solver, CMG, RNG-cursor, and matrix-RHS scratch remain local to
vckss_scale_runtime__stata_run().
*/

real scalar vckss_scale_runtime__api_level()
{
    return(3)
}

string scalar vckss_scale_runtime__build_id()
{
    return("vckss-scale-runtime-api3-fe-buf1-buffered")
}

real rowvector vckss_srt__diagnostics(
    struct vckss_result scalar out)
{
    return((out.n_stored,out.n_physical,out.worker_levels,
        out.firm_levels,out.parameters,out.deletion_units,
        out.target_weight_sum,out.max_leverage,out.information_rcond,
        out.inverse_relres,out.solver_iterations,
        out.solver_max_residual,out.probes,out.weighted_rss,
        out.fit_seconds,out.leverage_seconds,out.target_seconds,
        out.correction_seconds,out.preconditioner_seconds,
        out.preconditioner_ratio,out.control_schur_rcond,
        out.deletion_rank_gap,out.full_parameters,
        out.correction_parameters,out.schur_seconds,
        out.preconditioner_apply_seconds,out.pcg_seconds,
        out.solver_backend_seconds,out.solver_schur_actions,
        out.solver_schur_batches,out.solver_precond_applications,
        out.solver_precond_batches,out.fe_workspace_applicable,
        out.fe_workspace_builds,out.fe_buffered_schur_batches,
        out.fe_legacy_schur_batches,out.fe_buffered_schur_columns,
        out.fe_legacy_schur_columns,out.fe_packed_fallback_batches,
        out.fe_max_buffer_width,out.fe_workspace_peak_bytes,
        out.fe_cell_bytes_avoided))
}

void vckss_scale_runtime__stata_run(
    real scalar probes,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit,
    string scalar requested_route,
    real scalar memory_envelope_bytes,
    string scalar results_name,
    string scalar diagnostics_name,
    string scalar solver_diagnostics_name,
    string scalar route_diagnostics_name,
    string scalar pilot_diagnostics_name,
    string scalar scale_receipt_name,
    string scalar status_local,
    string scalar message_local,
    string scalar route_local,
    string scalar reason_local,
    string scalar fallback_status_local,
    string scalar fallback_message_local,
    string scalar pilot_status_local,
    string scalar pilot_failure_local,
    string scalar rng_contract_local,
    string scalar rng_implementation_local,
    string scalar rng_runtime_local)
{
    external struct vckss_scale_runtime_state scalar VCKSS_SCALE_RUNTIME
    struct vckss_scale_rng_context scalar rng_context
    struct vckss_scale_atom_provider scalar provider
    struct vckss_scale_route_context scalar route_context
    struct vckss_route_result scalar routed
    struct vckss_result scalar out
    struct vckss_scale_engine_result scalar rich
    struct vckss_fe_design scalar base
    real matrix results

    routed = vckss_solver__empty_route()
    rich = vckss_scale_eng__empty_result()
    if (VCKSS_SCALE_RUNTIME.status != "PREPARED") {
        routed.estimator = vckss__failure(
            "FASTPATH_STATE_UNAVAILABLE",VCKSS_SCALE_RUNTIME.message)
        routed.status = routed.estimator.status
        routed.message = routed.estimator.message
    }
    else {
        rng_context = vckss_scale_eng__rng_context(
            "per_domain_stream_cursor",seed,
            VCKSS_SCALE_RUNTIME.unit_semantic_rank,
            VCKSS_SCALE_RUNTIME.design.unit_frequency,
            VCKSS_SCALE_RUNTIME.stratum_semantic_rank,
            VCKSS_SCALE_RUNTIME.design.strata.physical_count)
        if (rng_context.status != "CONVERGED") {
            routed.estimator = vckss__failure(
                rng_context.status,"compressed RNG context is unavailable")
            routed.status = routed.estimator.status
            routed.message = routed.estimator.message
        }
        else {
            provider = vckss_scale_eng__rng_provider(&rng_context)
            route_context = vckss_scale_eng__route_context(
                &VCKSS_SCALE_RUNTIME.design,provider,
                probes,leverage_batch,target_batch,
                tolerance,maxiter,rank_tolerance,block_tolerance)
            if (route_context.status != "CONVERGED") {
                routed.estimator = vckss__failure(
                    route_context.status,"compressed route context is invalid")
                routed.status = routed.estimator.status
                routed.message = routed.estimator.message
            }
            else {
                base = vckss_scale__fe_view(
                    &VCKSS_SCALE_RUNTIME.design)
                if (base.status != "CONVERGED") {
                    routed.estimator = vckss__failure(
                        base.status,base.message)
                    routed.status = routed.estimator.status
                    routed.message = routed.estimator.message
                }
                else {
                routed = vckss_solver__jla_routed(
                    VCKSS_SCALE_RUNTIME.design.cell_outcome_mean,
                    VCKSS_SCALE_RUNTIME.design.cell_worker,
                    VCKSS_SCALE_RUNTIME.design.cell_firm,J(
                        VCKSS_SCALE_RUNTIME.design.coefficient_cells,0,.),
                    VCKSS_SCALE_RUNTIME.design.cell_frequency,
                    VCKSS_SCALE_RUNTIME.design.cell_target_mass,
                    (1::VCKSS_SCALE_RUNTIME.design.coefficient_cells),
                    "match","joint",
                    probes,max((leverage_batch,target_batch)),seed,
                    tolerance,maxiter,rank_tolerance,block_tolerance,
                    blocksize_limit,requested_route,memory_envelope_bytes,
                    vckss_scale_eng__callback_ptr(),&route_context,
                    J(0,1,.),0,&base)
                rich = route_context.last
                }
            }
        }
    }
    out = routed.estimator
    results = out.plugin \ out.correction \ out.corrected \
        out.numerical_mcse
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,
        vckss_srt__diagnostics(out))
    st_matrix(solver_diagnostics_name,out.solver_rhs_diagnostics)
    st_matrix(route_diagnostics_name,routed.diagnostics)
    st_matrix(pilot_diagnostics_name,routed.pilot_diagnostics)
    st_matrix(scale_receipt_name,(
        rich.coefficient_cells,rich.deletion_units,rich.target_strata,
        rich.max_reciprocal_residual,rich.residual_gate,
        rich.target_identity_residual,rich.max_complete_residual,
        rich.n_stored,rich.n_physical,rich.total_seconds,
        rich.rng_seconds))
    st_local(status_local,out.status)
    st_local(message_local,out.message)
    st_local(route_local,routed.route)
    st_local(reason_local,routed.reason)
    st_local(fallback_status_local,routed.fallback_status)
    st_local(fallback_message_local,routed.fallback_message)
    st_local(pilot_status_local,invtokens(routed.pilot_status,"|"))
    st_local(pilot_failure_local,
        invtokens(routed.pilot_failure_reason,"|"))
    st_local(rng_contract_local,rich.rng_contract)
    st_local(rng_implementation_local,"per_domain_stream_cursor")
    st_local(rng_runtime_local,
        strofreal(st_numscalar("c(stata_version)"),"%9.0g"))
}

end
