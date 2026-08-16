*! kss_bc production solver routing adapter
*! version 0.2.0-dev 15aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

real scalar kssbc_solver__api_level()
{
    return(18)
}

string scalar kssbc_solver__build_id()
{
    return("kss-bc-solver-api18-production-routing")
}

struct kssbc_route_result
{
    struct kssbc_result scalar estimator
    string scalar status
    string scalar message
    string scalar route
    string scalar reason
    string scalar fallback_status
    string scalar fallback_message
    real rowvector diagnostics
}

struct kssbc_route_result scalar kssbc_solver__empty_route()
{
    struct kssbc_route_result scalar out

    out.estimator = kssbc__failure(
        "INVALID_SOLVER_BACKEND","solver routing input is invalid")
    out.status = "INVALID_SOLVER_BACKEND"
    out.message = "solver routing input is invalid"
    out.route = ""
    out.reason = ""
    out.fallback_status = ""
    out.fallback_message = ""
    out.diagnostics = J(1,26,.)
    return(out)
}

real colvector kssbc_solver__canonical_keys(
    real colvector identifier,
    real scalar levels)
{
    real scalar level, begin, finish
    real colvector row, sorted, key
    real matrix panel

    if (rows(identifier) == 0 | cols(identifier) != 1 |
        levels < 1 | levels != floor(levels) | hasmissing(identifier) |
        min(identifier) != 1 | max(identifier) != levels) {
        return(J(0,1,.))
    }
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

real scalar kssbc_solver__planned_rhs(
    real scalar probes,
    real scalar controls,
    string scalar nuisance)
{
    if (missing(probes) | probes < 2 | probes != floor(probes) |
        missing(controls) | controls < 0 | controls != floor(controls) |
        !(nuisance == "joint" | nuisance == "fixedoffset")) return(.)
    return(3*probes+controls+1+(nuisance == "fixedoffset" & controls > 0))
}

real scalar kssbc_solver__cmg_runtime_ok()
{
    return(kssbc_cmg__api_level() == 5 &
        kssbc_cmg__design_label() ==
        "clean-room-cmg-inspired-degree3-hybrid-v5-robust-hierarchy")
}

// The routing scores below are deterministic counts of scalar-equivalent
// work.  The coefficients distinguish row aggregation, V-cycle edge/vertex
// passes, terminal triangular solves, and terminal factor setup.  Measured
// wall times remain diagnostics and never enter a routing decision.
real scalar kssbc_solver__diagonal_work(
    real scalar planned_rhs,
    real scalar iterations,
    real scalar observations,
    real scalar workers,
    real scalar firms)
{
    real scalar schur_work, preconditioner_work

    if (missing((planned_rhs,iterations,observations,workers,firms)) |
        planned_rhs < 1 | iterations < 0 | observations < 1 |
        workers < 1 | firms < 2) return(.)
    schur_work = 8*observations+4*(workers+firms)
    preconditioner_work = 4*firms
    return(planned_rhs*iterations*(schur_work+preconditioner_work))
}

real scalar kssbc_solver__cmg_work(
    real scalar planned_rhs,
    real scalar iterations,
    real scalar observations,
    real scalar workers,
    real scalar firms,
    real scalar hybrid_vertices,
    real scalar hybrid_edges,
    real scalar edge_complexity,
    real scalar vertex_complexity,
    real scalar dense_factor_bytes)
{
    real scalar schur_work, vcycle_work, setup_work, factor_entries
    real scalar terminal_order

    if (missing((planned_rhs,iterations,observations,workers,firms,
                 hybrid_vertices,hybrid_edges,edge_complexity,
                 vertex_complexity,dense_factor_bytes)) |
        planned_rhs < 1 | iterations < 0 | observations < 1 |
        workers < 1 | firms < 2 | hybrid_vertices < 1 |
        hybrid_edges < 0 | edge_complexity <= 0 |
        vertex_complexity <= 0 | dense_factor_bytes < 0) return(.)
    schur_work = 8*observations+4*(workers+firms)
    factor_entries = dense_factor_bytes/8
    terminal_order = max((1,
        floor((sqrt(1+8*factor_entries)-1)/2)))
    vcycle_work = 8*edge_complexity*hybrid_edges+
        12*vertex_complexity*hybrid_vertices+2*factor_entries
    setup_work = 24*(edge_complexity*hybrid_edges+
        vertex_complexity*hybrid_vertices)+
        factor_entries*terminal_order/64
    return(setup_work+
        planned_rhs*iterations*(schur_work+vcycle_work))
}

real scalar kssbc_solver__fallback_work_ok(real scalar work)
{
    // Eight billion row-equivalent scalar actions is the bounded fallback
    // envelope.  It admits genuinely moderate B1 recovery, but prevents a
    // failed large-graph CMG setup from silently launching an implausible
    // repeated-RHS diagonal run.
    return(!missing(work) & work >= 0 & work <= 8e9)
}

real scalar kssbc_solver__cmg_work_wins(real scalar work_ratio)
{
    return(!missing(work_ratio) & work_ratio >= 0 & work_ratio <= 0.80)
}

struct kssbc_solver_cmg_context
{
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy
    struct kssbc_cmg__workspace scalar workspace
    real scalar use_workspace
}

struct kssbc_solver_cmg_context scalar kssbc_solver__cmg_context(
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy,
    real scalar batch_capacity,
    real scalar memory_envelope_bytes,
    real scalar enable_workspace)
{
    struct kssbc_solver_cmg_context scalar out
    real scalar workspace_cap

    out.hierarchy = hierarchy
    out.workspace = kssbc_cmg__empty_workspace()
    out.use_workspace = 0
    if (hierarchy == NULL | (*hierarchy).status != "CONVERGED" |
        missing(batch_capacity) | batch_capacity < 1 |
        batch_capacity != floor(batch_capacity) |
        missing(memory_envelope_bytes) | memory_envelope_bytes <= 0 |
        !(enable_workspace == 0 | enable_workspace == 1)) {
        return(out)
    }
    if (!enable_workspace) return(out)
    workspace_cap = memory_envelope_bytes-
        (*hierarchy).structural_bytes-(*hierarchy).dense_factor_bytes-
        (*hierarchy).options.action_scratch_bytes
    if (missing(workspace_cap) | workspace_cap <= 0) return(out)
    out.workspace = kssbc_cmg__workspace_init(
        *hierarchy,batch_capacity,workspace_cap)
    out.use_workspace = (out.workspace.status == "CONVERGED" &
        out.workspace.predicted_peak_bytes+
            (*hierarchy).dense_factor_bytes <= memory_envelope_bytes)
    return(out)
}

struct kssbc_cmg__apply_result scalar kssbc_solver__cmg_apply_ws(
    struct kssbc_solver_cmg_context scalar context,
    real matrix firm_rhs)
{
    struct kssbc_cmg__apply_result scalar out, core
    struct kssbc_cmg__level scalar fine
    real matrix compatible, injected
    real colvector firm_component

    out = kssbc_cmg__empty_apply_result()
    if (context.hierarchy == NULL |
        (*context.hierarchy).status != "CONVERGED" |
        (*context.hierarchy).n_level < 1 |
        context.workspace.status != "CONVERGED" |
        cols(firm_rhs) < 1 |
        cols(firm_rhs) > context.workspace.batch_capacity |
        hasmissing(firm_rhs)) return(out)
    fine = *(*context.hierarchy).level[1]
    if (rows(firm_rhs) != fine.graph.n_firm) {
        out.status = "INVALID_RHS"
        out.message = "KSS firm RHS has the wrong shape"
        return(out)
    }
    firm_component = fine.component[(1::fine.graph.n_firm)]
    compatible = kssbc_cmg__project_labels(firm_component,firm_rhs)
    if (rows(compatible) != fine.graph.n_firm) return(out)
    injected = J(fine.graph.n_vertex,cols(firm_rhs),0)
    injected[(1::fine.graph.n_firm),.] = compatible
    core = kssbc_cmg__workspace_apply(
        *context.hierarchy,context.workspace,injected)
    if (core.status != "CONVERGED") return(core)
    out = core
    out.value = kssbc_cmg__project_labels(
        firm_component,core.value[(1::fine.graph.n_firm),.])
    if (rows(out.value) != fine.graph.n_firm | hasmissing(out.value)) {
        out.status = "PULLBACK_BREAKDOWN"
        out.message = "KSS quotient pullback failed"
        out.value = J(0,0,.)
    }
    return(out)
}

struct kssbc_preconditioner_result scalar kssbc_solver__cmg_apply(
    pointer scalar context,
    struct kssbc_fe_design scalar design,
    real matrix residual)
{
    struct kssbc_preconditioner_result scalar out
    struct kssbc_cmg__apply_result scalar applied
    pointer(struct kssbc_solver_cmg_context scalar) scalar cmg_context
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy

    out.status = "INVALID_SOLVER_BACKEND"
    out.message = "invalid CMG preconditioner input"
    out.value = J(0,0,.)
    cmg_context = context
    if (context == NULL | design.status != "CONVERGED" |
        rows(residual) != design.firm_levels | cols(residual) < 1 |
        hasmissing(residual)) return(out)
    hierarchy = (*cmg_context).hierarchy
    if (hierarchy == NULL) return(out)
    if ((*cmg_context).use_workspace &
        cols(residual) <= (*cmg_context).workspace.batch_capacity) {
        applied = kssbc_solver__cmg_apply_ws(
            *cmg_context,residual)
    }
    else applied = kssbc_cmg__apply_kss(*hierarchy,residual)
    out.status = applied.status
    out.message = applied.message
    out.value = applied.value
    return(out)
}

struct kssbc_solver_backend scalar kssbc_solver__cmg_backend(
    pointer(struct kssbc_solver_cmg_context scalar) scalar context)
{
    struct kssbc_solver_backend scalar out

    out.route = "CMG"
    out.context = context
    out.apply = &kssbc_solver__cmg_apply()
    out.exact_inverse = ((*context).hierarchy != NULL &
        (*(*context).hierarchy).status == "CONVERGED" &
        (*(*context).hierarchy).n_level == 1)
    return(out)
}

// Internal hierarchy_from_cells path.  Its caller owns cells preparation and
// preflight validation, so graph construction never rescans the retained
// observation arrays.
struct kssbc_cmg__hierarchy scalar kssbc_solver__hierarchy_cells(
    struct kssbc_cmg__cells scalar cells,
    real scalar memory_envelope_bytes,
    real scalar planned_rhs)
{
    struct kssbc_cmg__hierarchy scalar out
    struct kssbc_cmg__graph scalar graph
    struct kssbc_cmg__options scalar options

    out = kssbc_cmg__empty_hierarchy()
    if (cells.status != "CONVERGED" |
        missing(memory_envelope_bytes) | memory_envelope_bytes <= 0 |
        missing(planned_rhs) | planned_rhs < 1 |
        planned_rhs != floor(planned_rhs)) {
        out.message = "validated CMG cells and resources are required"
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

struct kssbc_cmg__hierarchy scalar kssbc_solver__hierarchy(
    struct kssbc_fe_design scalar design,
    real colvector worker_key,
    real colvector firm_key,
    real scalar memory_envelope_bytes,
    real scalar planned_rhs)
{
    struct kssbc_cmg__hierarchy scalar out
    struct kssbc_cmg__cells scalar cells
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
    return(kssbc_solver__hierarchy_cells(
        cells,memory_envelope_bytes,planned_rhs))
}

real matrix kssbc_solver__pilot_rhs(
    struct kssbc_fe_design scalar design,
    real colvector firm_key)
{
    real matrix firm_rhs
    real colvector component

    // Pilot RHSs live on firm coordinates, not hybrid coordinates.  The
    // retained worker-firm graph is connected after production pruning, so
    // the firm quotient has one component.
    component = J(design.firm_levels,1,1)
    firm_rhs = kssbc_cmg__pilot_rhs(firm_key,component)
    if (rows(firm_rhs) != design.firm_levels) return(J(0,0,.))
    return(J(design.worker_levels,4,0) \
        firm_rhs[1..(design.firm_levels-1),.])
}

real scalar kssbc_solver__pilots_valid(
    struct kssbc_solve_result scalar solved,
    real scalar cap,
    real scalar tolerance)
{
    real scalar residual_gate

    residual_gate = max((1e-11,10*tolerance))
    if (solved.status != "CONVERGED" | cols(solved.rhs_status) != 4 |
        cols(solved.rhs_iterations) != 4 |
        cols(solved.rhs_relres) != 4 | hasmissing(solved.rhs_iterations) |
        hasmissing(solved.rhs_relres) | max(solved.rhs_iterations) > cap |
        max(solved.rhs_relres) > residual_gate |
        min(solved.rhs_status :== "CONVERGED") != 1) {
        return(0)
    }
    return(1)
}

struct kssbc_route_result scalar kssbc_solver__jla_routed(
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
    string scalar requested_route,
    real scalar memory_envelope_bytes)
{
    struct kssbc_route_result scalar out
    struct kssbc_fe_design scalar base
    struct kssbc_cmg__cells scalar cells
    struct kssbc_cmg__options scalar options
    struct kssbc_cmg__preflight_result scalar preflight
    struct kssbc_cmg__hierarchy scalar hierarchy
    struct kssbc_cmg__level scalar fine
    struct kssbc_solver_cmg_context scalar cmg_context
    struct kssbc_solver_backend scalar diagonal_backend, cmg_backend, backend
    struct kssbc_solve_result scalar diagonal_pilots, cmg_pilots
    real colvector worker_key, firm_key
    real matrix pilot_rhs
    real scalar planned_rhs, setup_seconds, hierarchy_seconds
    real scalar diagonal_seconds, cmg_seconds, pilot_cap
    real scalar diagonal_work, cmg_work, work_ratio
    real scalar hybrid_vertices, hybrid_edges, forecast_peak, route_code
    real scalar use_cmg, diagonal_valid, diagonal_realistic, cmg_valid
    real scalar diagonal_max_iterations, cmg_max_iterations
    real scalar workspace_capacity, solver_memory_bytes
    real scalar hierarchy_memory_bytes
    real scalar base_bytes, hierarchy_peak
    real scalar terminal_vertices

    out = kssbc_solver__empty_route()
    requested_route = strupper(strtrim(requested_route))
    planned_rhs = kssbc_solver__planned_rhs(
        probes,cols(controls),nuisance)
    if (missing(planned_rhs) |
        !(requested_route == "AUTO" | requested_route == "DIAGONAL" |
          requested_route == "CMG") |
        missing(memory_envelope_bytes) |
        memory_envelope_bytes < 1024^3 |
        memory_envelope_bytes > 56*1024^3) return(out)
    if (requested_route != "DIAGONAL" &
        !kssbc_solver__cmg_runtime_ok()) {
        out.estimator = kssbc__failure(
            "CMG_VERSION_MISMATCH",
            "CMG API 5 robust-hierarchy runtime is required")
        out.status = out.estimator.status
        out.message = out.estimator.message
        return(out)
    }
    // Probe batching reserves the other 35% in the ado layer.  Limiting the
    // solver here makes the two independently enforced forecasts additive.
    solver_memory_bytes = 0.65*memory_envelope_bytes

    timer_clear(88)
    timer_clear(89)
    timer_clear(90)
    timer_on(88)
    base = kssbc__fe_prepare(worker,firm,frequency,rank_tolerance)
    timer_off(88)
    if (base.status != "CONVERGED") {
        out.estimator = kssbc__failure(base.status,base.message)
        out.status = base.status
        out.message = base.message
        return(out)
    }
    setup_seconds = kssbc__timer_seconds(88)
    base_bytes = 8*(8*base.n+
        5*(base.worker_levels+base.firm_levels))
    hierarchy_memory_bytes = solver_memory_bytes-base_bytes
    hierarchy = kssbc_cmg__empty_hierarchy()
    diagonal_backend = kssbc__diagonal_backend()
    backend = diagonal_backend
    // Route diagnostic slot 22 is cumulative CMG structural-preparation
    // time: the one cells/preflight pass, graph/hierarchy construction if
    // attempted, and bounded backend-context initialization.  It excludes
    // deterministic B1 and CMG pilot solves.
    hierarchy_seconds = 0
    preflight = kssbc_cmg__empty_preflight()
    preflight.planned_rhs = planned_rhs
    if (max((planned_rhs,8)) < 32) preflight.pilot_cap = 128
    else if (max((planned_rhs,8)) < 128) preflight.pilot_cap = 64
    else preflight.pilot_cap = 32
    preflight.predicted_vertices = 0
    preflight.predicted_edges = 0
    preflight.predicted_structural_bytes = 0
    preflight.predicted_scratch_bytes = 0
    if (requested_route == "DIAGONAL") {
        // Explicit B1 does not depend on canonical CMG keys, construction
        // scratch, hierarchy feasibility, or any CMG preflight status.
        preflight.status = "NOT_RUN"
        preflight.route = "DIAGONAL"
        preflight.message = "CMG preflight bypassed for explicit diagonal B1"
    }
    else {
        if (missing(hierarchy_memory_bytes) |
            hierarchy_memory_bytes <= 0) {
            out.estimator = kssbc__failure(
                "SOLVER_MEMORY_LIMIT",
                "persistent FE design exhausts the solver memory reservation")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        worker_key = kssbc_solver__canonical_keys(
            base.worker,base.worker_levels)
        firm_key = kssbc_solver__canonical_keys(
            base.firm,base.firm_levels)
        if (rows(worker_key) != base.worker_levels |
            rows(firm_key) != base.firm_levels) {
            out.estimator = kssbc__failure(
                "CANONICAL_KEYS_UNAVAILABLE",
                "ID-free canonical CMG vertex keys are unavailable")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        timer_on(90)
        cells = kssbc_cmg__cells_prepare(
            base.worker,base.firm,base.frequency,worker_key,firm_key)
        if (cells.status != "CONVERGED") {
            timer_off(90)
            out.estimator = kssbc__failure(cells.status,cells.message)
            out.status = cells.status
            out.message = cells.message
            return(out)
        }
        options = kssbc_cmg__options_resource(
            hierarchy_memory_bytes,base.firm_levels,planned_rhs)
        preflight = kssbc_cmg__preflight(
            cells,planned_rhs,1,hierarchy_memory_bytes,options)
        if (preflight.status != "CONVERGED") {
            timer_off(90)
            // Without bounded B1 pilots there is no evidence that fallback is
            // realistic.  AUTO therefore fails closed before RNG rather than
            // treating a CMG preflight error as permission to launch B1.
            out.estimator = kssbc__failure(
                preflight.status,preflight.message)
            out.status = preflight.status
            out.message = preflight.message
            return(out)
        }
        if (preflight.predicted_structural_bytes+
            preflight.predicted_scratch_bytes > hierarchy_memory_bytes) {
            timer_off(90)
            out.estimator = kssbc__failure(
                "SOLVER_MEMORY_LIMIT",
                "CMG preflight plus persistent FE design exceeds the solver memory reservation")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        timer_off(90)
        hierarchy_seconds = kssbc__timer_seconds(90)
    }
    pilot_cap = preflight.pilot_cap
    hybrid_vertices = preflight.predicted_vertices
    hybrid_edges = preflight.predicted_edges
    diagonal_seconds = 0
    cmg_seconds = 0
    diagonal_work = .
    cmg_work = .
    work_ratio = .
    diagonal_max_iterations = .
    cmg_max_iterations = .
    route_code = 1
    terminal_vertices = .

    if (requested_route == "DIAGONAL") {
        out.route = "DIAGONAL"
        out.reason = "diagonal B1 was explicitly requested"
    }
    else if (requested_route == "AUTO" & preflight.route == "DIAGONAL") {
        out.route = "DIAGONAL"
        out.reason = preflight.message
    }
    else {
        pilot_rhs = kssbc_solver__pilot_rhs(base,firm_key)
        if (rows(pilot_rhs) == 0) {
            out.estimator = kssbc__failure(
                "ROUTING_PILOT_FAILED",
                "deterministic routing right-hand sides are unavailable")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        timer_on(89)
        diagonal_pilots = kssbc__fe_solve_matrix_backend(
            base,pilot_rhs,tolerance,maxiter,diagonal_backend)
        timer_off(89)
        diagonal_seconds = kssbc__timer_seconds(89)
        diagonal_valid = kssbc_solver__pilots_valid(
            diagonal_pilots,pilot_cap,tolerance)
        if (diagonal_pilots.status == "CONVERGED") {
            diagonal_max_iterations = max(diagonal_pilots.rhs_iterations)
            diagonal_work = kssbc_solver__diagonal_work(
                planned_rhs,diagonal_max_iterations,base.n,
                base.worker_levels,base.firm_levels)
        }
        diagonal_realistic = diagonal_valid &
            kssbc_solver__fallback_work_ok(diagonal_work)
        if (requested_route == "AUTO" & diagonal_realistic &
            max(diagonal_pilots.rhs_iterations) <= 4) {
            out.route = "DIAGONAL"
            out.reason =
                "four deterministic pilots converged in at most four B1 steps"
        }
        else {
            timer_on(90)
            hierarchy = kssbc_solver__hierarchy_cells(
                cells,hierarchy_memory_bytes,planned_rhs)
            timer_off(90)
            hierarchy_seconds = kssbc__timer_seconds(90)
            if (hierarchy.n_level >= 1) {
                fine = *hierarchy.level[1]
                hybrid_vertices = fine.graph.n_vertex
                hybrid_edges = fine.graph.n_edge
            }
            if (hierarchy.status == "CONVERGED") {
                terminal_vertices =
                    (*hierarchy.level[hierarchy.n_level]).graph.n_vertex
            }
            if (hierarchy.status != "CONVERGED") {
                if (requested_route == "AUTO" & diagonal_realistic) {
                    out.route = "DIAGONAL"
                    out.reason = "CMG setup failed before RNG; B1 passed bounded iteration, complete-residual, and work gates"
                    out.fallback_status = hierarchy.status
                    out.fallback_message = hierarchy.message
                }
                else {
                    if (requested_route == "CMG") {
                        out.estimator = kssbc__failure(
                            "FORCED_CMG_FAILED",hierarchy.message)
                    }
                    else out.estimator = kssbc__failure(
                        hierarchy.status,hierarchy.message)
                    out.status = out.estimator.status
                    out.message = out.estimator.message
                    out.route = "CMG"
                    out.reason = hierarchy.message
                    hierarchy_peak = max((
                        preflight.predicted_structural_bytes+
                            preflight.predicted_scratch_bytes,
                        hierarchy.structural_bytes+
                            hierarchy.dense_factor_bytes))
                    forecast_peak = base_bytes+hierarchy_peak
                    out.diagnostics = (planned_rhs,memory_envelope_bytes,
                        setup_seconds,hierarchy.n_level,
                        hierarchy.edge_complexity,hierarchy.vertex_complexity,
                        hierarchy.structural_bytes,hierarchy.dense_factor_bytes,
                        base.worker_levels,base.firm_levels,hybrid_vertices,
                        hybrid_edges,2,preflight.predicted_vertices,
                        preflight.predicted_edges,
                        preflight.predicted_structural_bytes,
                        preflight.predicted_scratch_bytes,pilot_cap,
                        diagonal_max_iterations,.,diagonal_seconds,
                        hierarchy_seconds,.,.,forecast_peak,
                        terminal_vertices)
                    return(out)
                }
            }
            else {
                workspace_capacity = max((4,batch,cols(controls)))
                timer_on(90)
                // Scale profiling rejects the reusable-workspace path: it was
                // 34--74% slower at 32,768 vertices (and about 63% slower at
                // 100,000 vertices).  Retain the certified API for equality
                // tests, but use the faster ordinary batched CMG application.
                cmg_context = kssbc_solver__cmg_context(
                    &hierarchy,workspace_capacity,hierarchy_memory_bytes,0)
                timer_off(90)
                hierarchy_seconds = kssbc__timer_seconds(90)
                cmg_backend = kssbc_solver__cmg_backend(&cmg_context)
                timer_clear(89)
                timer_on(89)
                cmg_pilots = kssbc__fe_solve_matrix_backend(
                    base,pilot_rhs,tolerance,maxiter,cmg_backend)
                timer_off(89)
                cmg_seconds = kssbc__timer_seconds(89)
                cmg_valid = kssbc_solver__pilots_valid(cmg_pilots,
                    min((pilot_cap,250)),tolerance)
                if (cmg_pilots.status == "CONVERGED") {
                    cmg_max_iterations = max(cmg_pilots.rhs_iterations)
                    cmg_work = kssbc_solver__cmg_work(
                        planned_rhs,cmg_max_iterations,base.n,
                        base.worker_levels,base.firm_levels,
                        hybrid_vertices,hybrid_edges,
                        hierarchy.edge_complexity,
                        hierarchy.vertex_complexity,
                        hierarchy.dense_factor_bytes)
                }
                if (!missing(diagonal_work) & diagonal_work > 0 &
                    !missing(cmg_work)) {
                    work_ratio = cmg_work/diagonal_work
                }
                use_cmg = cmg_valid &
                    (requested_route == "CMG" | !diagonal_realistic |
                     kssbc_solver__cmg_work_wins(work_ratio))
                if (requested_route == "CMG" & !cmg_valid) {
                    out.estimator = kssbc__failure(
                        "FORCED_CMG_FAILED",
                        "forced CMG pilots did not pass the bounded iteration and complete-residual gate")
                    out.status = out.estimator.status
                    out.message = out.estimator.message
                    out.route = "CMG"
                    out.reason = out.estimator.message
                    hierarchy_peak = max((
                        preflight.predicted_structural_bytes+
                            preflight.predicted_scratch_bytes,
                        hierarchy.structural_bytes+
                            hierarchy.dense_factor_bytes+
                            hierarchy.options.action_scratch_bytes))
                    if (cmg_context.use_workspace) {
                        hierarchy_peak = max((hierarchy_peak,
                            cmg_context.workspace.predicted_peak_bytes+
                                hierarchy.dense_factor_bytes))
                    }
                    forecast_peak = base_bytes+hierarchy_peak
                    out.diagnostics = (planned_rhs,memory_envelope_bytes,
                        setup_seconds+diagonal_seconds+hierarchy_seconds+
                            cmg_seconds,
                        hierarchy.n_level,hierarchy.edge_complexity,
                        hierarchy.vertex_complexity,
                        hierarchy.structural_bytes,
                        hierarchy.dense_factor_bytes,base.worker_levels,
                        base.firm_levels,hybrid_vertices,hybrid_edges,2,
                        preflight.predicted_vertices,
                        preflight.predicted_edges,
                        preflight.predicted_structural_bytes,
                        preflight.predicted_scratch_bytes,pilot_cap,
                        diagonal_max_iterations,cmg_max_iterations,
                        diagonal_seconds,hierarchy_seconds,cmg_seconds,
                        work_ratio,forecast_peak,terminal_vertices)
                    return(out)
                }
                if (use_cmg) {
                    backend = cmg_backend
                    out.route = "CMG"
                    route_code = 2
                    if (!diagonal_valid) {
                        out.reason =
                            "B1 pilots exceeded the bounded iteration or complete-residual gate"
                    }
                    else if (!diagonal_realistic) {
                        out.reason =
                            "B1 projected work exceeds the bounded fallback envelope"
                    }
                    else out.reason =
                        "CMG passed deterministic convergence and amortized-work gates"
                }
                else if (diagonal_realistic) {
                    out.route = "DIAGONAL"
                    out.reason =
                        "B1 passed bounded gates and CMG did not improve deterministic repeated-RHS work"
                    if (!cmg_valid) {
                        out.fallback_status = "CMG_PILOT_REJECTED"
                        out.fallback_message =
                            "CMG pilots did not pass the bounded iteration and complete-residual gate"
                    }
                }
                else {
                    out.estimator = kssbc__failure(
                        "NO_REALISTIC_SOLVER_ROUTE",
                        "neither B1 nor CMG passed bounded convergence, complete-residual, and deterministic-work gates")
                    out.status = out.estimator.status
                    out.message = out.estimator.message
                    out.route = "CMG"
                    out.reason = out.estimator.message
                    hierarchy_peak = max((
                        preflight.predicted_structural_bytes+
                            preflight.predicted_scratch_bytes,
                        hierarchy.structural_bytes+
                            hierarchy.dense_factor_bytes+
                            hierarchy.options.action_scratch_bytes))
                    if (cmg_context.use_workspace) {
                        hierarchy_peak = max((hierarchy_peak,
                            cmg_context.workspace.predicted_peak_bytes+
                                hierarchy.dense_factor_bytes))
                    }
                    forecast_peak = base_bytes+hierarchy_peak
                    out.diagnostics = (planned_rhs,memory_envelope_bytes,
                        setup_seconds+diagonal_seconds+hierarchy_seconds+
                            cmg_seconds,
                        hierarchy.n_level,hierarchy.edge_complexity,
                        hierarchy.vertex_complexity,
                        hierarchy.structural_bytes,
                        hierarchy.dense_factor_bytes,base.worker_levels,
                        base.firm_levels,hybrid_vertices,hybrid_edges,2,
                        preflight.predicted_vertices,
                        preflight.predicted_edges,
                        preflight.predicted_structural_bytes,
                        preflight.predicted_scratch_bytes,pilot_cap,
                        diagonal_max_iterations,cmg_max_iterations,
                        diagonal_seconds,hierarchy_seconds,cmg_seconds,
                        work_ratio,forecast_peak,terminal_vertices)
                    return(out)
                }
            }
        }
    }

    setup_seconds = setup_seconds+diagonal_seconds+hierarchy_seconds+cmg_seconds
    // Enforce the solver reservation before kssbc__jla_backend initializes
    // the registered random stream.  A direct B1 route does not reserve a
    // hypothetical CMG graph; its persistent FE-design storage is linear in
    // the retained rows and coordinates.
    forecast_peak = base_bytes
    if (hierarchy.status == "CONVERGED") {
        hierarchy_peak = hierarchy.structural_bytes+
            hierarchy.dense_factor_bytes+
            hierarchy.options.action_scratch_bytes
        if (cmg_context.use_workspace) {
            hierarchy_peak = max((hierarchy_peak,
                cmg_context.workspace.predicted_peak_bytes+
                    hierarchy.dense_factor_bytes))
        }
        forecast_peak = base_bytes+hierarchy_peak
    }
    else if (hierarchy.attempted_n_level > 0) {
        hierarchy_peak = max((
            preflight.predicted_structural_bytes+
                preflight.predicted_scratch_bytes,
            hierarchy.structural_bytes+hierarchy.dense_factor_bytes))
        forecast_peak = base_bytes+hierarchy_peak
    }
    if (missing(forecast_peak) | forecast_peak > solver_memory_bytes) {
        out.estimator = kssbc__failure(
            "SOLVER_MEMORY_LIMIT",
            "solver forecast exceeds its reserved 65% memory envelope")
        out.status = out.estimator.status
        out.message = out.estimator.message
        out.reason = out.estimator.message
        out.diagnostics = (planned_rhs,memory_envelope_bytes,setup_seconds,
            hierarchy.n_level,hierarchy.edge_complexity,
            hierarchy.vertex_complexity,hierarchy.structural_bytes,
            hierarchy.dense_factor_bytes,base.worker_levels,base.firm_levels,
            hybrid_vertices,hybrid_edges,route_code,
            preflight.predicted_vertices,preflight.predicted_edges,
            preflight.predicted_structural_bytes,
            preflight.predicted_scratch_bytes,pilot_cap,
            diagonal_max_iterations,cmg_max_iterations,
            diagonal_seconds,hierarchy_seconds,cmg_seconds,work_ratio,
            forecast_peak,terminal_vertices)
        return(out)
    }
    out.estimator = kssbc__jla_backend(
        y,worker,firm,controls,frequency,target_weight,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit,
        base,backend,setup_seconds)
    out.status = out.estimator.status
    out.message = out.estimator.message
    out.diagnostics = (planned_rhs,memory_envelope_bytes,setup_seconds,
        hierarchy.n_level,hierarchy.edge_complexity,
        hierarchy.vertex_complexity,hierarchy.structural_bytes,
        hierarchy.dense_factor_bytes,base.worker_levels,base.firm_levels,
        hybrid_vertices,hybrid_edges,route_code,
        preflight.predicted_vertices,preflight.predicted_edges,
        preflight.predicted_structural_bytes,
        preflight.predicted_scratch_bytes,pilot_cap,
        diagonal_max_iterations,cmg_max_iterations,
        diagonal_seconds,hierarchy_seconds,cmg_seconds,work_ratio,
        forecast_peak,terminal_vertices)
    return(out)
}

void kssbc__stata_jla_routed(
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
    string scalar requested_route,
    real scalar memory_envelope_bytes,
    string scalar results_name,
    string scalar status_local,
    string scalar message_local,
    string scalar diagnostics_name,
    string scalar solver_diagnostics_name,
    string scalar route_diagnostics_name,
    string scalar route_local,
    string scalar reason_local,
    string scalar fallback_status_local,
    string scalar fallback_message_local)
{
    struct kssbc_route_result scalar routed
    struct kssbc_result scalar out
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
    routed = kssbc_solver__jla_routed(
        y,worker,firm,controls,frequency,target,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit,
        requested_route,memory_envelope_bytes)
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
    st_matrix(route_diagnostics_name,routed.diagnostics)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
    st_local(route_local,routed.route)
    st_local(reason_local,routed.reason)
    st_local(fallback_status_local,routed.fallback_status)
    st_local(fallback_message_local,routed.fallback_message)
}

end
