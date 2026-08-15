*! Forced test-only KSS/CMG adapter; not part of the installed package

version 18.0

mata:
mata set matastrict on
mata set matalnum on

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

struct kssbc_solve_result scalar kssbc__fe_solve_matrix_cmg(
    struct kssbc_fe_design scalar design,
    struct kssbc_cmg__hierarchy scalar hierarchy,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct kssbc_solve_result scalar out
    struct kssbc_cmg__apply_result scalar applied
    real scalar workers, firms, columns, column, iteration, active_count
    real scalar denominator, alpha, rz_new, normalization
    real matrix worker_rhs, firm_rhs, full_firm_rhs, full_rhs, worker_base
    real matrix reduced_rhs, firm_coefficient, residual, preconditioned
    real matrix direction, action, replacement_argument, explicit_residual
    real matrix fitted, worker_coefficient, worker_lhs, firm_lhs
    real matrix full_residual
    real rowvector reduced_scale, rz, active, restart

    out.status = "INVALID_INPUT"
    out.message = "invalid forced-CMG right-hand side"
    out.rhs_status = J(1,cols(right_hand_side),"INVALID_INPUT")
    out.coefficient = J(0,0,.)
    out.iterations = .
    out.relres = .
    out.rhs_iterations = J(1,cols(right_hand_side),.)
    out.rhs_relres = J(1,cols(right_hand_side),.)
    out.schur_actions = 0
    out.schur_batches = 0
    out.preconditioner_applications = 0
    out.preconditioner_batches = 0
    out.schur_seconds = 0
    out.preconditioner_seconds = 0
    out.pcg_seconds = 0
    workers = design.worker_levels
    firms = design.firm_levels
    columns = cols(right_hand_side)
    if (design.status != "CONVERGED" | hierarchy.status != "CONVERGED" |
        columns < 1 | rows(right_hand_side) != workers+firms-1 |
        hasmissing(right_hand_side)) return(out)

    timer_clear(96)
    timer_clear(97)
    timer_clear(98)
    timer_on(96)
    worker_rhs = right_hand_side[1..workers,.]
    firm_rhs = right_hand_side[(workers+1)..rows(right_hand_side),.]
    full_firm_rhs = firm_rhs \
        (colsum(worker_rhs)-colsum(firm_rhs))
    full_rhs = worker_rhs \
        full_firm_rhs
    worker_base = worker_rhs :/ design.worker_weight
    reduced_rhs = full_firm_rhs-kssbc__group_sum(
        design.frequency:*worker_base[design.worker,.],
        design.firm_order,design.firm_panel)
    reduced_rhs = reduced_rhs-
        J(firms,1,1)*(colsum(reduced_rhs):/firms)
    reduced_scale = J(1,columns,.)
    firm_coefficient = J(firms,columns,0)
    residual = reduced_rhs
    direction = J(firms,columns,0)
    rz = J(1,columns,0)
    active = J(1,columns,0)
    out.rhs_status = J(1,columns,"PENDING")
    out.rhs_iterations = J(1,columns,0)
    for (column=1; column<=columns; column++) {
        reduced_scale[column] = kssbc__norm2(reduced_rhs[.,column])
        if (reduced_scale[column] == 0) {
            out.rhs_status[column] = "CONVERGED"
        }
        else active[column] = 1
    }
    active_count = sum(active)
    if (active_count > 0) {
        timer_on(98)
        applied = kssbc_cmg__apply_kss(hierarchy,residual)
        timer_off(98)
        if (applied.status != "CONVERGED") {
            out.status = applied.status
            out.message = applied.message
            timer_off(96)
            out.preconditioner_seconds = kssbc__timer_seconds(98)
            out.pcg_seconds = kssbc__timer_seconds(96)
            return(out)
        }
        preconditioned = applied.value
        out.preconditioner_applications = active_count
        out.preconditioner_batches = 1
        for (column=1; column<=columns; column++) {
            if (!active[column]) {
                preconditioned[.,column] = J(firms,1,0)
                continue
            }
            direction[.,column] = preconditioned[.,column]
            rz[column] =
                (residual[.,column]'*preconditioned[.,column])[1,1]
            if (rz[column] <= 0 | rz[column] >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "forced CMG is not positive on an active RHS"
                out.rhs_status[column] = out.status
                timer_off(96)
                out.preconditioner_seconds = kssbc__timer_seconds(98)
                out.pcg_seconds = kssbc__timer_seconds(96)
                return(out)
            }
        }
    }

    for (iteration=1; iteration<=maxiter & active_count>0; iteration++) {
        restart = J(1,columns,0)
        timer_on(97)
        action = kssbc__fe_schur_action(design,direction)
        timer_off(97)
        out.schur_actions = out.schur_actions+active_count
        out.schur_batches = out.schur_batches+1
        for (column=1; column<=columns; column++) {
            if (!active[column]) continue
            denominator =
                (direction[.,column]'*action[.,column])[1,1]
            if (denominator <= 0 | denominator >= . |
                rz[column] <= 0 | rz[column] >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "forced-CMG PCG lost positive curvature"
                out.rhs_status[column] = out.status
                timer_off(96)
                out.schur_seconds = kssbc__timer_seconds(97)
                out.preconditioner_seconds = kssbc__timer_seconds(98)
                out.pcg_seconds = kssbc__timer_seconds(96)
                return(out)
            }
            alpha = rz[column]/denominator
            firm_coefficient[.,column] =
                firm_coefficient[.,column]+alpha:*direction[.,column]
            residual[.,column] = residual[.,column]-alpha:*action[.,column]
            residual[.,column] = residual[.,column]:-
                mean(residual[.,column])
        }
        if (mod(iteration,100) == 0) {
            replacement_argument = firm_coefficient
            for (column=1; column<=columns; column++) {
                if (!active[column]) {
                    replacement_argument[.,column] = J(firms,1,0)
                }
            }
            timer_on(97)
            action = kssbc__fe_schur_action(design,replacement_argument)
            timer_off(97)
            out.schur_actions = out.schur_actions+active_count
            out.schur_batches = out.schur_batches+1
            explicit_residual = reduced_rhs-action
            for (column=1; column<=columns; column++) {
                if (!active[column]) continue
                explicit_residual[.,column] =
                    explicit_residual[.,column]:-
                    mean(explicit_residual[.,column])
                if (kssbc__norm2(explicit_residual[.,column]) <=
                    tolerance*reduced_scale[column] |
                    kssbc__norm2(explicit_residual[.,column]-
                        residual[.,column]) >
                    max((1e-14*reduced_scale[column],
                        0.1*tolerance*reduced_scale[column]))) {
                    residual[.,column] = explicit_residual[.,column]
                    restart[column] = 1
                }
            }
        }
        for (column=1; column<=columns; column++) {
            if (!active[column]) continue
            if (kssbc__norm2(residual[.,column]) <=
                tolerance*reduced_scale[column]) {
                active[column] = 0
                out.rhs_status[column] = "CONVERGED"
                out.rhs_iterations[column] = iteration
                residual[.,column] = J(firms,1,0)
                direction[.,column] = J(firms,1,0)
            }
        }
        active_count = sum(active)
        if (active_count == 0) break
        timer_on(98)
        applied = kssbc_cmg__apply_kss(hierarchy,residual)
        timer_off(98)
        if (applied.status != "CONVERGED") {
            out.status = applied.status
            out.message = applied.message
            timer_off(96)
            out.schur_seconds = kssbc__timer_seconds(97)
            out.preconditioner_seconds = kssbc__timer_seconds(98)
            out.pcg_seconds = kssbc__timer_seconds(96)
            return(out)
        }
        preconditioned = applied.value
        out.preconditioner_applications =
            out.preconditioner_applications+active_count
        out.preconditioner_batches = out.preconditioner_batches+1
        for (column=1; column<=columns; column++) {
            if (!active[column]) {
                preconditioned[.,column] = J(firms,1,0)
                continue
            }
            rz_new =
                (residual[.,column]'*preconditioned[.,column])[1,1]
            if (rz_new <= 0 | rz_new >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "forced CMG returned nonpositive residual energy"
                out.rhs_status[column] = out.status
                timer_off(96)
                out.schur_seconds = kssbc__timer_seconds(97)
                out.preconditioner_seconds = kssbc__timer_seconds(98)
                out.pcg_seconds = kssbc__timer_seconds(96)
                return(out)
            }
            if (restart[column]) {
                direction[.,column] = preconditioned[.,column]
            }
            else {
                direction[.,column] = preconditioned[.,column]+
                    (rz_new/rz[column]):*direction[.,column]
            }
            rz[column] = rz_new
        }
    }
    if (active_count > 0) {
        out.status = "PCG_NONCONVERGENCE"
        out.message = "forced-CMG PCG exceeded maxiter()"
        for (column=1; column<=columns; column++) {
            if (active[column]) {
                out.rhs_status[column] = out.status
                out.rhs_iterations[column] = maxiter
            }
        }
        timer_off(96)
        out.schur_seconds = kssbc__timer_seconds(97)
        out.preconditioner_seconds = kssbc__timer_seconds(98)
        out.pcg_seconds = kssbc__timer_seconds(96)
        return(out)
    }

    for (column=1; column<=columns; column++) {
        normalization = firm_coefficient[firms,column]
        firm_coefficient[.,column] =
            firm_coefficient[.,column]:-normalization
    }
    fitted = firm_coefficient[design.firm,.]
    worker_coefficient = worker_base-kssbc__group_sum(
        design.frequency:*fitted,
        design.worker_order,design.worker_panel):/design.worker_weight
    out.coefficient = worker_coefficient \
        firm_coefficient[1..(firms-1),.]
    fitted = kssbc__fe_predict(design,out.coefficient)
    worker_lhs = kssbc__group_sum(
        design.frequency:*fitted,design.worker_order,design.worker_panel)
    firm_lhs = kssbc__group_sum(
        design.frequency:*fitted,design.firm_order,design.firm_panel)
    full_residual = (worker_lhs-worker_rhs) \
        (firm_lhs-full_firm_rhs)
    out.rhs_relres = kssbc__column_relres(full_residual,full_rhs)
    if (cols(out.rhs_relres) != columns | hasmissing(out.coefficient) |
        hasmissing(out.rhs_relres)) {
        out.status = "SOLVER_RESIDUAL_FAILED"
        out.message = "forced-CMG full residual is nonfinite"
    }
    else {
        for (column=1; column<=columns; column++) {
            if (out.rhs_relres[column] > max((1e-11,10*tolerance))) {
                out.status = "SOLVER_RESIDUAL_FAILED"
                out.message = "forced-CMG full residual exceeds tolerance"
                out.rhs_status[column] = out.status
                break
            }
        }
    }
    timer_off(96)
    out.schur_seconds = kssbc__timer_seconds(97)
    out.preconditioner_seconds = kssbc__timer_seconds(98)
    out.pcg_seconds = kssbc__timer_seconds(96)
    if (out.status == "SOLVER_RESIDUAL_FAILED") return(out)
    out.status = "CONVERGED"
    out.message = "forced test-only CMG KSS solves converged"
    out.iterations = max(out.rhs_iterations)
    out.relres = max(out.rhs_relres)
    return(out)
}

end
