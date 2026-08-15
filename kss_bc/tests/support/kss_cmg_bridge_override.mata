*! Test-only replacement for kssbc__stata_jla(); load only after mata drop

version 18.0

mata:
mata set matastrict on
mata set matalnum on

void kssbc__stata_jla(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar sample_name,
    string scalar deletion,
    string scalar nuisance,
    real scalar probes,
    real scalar batch,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit,
    string scalar results_name,
    string scalar status_local,
    string scalar message_local,
    string scalar diagnostics_name,
    string scalar solver_diagnostics_name)
{
    struct kssbc_cmg_estimator_result scalar routed
    struct kssbc_result scalar out
    real scalar memory_gib
    real colvector y, worker, firm, frequency, target, deletion_id
    real matrix controls, results, diagnostics

    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    memory_gib = strtoreal(st_global("KSSBC_CMG_MEMORY_GIB"))
    if (missing(memory_gib)) memory_gib = 4
    routed = kssbc__jla_cmg(
        y,worker,firm,controls,frequency,target,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit,
        memory_gib*1024^3)
    out = routed.estimator
    results = out.plugin \ out.correction \ out.corrected \
        out.numerical_mcse
    diagnostics = (out.n_stored,out.n_physical,out.worker_levels,
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
        out.solver_precond_batches)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_matrix(solver_diagnostics_name,out.solver_rhs_diagnostics)
    st_matrix("KSSBC_CMG_ROUTE_DIAGNOSTICS",routed.diagnostics)
    st_global("KSSBC_CMG_STATUS",routed.cmg_status)
    st_global("KSSBC_CMG_MESSAGE",routed.cmg_message)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

end
