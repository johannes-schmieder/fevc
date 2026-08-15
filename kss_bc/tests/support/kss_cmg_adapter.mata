*! Forced test-only KSS/CMG adapter; not part of the installed package

version 18.0

mata:
mata set matastrict on
mata set matalnum on

struct kssbc_cmg_estimator_result
{
    struct kssbc_result scalar estimator
    string scalar cmg_status
    string scalar cmg_message
    real rowvector diagnostics
}

struct kssbc_cmg__hierarchy scalar kssbc__cmg_hierarchy(
    struct kssbc_fe_design scalar design,
    real colvector worker_key,
    real colvector firm_key,
    real scalar memory_envelope_bytes,
    real scalar planned_rhs)
{
    struct kssbc_cmg__hierarchy scalar out
    struct kssbc_cmg__cells scalar cells
    struct kssbc_cmg__graph scalar graph
    struct kssbc_cmg__options scalar options
    struct kssbc_cmg__preflight_result scalar preflight

    out = kssbc_cmg__empty_hierarchy()
    if (design.status != "CONVERGED") {
        out.message = "KSS design is not prepared"
        return(out)
    }
    cells = kssbc_cmg__cells_prepare(
        design.worker,design.firm,design.frequency,worker_key,firm_key)
    if (cells.status != "CONVERGED") {
        out.status = cells.status
        out.message = cells.message
        return(out)
    }
    options = kssbc_cmg__options_resource(
        memory_envelope_bytes,design.firm_levels,planned_rhs)
    preflight = kssbc_cmg__preflight(
        cells,planned_rhs,1,memory_envelope_bytes,options)
    if (preflight.status != "CONVERGED") {
        out.status = preflight.status
        out.message = preflight.message
        return(out)
    }
    graph = kssbc_cmg__hybrid_build(cells)
    if (graph.status != "CONVERGED") {
        out.status = graph.status
        out.message = graph.message
        return(out)
    }
    options = kssbc_cmg__options_resource(
        memory_envelope_bytes,graph.n_vertex,planned_rhs)
    return(kssbc_cmg__hierarchy_build(graph,options))
}

struct kssbc_preconditioner_result scalar kssbc__cmg_apply(
    pointer scalar context,
    struct kssbc_fe_design scalar design,
    real matrix residual)
{
    struct kssbc_preconditioner_result scalar out
    struct kssbc_cmg__apply_result scalar applied
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy

    out.status = "INVALID_INPUT"
    out.message = "invalid forced-CMG preconditioner input"
    out.value = J(0,0,.)
    hierarchy = context
    if (context == NULL | design.status != "CONVERGED" |
        rows(residual) != design.firm_levels | cols(residual) < 1 |
        hasmissing(residual)) return(out)
    applied = kssbc_cmg__apply_kss(*hierarchy,residual)
    out.status = applied.status
    out.message = applied.message
    out.value = applied.value
    return(out)
}

struct kssbc_solver_backend scalar kssbc__cmg_backend(
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy)
{
    struct kssbc_solver_backend scalar out

    out.route = "CMG"
    out.context = hierarchy
    out.apply = &kssbc__cmg_apply()
    return(out)
}

struct kssbc_solve_result scalar kssbc__fe_solve_matrix_cmg(
    struct kssbc_fe_design scalar design,
    struct kssbc_cmg__hierarchy scalar hierarchy,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct kssbc_solver_backend scalar backend

    backend = kssbc__cmg_backend(&hierarchy)
    return(kssbc__fe_solve_matrix_backend(
        design,right_hand_side,tolerance,maxiter,backend))
}

real colvector kssbc__canonical_vertex_keys(
    real colvector identifier,
    real scalar levels)
{
    real scalar level, begin, finish
    real colvector row, sorted, key
    real matrix panel

    if (rows(identifier) == 0 | cols(identifier) != 1 |
        levels < 1 | levels != floor(levels) | hasmissing(identifier) |
        min(identifier) != 1 | max(identifier) != levels) return(J(0,1,.))
    row = 1::rows(identifier)
    sorted = order(identifier,1)
    panel = panelsetup(identifier[sorted],1)
    if (rows(panel) != levels) return(J(0,1,.))
    key = J(levels,1,.)
    for (level=1; level<=levels; level++) {
        begin = panel[level,1]
        finish = panel[level,2]
        key[level] = min(row[sorted[|begin\finish|]])
    }
    if (hasmissing(key) | rows(uniqrows(sort(key,1))) != levels) {
        return(J(0,1,.))
    }
    return(key)
}

real scalar kssbc__planned_rhs(
    real scalar probes,
    real scalar controls,
    string scalar nuisance)
{
    if (missing(probes) | probes < 2 | probes != floor(probes) |
        missing(controls) | controls < 0 | controls != floor(controls) |
        !(nuisance == "joint" | nuisance == "fixedoffset")) return(.)
    return(3*probes+controls+1+(nuisance == "fixedoffset" & controls > 0))
}

struct kssbc_cmg_estimator_result scalar kssbc__jla_cmg(
    real colvector y,
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector target_weight,
    real colvector deletion_id,
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
    real scalar memory_envelope_bytes)
{
    struct kssbc_cmg_estimator_result scalar out
    struct kssbc_fe_design scalar base
    struct kssbc_cmg__hierarchy scalar hierarchy
    struct kssbc_cmg__level scalar fine_level
    struct kssbc_solver_backend scalar backend
    real scalar planned_rhs, setup_seconds, hybrid_vertices, hybrid_edges
    real colvector worker_key, firm_key

    out.estimator = kssbc__failure(
        "INVALID_SOLVER_BACKEND","forced-CMG estimator setup is invalid")
    out.cmg_status = "INVALID_INPUT"
    out.cmg_message = "forced-CMG estimator setup is invalid"
    out.diagnostics = J(1,12,.)
    planned_rhs = kssbc__planned_rhs(probes,cols(controls),nuisance)
    if (missing(planned_rhs) | missing(memory_envelope_bytes) |
        memory_envelope_bytes < 1024^3 | memory_envelope_bytes > 56*1024^3) {
        return(out)
    }

    timer_clear(70)
    timer_on(70)
    base = kssbc__fe_prepare(worker,firm,frequency,rank_tolerance)
    if (base.status != "CONVERGED") {
        timer_off(70)
        out.estimator = kssbc__failure(base.status,base.message)
        out.cmg_status = base.status
        out.cmg_message = base.message
        return(out)
    }
    worker_key = kssbc__canonical_vertex_keys(
        base.worker,base.worker_levels)
    firm_key = kssbc__canonical_vertex_keys(base.firm,base.firm_levels)
    if (rows(worker_key) != base.worker_levels |
        rows(firm_key) != base.firm_levels) {
        timer_off(70)
        out.estimator = kssbc__failure(
            "CANONICAL_KEYS_UNAVAILABLE",
            "ID-free canonical CMG vertex keys are unavailable")
        out.cmg_status = out.estimator.status
        out.cmg_message = out.estimator.message
        return(out)
    }
    hierarchy = kssbc__cmg_hierarchy(
        base,worker_key,firm_key,memory_envelope_bytes,planned_rhs)
    timer_off(70)
    setup_seconds = kssbc__timer_seconds(70)
    out.cmg_status = hierarchy.status
    out.cmg_message = hierarchy.message
    hybrid_vertices = .
    hybrid_edges = .
    if (hierarchy.n_level >= 1) {
        fine_level = *hierarchy.level[1]
        hybrid_vertices = fine_level.graph.n_vertex
        hybrid_edges = fine_level.graph.n_edge
    }
    out.diagnostics = (planned_rhs,memory_envelope_bytes,setup_seconds,
        hierarchy.n_level,hierarchy.edge_complexity,
        hierarchy.vertex_complexity,hierarchy.structural_bytes,
        hierarchy.dense_factor_bytes,base.worker_levels,base.firm_levels,
        hybrid_vertices,hybrid_edges)
    if (hierarchy.status != "CONVERGED") {
        out.estimator = kssbc__failure(hierarchy.status,hierarchy.message)
        return(out)
    }
    backend = kssbc__cmg_backend(&hierarchy)
    out.estimator = kssbc__jla_backend(
        y,worker,firm,controls,frequency,target_weight,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit,
        base,backend,setup_seconds)
    return(out)
}

end
