version 18.0
clear all
set more off
set varabbrev off

args repository_root graph_family vertices batch_columns tolerance maxiter output_csv coarse_arg scratch_mib_arg
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    exit 198
}
if !inlist(`"`graph_family'"',"path","ring","barbell") exit 198
capture confirm integer number `vertices'
if _rc | `vertices' < 12 exit 198
capture confirm integer number `batch_columns'
if _rc | `batch_columns' < 1 exit 198
capture confirm number `tolerance'
if _rc | `tolerance' <= 0 exit 198
capture confirm integer number `maxiter'
if _rc | `maxiter' < 1 exit 198
local coarse_max = cond(strtrim("`coarse_arg'") == "", 128, real("`coarse_arg'"))
local scratch_mib = cond(strtrim("`scratch_mib_arg'") == "", 64, real("`scratch_mib_arg'"))
// API 4 permits the memory-rich repeated-RHS terminal through 6,144.
if missing(`coarse_max') | !inrange(`coarse_max',2,6144) | ///
    missing(`scratch_mib') | `scratch_mib' < 1 exit 198

mata: mata clear
do `"`repository_root'/fevc/cmg/generated/cmg_test.mata"'

mata:
struct cmgsolve__result
{
    string scalar status
    string rowvector column_status
    real rowvector iterations
    real rowvector relres
    real scalar operator_calls
    real scalar preconditioner_calls
}

struct cmgsolve__result scalar cmgsolve__empty(real scalar columns)
{
    struct cmgsolve__result scalar out

    out.status = "INVALID_INPUT"
    out.column_status = J(1,columns,"INVALID_INPUT")
    out.iterations = J(1,columns,0)
    out.relres = J(1,columns,.)
    out.operator_calls = 0
    out.preconditioner_calls = 0
    return(out)
}

real scalar cmgsolve__median(real rowvector values)
{
    real colvector ordered
    real scalar count, middle

    if (cols(values) == 0 | hasmissing(values)) return(.)
    ordered = sort(values',1)
    count = rows(ordered)
    middle = floor((count+1)/2)
    if (mod(count,2)) return(ordered[middle])
    return((ordered[middle]+ordered[middle+1])/2)
}

struct cmgtest__graph scalar cmgsolve__graph(
    string scalar family, real scalar vertices)
{
    struct cmgtest__graph scalar graph
    real colvector raw_u, raw_v, raw_weight
    real scalar edge, half, group, offset, local_vertex, next, chord, cursor
    real scalar swap

    graph = cmgtest__empty_graph()
    graph.n_firm = vertices
    graph.n_vertex = vertices
    graph.n_auxiliary = 0
    graph.weight_scale = 1
    graph.key_primary = (1::vertices)
    graph.key_type = J(vertices,1,0)
    graph.auxiliary_worker = J(0,1,.)
    if (family == "path") {
        graph.u = (1::(vertices-1))
        graph.v = (2::vertices)
        graph.weight = J(vertices-1,1,1)
    }
    else if (family == "ring") {
        graph.u = (1::(vertices-1)) \ 1
        graph.v = (2::vertices) \ vertices
        graph.weight = J(vertices,1,1)
    }
    else {
        if (mod(vertices,2) | vertices/2 < 6) return(graph)
        half = vertices/2
        raw_u = J(2*vertices+1,1,.)
        raw_v = J(2*vertices+1,1,.)
        raw_weight = J(2*vertices+1,1,1)
        cursor = 0
        for (group=1; group<=2; group++) {
            offset = (group-1)*half
            for (local_vertex=1; local_vertex<=half; local_vertex++) {
                next = mod(local_vertex,half)+1
                chord = mod(local_vertex+1,half)+1
                cursor = cursor+1
                raw_u[cursor] = offset+local_vertex
                raw_v[cursor] = offset+next
                if (raw_v[cursor] < raw_u[cursor]) {
                    swap = raw_u[cursor]
                    raw_u[cursor] = raw_v[cursor]
                    raw_v[cursor] = swap
                }
                cursor = cursor+1
                raw_u[cursor] = offset+local_vertex
                raw_v[cursor] = offset+chord
                if (raw_v[cursor] < raw_u[cursor]) {
                    swap = raw_u[cursor]
                    raw_u[cursor] = raw_v[cursor]
                    raw_v[cursor] = swap
                }
            }
        }
        cursor = cursor+1
        raw_u[cursor] = half
        raw_v[cursor] = half+1
        raw_weight[cursor] = 1e-4
        graph.u = raw_u
        graph.v = raw_v
        graph.weight = raw_weight
    }
    graph.n_edge = rows(graph.weight)
    graph.status = "CONVERGED"
    return(cmgtest__graph_finalize(graph))
}

real matrix cmgsolve__rhs(real scalar vertices, real scalar columns)
{
    real matrix out, pilot
    real scalar column

    out = J(vertices,columns,0)
    pilot = cmgtest__pilot_rhs((1::vertices),J(vertices,1,1))
    for (column=1; column<=min((4,columns)); column++) {
        out[.,column] = pilot[.,column]
    }
    for (column=5; column<=columns; column++) {
        out[.,column] = sin((1::vertices):*((column+0.5)/(columns+7)))+
            0.5:*cos((1::vertices):*((2*column+1)/(2*columns+11)))
        out[.,column] = out[.,column]:-mean(out[.,column])
        out[.,column] = out[.,column]:/max(abs(out[.,column]))
    }
    return(out)
}

struct cmgsolve__result scalar cmgsolve__pcg(
    struct cmgtest__graph scalar graph,
    struct cmgtest__hierarchy scalar hierarchy,
    real matrix right_hand_side,
    real scalar use_cmg,
    real scalar tolerance,
    real scalar maxiter)
{
    struct cmgsolve__result scalar out
    struct cmgtest__apply_result scalar applied
    real matrix rhs, solution, residual, preconditioned, direction, action
    real rowvector scale, norm, rho, rho_new, denominator, alpha, beta, active
    real colvector use, inactive, bad
    real scalar columns, column, iteration

    columns = cols(right_hand_side)
    out = cmgsolve__empty(columns)
    if (graph.status != "CONVERGED" |
        (use_cmg & hierarchy.status != "CONVERGED") |
        rows(right_hand_side) != graph.n_vertex | columns < 1 |
        hasmissing(right_hand_side) | missing(tolerance) | tolerance <= 0 |
        maxiter < 1 | maxiter != floor(maxiter)) return(out)
    rhs = cmgtest__project_labels(
        cmgtest__components(graph),right_hand_side)
    scale = colmax(abs(rhs))
    scale = scale+(scale:==0)
    rhs = rhs:/(J(graph.n_vertex,1,1)*scale)
    norm = sqrt(colsum(rhs:^2))
    norm = norm+(norm:==0)
    solution = J(graph.n_vertex,columns,0)
    residual = rhs
    active = (sqrt(colsum(residual:^2)):/norm:>tolerance/2)
    out.column_status = J(1,columns,"MAXITER")
    for (column=1; column<=columns; column++) {
        if (!active[column]) out.column_status[column] = "CONVERGED_INITIAL"
    }
    inactive = selectindex((active:==0)')
    if (rows(inactive)) residual[,inactive] = J(graph.n_vertex,rows(inactive),0)
    if (use_cmg) {
        applied = cmgtest__apply(hierarchy,residual)
        if (applied.status != "CONVERGED") {
            out.status = applied.status
            return(out)
        }
        preconditioned = applied.value
    }
    else {
        preconditioned = residual:/(graph.degree*J(1,columns,1))
        preconditioned = cmgtest__project_labels(
            cmgtest__components(graph),preconditioned)
    }
    out.preconditioner_calls = 1
    direction = preconditioned
    rho = colsum(residual:*preconditioned)
    for (iteration=1; iteration<=maxiter & sum(active)>0; iteration++) {
        action = cmgtest__graph_action(graph,direction,64*1024^2)
        out.operator_calls = out.operator_calls+1
        if (rows(action) != graph.n_vertex) {
            out.status = "OPERATOR_BREAKDOWN"
            return(out)
        }
        denominator = colsum(direction:*action)
        bad = selectindex((active :&
            ((denominator:<=0):|(denominator:>=.)))')
        if (rows(bad)) {
            active[bad'] = J(1,rows(bad),0)
            out.column_status[bad'] = J(1,rows(bad),"CURVATURE_BREAKDOWN")
            out.iterations[bad'] = J(1,rows(bad),iteration)
            residual[,bad] = J(graph.n_vertex,rows(bad),0)
            direction[,bad] = J(graph.n_vertex,rows(bad),0)
        }
        use = selectindex(active')
        if (!rows(use)) break
        alpha = J(1,columns,0)
        alpha[use'] = rho[use']:/denominator[use']
        solution = solution+direction*diag(alpha)
        residual = residual-action*diag(alpha)
        out.relres = sqrt(colsum(residual:^2)):/norm
        for (column=1; column<=rows(use); column++) {
            if (out.relres[use[column]] <= tolerance/2) {
                active[use[column]] = 0
                out.column_status[use[column]] = "CONVERGED"
                out.iterations[use[column]] = iteration
            }
        }
        inactive = selectindex((active:==0)')
        if (rows(inactive)) {
            residual[,inactive] = J(graph.n_vertex,rows(inactive),0)
            direction[,inactive] = J(graph.n_vertex,rows(inactive),0)
        }
        if (!sum(active)) break
        if (mod(iteration,100) == 0) {
            residual = rhs-cmgtest__graph_action(
                graph,solution,64*1024^2)
            out.operator_calls = out.operator_calls+1
            if (rows(inactive)) {
                residual[,inactive] = J(graph.n_vertex,rows(inactive),0)
            }
        }
        if (use_cmg) {
            applied = cmgtest__apply(hierarchy,residual)
            out.preconditioner_calls = out.preconditioner_calls+1
            if (applied.status != "CONVERGED") {
                out.status = applied.status
                return(out)
            }
            preconditioned = applied.value
        }
        else {
            preconditioned = residual:/(graph.degree*J(1,columns,1))
            preconditioned = cmgtest__project_labels(
                cmgtest__components(graph),preconditioned)
            out.preconditioner_calls = out.preconditioner_calls+1
        }
        rho_new = colsum(residual:*preconditioned)
        bad = selectindex((active :& ((rho_new:<=0):|(rho_new:>=.)))')
        if (rows(bad)) {
            active[bad'] = J(1,rows(bad),0)
            out.column_status[bad'] = J(1,rows(bad),"PRECONDITIONER_BREAKDOWN")
            out.iterations[bad'] = J(1,rows(bad),iteration)
            residual[,bad] = J(graph.n_vertex,rows(bad),0)
            direction[,bad] = J(graph.n_vertex,rows(bad),0)
        }
        use = selectindex(active')
        if (!rows(use)) break
        beta = J(1,columns,0)
        beta[use'] = rho_new[use']:/rho[use']
        direction = preconditioned+direction*diag(beta)
        inactive = selectindex((active:==0)')
        if (rows(inactive)) direction[,inactive] = J(graph.n_vertex,rows(inactive),0)
        rho = rho_new
    }
    use = selectindex(active')
    if (rows(use)) {
        out.column_status[use'] = J(1,rows(use),"MAXITER")
        out.iterations[use'] = J(1,rows(use),maxiter)
    }
    residual = rhs-cmgtest__graph_action(graph,solution,64*1024^2)
    out.operator_calls = out.operator_calls+1
    out.relres = sqrt(colsum(residual:^2)):/norm
    for (column=1; column<=columns; column++) {
        if (out.relres[column] <= tolerance) {
            out.column_status[column] = "CONVERGED_RECOMPUTED"
        }
        else if (substr(out.column_status[column],1,9) == "CONVERGED") {
            out.column_status[column] = "FAILED_RESIDUAL_CHECK"
        }
    }
    out.status = "CONVERGED"
    for (column=1; column<=columns; column++) {
        if (substr(out.column_status[column],1,9) != "CONVERGED") {
            out.status = out.column_status[column]
            break
        }
    }
    return(out)
}

void cmgsolve__run(string scalar family, real scalar vertices,
    real scalar columns, real scalar tolerance, real scalar maxiter,
    real scalar coarse_max, real scalar scratch_mib)
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgsolve__result scalar diagonal, cmg
    real matrix rhs, timing
    real scalar setup_seconds, diagonal_seconds, cmg_seconds

    graph = cmgsolve__graph(family,vertices)
    if (graph.status != "CONVERGED") {
        errprintf("graph construction failed: %s: %s\n",
            graph.status,graph.message)
        exit(3498)
    }
    options = cmgtest__options_default()
    options.coarse_max = coarse_max
    options.action_scratch_bytes = scratch_mib*1024^2
    timer_clear(1)
    timer_on(1)
    hierarchy = cmgtest__hierarchy_build(graph,options)
    timer_off(1)
    timing = timer_value(1)
    setup_seconds = timing[1,1]
    if (hierarchy.status != "CONVERGED") {
        errprintf("hierarchy construction failed: %s: %s\n",
            hierarchy.status,hierarchy.message)
        exit(3498)
    }
    rhs = cmgsolve__rhs(vertices,columns)

    timer_clear(2)
    timer_on(2)
    diagonal = cmgsolve__pcg(
        graph,hierarchy,rhs,0,tolerance,maxiter)
    timer_off(2)
    timing = timer_value(2)
    diagonal_seconds = timing[1,1]

    timer_clear(3)
    timer_on(3)
    cmg = cmgsolve__pcg(graph,hierarchy,rhs,1,tolerance,maxiter)
    timer_off(3)
    timing = timer_value(3)
    cmg_seconds = timing[1,1]

    st_global("cmgsolve_diagonal_status",diagonal.status)
    st_global("cmgsolve_cmg_status",cmg.status)
    st_numscalar("cmgsolve_vertices",vertices)
    st_numscalar("cmgsolve_edges",graph.n_edge)
    st_numscalar("cmgsolve_columns",columns)
    st_numscalar("cmgsolve_tolerance",tolerance)
    st_numscalar("cmgsolve_maxiter",maxiter)
    st_numscalar("cmgsolve_setup",setup_seconds)
    st_numscalar("cmgsolve_diagonal",diagonal_seconds)
    st_numscalar("cmgsolve_cmg",cmg_seconds)
    st_numscalar("cmgsolve_diagonal_max",max(diagonal.iterations))
    st_numscalar("cmgsolve_cmg_max",max(cmg.iterations))
    st_numscalar("cmgsolve_diagonal_median",
        cmgsolve__median(diagonal.iterations))
    st_numscalar("cmgsolve_cmg_median",cmgsolve__median(cmg.iterations))
    st_numscalar("cmgsolve_diagonal_relres",max(diagonal.relres))
    st_numscalar("cmgsolve_cmg_relres",max(cmg.relres))
    st_numscalar("cmgsolve_levels",hierarchy.n_level)
    st_numscalar("cmgsolve_structural",hierarchy.structural_bytes)
    st_numscalar("cmgsolve_coarse_max",coarse_max)
    st_numscalar("cmgsolve_scratch_mib",scratch_mib)
}

cmgsolve__run(st_local("graph_family"),strtoreal(st_local("vertices")),
    strtoreal(st_local("batch_columns")),strtoreal(st_local("tolerance")),
    strtoreal(st_local("maxiter")),strtoreal(st_local("coarse_max")),
    strtoreal(st_local("scratch_mib")))
end

clear
quietly set obs 1
generate str8 stata_version = "`c(stata_version)'"
generate double processors = c(processors)
generate str12 graph_family = `"`graph_family'"'
generate double vertices = scalar(cmgsolve_vertices)
generate double edges = scalar(cmgsolve_edges)
generate double batch_columns = scalar(cmgsolve_columns)
generate double tolerance = scalar(cmgsolve_tolerance)
generate double maxiter = scalar(cmgsolve_maxiter)
generate str32 diagonal_status = "$cmgsolve_diagonal_status"
generate str32 cmg_status = "$cmgsolve_cmg_status"
generate double setup_seconds = scalar(cmgsolve_setup)
generate double diagonal_seconds = scalar(cmgsolve_diagonal)
generate double cmg_seconds = scalar(cmgsolve_cmg)
generate double diagonal_max_iterations = scalar(cmgsolve_diagonal_max)
generate double cmg_max_iterations = scalar(cmgsolve_cmg_max)
generate double diagonal_median_iterations = scalar(cmgsolve_diagonal_median)
generate double cmg_median_iterations = scalar(cmgsolve_cmg_median)
generate double diagonal_max_relres = scalar(cmgsolve_diagonal_relres)
generate double cmg_max_relres = scalar(cmgsolve_cmg_relres)
generate double hierarchy_levels = scalar(cmgsolve_levels)
generate double structural_bytes = scalar(cmgsolve_structural)
generate double coarse_max = scalar(cmgsolve_coarse_max)
generate double action_scratch_mib = scalar(cmgsolve_scratch_mib)
export delimited using `"`output_csv'"', replace
di as result "CMG SOLVER BENCHMARK PASS"
exit 0
