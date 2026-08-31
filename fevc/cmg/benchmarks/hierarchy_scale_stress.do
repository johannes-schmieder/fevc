version 18.0
clear all
set more off
set varabbrev off

args repository_root output_csv scale vertices batch_columns coarse_max core_path
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    di as error "usage: do hierarchy_scale_stress.do repository_root " ///
        "output_csv [local|cz18|scc] [vertices] [batch_columns] " ///
        "[coarse_max] [core_path]"
    exit 198
}
if strtrim(`"`scale'"') == "" local scale "local"
if !inlist(`"`scale'"',"local","cz18","scc") exit 198
if strtrim(`"`vertices'"') == "" {
    if `"`scale'"' == "local" local vertices 8192
    else if `"`scale'"' == "cz18" local vertices 32768
    else local vertices 65536
}
if strtrim(`"`batch_columns'"') == "" {
    if `"`scale'"' == "scc" local batch_columns 8
    else local batch_columns 4
}
if strtrim(`"`coarse_max'"') == "" local coarse_max 128
capture confirm integer number `vertices'
if _rc | `vertices' < 8192 exit 198
capture confirm integer number `batch_columns'
if _rc | `batch_columns' < 2 exit 198
capture confirm integer number `coarse_max'
if _rc | `coarse_max' < 16 | `coarse_max' > 512 exit 198
if `coarse_max' >= `vertices' exit 198
if strtrim(`"`core_path'"') == "" {
    local core_path `"`repository_root'/fevc/cmg/generated/cmg_test.mata"'
}

mata: mata clear
quietly do `"`core_path'"'

mata:
struct cmgtest__graph scalar cmgscale__graph(real scalar vertices)
{
    struct cmgtest__graph scalar graph
    real colvector u, v, weight, index
    real scalar quarter

    quarter = floor(vertices/4)

    // A path joins two sparse, hub-rich lobes through two weak necks.  The
    // middle has deterministic long chords and highly irregular weights.
    // Each edge family has a distinct endpoint pattern, so no parallel edge
    // needs implicit collapsing.
    u = (1::(vertices-1))
    v = (2::vertices)
    weight = 1:+mod((1::(vertices-1)),17):/32
    weight[quarter] = 1/1024
    weight[3*quarter] = 1/2048

    index = (1::(quarter-2))
    u = u \ index
    v = v \ (index:+2)
    weight = weight \ (2:+mod(index,11):/16)

    index = ((3*quarter+1)::(vertices-2))
    u = u \ index
    v = v \ (index:+2)
    weight = weight \ (3:+mod(index,13):/32)

    index = select((18::quarter),
        mod((18::quarter):-18,16):==0)
    u = u \ J(rows(index),1,1)
    v = v \ index
    weight = weight \ (4:+mod(index,19):/16)

    index = select(((3*quarter)::(vertices-64)),
        mod(((3*quarter)::(vertices-64)):-3*quarter,17):==0)
    u = u \ index
    v = v \ J(rows(index),1,vertices)
    weight = weight \ (5:+mod(index,23):/32)

    index = ((quarter+1)::(3*quarter-37))
    u = u \ index
    v = v \ (index:+37)
    weight = weight \ (2:^(-mod(index,18)))

    graph = cmgtest__empty_graph()
    graph.n_firm = vertices
    graph.n_vertex = vertices
    graph.n_edge = rows(weight)
    graph.n_auxiliary = 0
    graph.weight_scale = 1
    graph.u = u
    graph.v = v
    graph.weight = weight
    graph.key_primary = (1::vertices)
    graph.key_type = J(vertices,1,0)
    graph.auxiliary_worker = J(0,1,.)
    graph.status = "CONVERGED"
    return(cmgtest__graph_finalize(graph))
}

void cmgscale__run(real scalar vertices, real scalar columns,
    real scalar coarse_max)
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar first, second
    struct cmgtest__diag_result scalar diagnostics
    struct cmgtest__apply_result scalar applied, applied_second, workspace_applied
    struct cmgtest__workspace scalar workspace
    struct cmgtest__level scalar level, second_level, terminal
    real matrix rhs, other_rhs, action, timer_result
    real scalar column, level_index, setup_seconds, rebuild_seconds
    real scalar apply_seconds, relative_residual, left, right, scale
    real scalar minimum_reduction, terminal_vertices, predicted_peak
    real scalar workspace_mreldif
    real scalar repetition, repetitions, timed_applications
    real scalar ordinary_seconds, workspace_seconds, workspace_time_ratio

    assert(cmgtest__api_level() == 8)
    graph = cmgscale__graph(vertices)
    if (graph.status != "CONVERGED") {
        errprintf("scale graph failed: %s: %s\n",graph.status,graph.message)
        exit(3498)
    }
    assert(graph.n_vertex > 6144)
    options = cmgtest__options_default()
    options.coarse_max = coarse_max

    timer_clear(1)
    timer_on(1)
    first = cmgtest__hierarchy_build(graph,options)
    timer_off(1)
    timer_result = timer_value(1)
    setup_seconds = timer_result[1,1]
    if (first.status != "CONVERGED") {
        errprintf("scale hierarchy failed: %s: %s\n",
            first.status,first.message)
        exit(3498)
    }

    timer_clear(2)
    timer_on(2)
    second = cmgtest__hierarchy_build(graph,options)
    timer_off(2)
    timer_result = timer_value(2)
    rebuild_seconds = timer_result[1,1]
    assert(second.status == "CONVERGED")
    assert(first.n_level == second.n_level)
    assert(first.n_level >= 3)
    assert(first.attempted_n_level == first.n_level)
    assert(all(first.attempted_level_table :==
        second.attempted_level_table))
    assert(!any(first.attempted_status :!= second.attempted_status))
    assert(!any(first.attempted_method :!= second.attempted_method))
    minimum_reduction = 1
    for (level_index=1; level_index<first.n_level; level_index++) {
        level = *first.level[level_index]
        second_level = *second.level[level_index]
        assert(max(abs(level.aggregation-second_level.aggregation)) == 0)
        assert(first.attempted_status[level_index] == "COMMITTED")
        assert(first.attempted_level_table[level_index,6] >=
            options.min_reduction)
        minimum_reduction = min((minimum_reduction,
            first.attempted_level_table[level_index,6]))
    }
    terminal = *first.level[first.n_level]
    terminal_vertices = terminal.graph.n_vertex
    assert(terminal_vertices <= coarse_max)
    assert(first.attempted_status[first.n_level] == "CONVERGED")
    assert(first.attempted_method[first.n_level] == "DENSE_TERMINAL")
    assert(first.dense_factor_bytes <= options.dense_factor_bytes)

    rhs = J(vertices,columns,.)
    other_rhs = J(vertices,columns,.)
    for (column=1; column<=columns; column++) {
        rhs[.,column] = sin((1::vertices):*(column/(columns+3))) +
            cos((1::vertices):/((column+1)*31))
        other_rhs[.,column] = cos((1::vertices):*(column/(columns+5))) -
            sin((1::vertices):/((column+2)*23))
        rhs[.,column] = rhs[.,column]:-mean(rhs[.,column])
        other_rhs[.,column] = other_rhs[.,column]:-
            mean(other_rhs[.,column])
    }

    timer_clear(3)
    timer_on(3)
    applied = cmgtest__apply(first,rhs)
    applied_second = cmgtest__apply(first,other_rhs)
    timer_off(3)
    timer_result = timer_value(3)
    apply_seconds = timer_result[1,1]
    assert(applied.status == "CONVERGED")
    assert(applied_second.status == "CONVERGED")
    assert(applied.levels_visited == first.n_level)
    assert(applied.edge_passes == 2*(first.n_level-1))
    action = cmgtest__graph_action(graph,applied.value,
        options.action_scratch_bytes)
    assert(rows(action) == vertices)
    assert(!hasmissing(action))
    relative_residual = sqrt(quadcross(vec(rhs-action),vec(rhs-action)) /
        quadcross(vec(rhs),vec(rhs)))
    assert(!missing(relative_residual))
    assert(relative_residual <= 2)

    left = quadcross(vec(rhs),vec(applied_second.value))
    right = quadcross(vec(applied.value),vec(other_rhs))
    scale = max((1,abs(left),abs(right)))
    assert(abs(left-right)/scale <= 5e-11)
    assert(quadcross(vec(rhs),vec(applied.value)) > 0)
    assert(quadcross(vec(other_rhs),vec(applied_second.value)) > 0)

    workspace = cmgtest__workspace_init(first,columns,1024^3)
    assert(workspace.status == "CONVERGED")
    workspace_applied = cmgtest__workspace_apply(first,workspace,rhs)
    assert(workspace_applied.status == "CONVERGED")
    workspace_mreldif = mreldif(workspace_applied.value,applied.value)
    assert(workspace_mreldif <= 1e-13)

    // Warm both paths above, then time two blocks in opposite order.  Both
    // paths see the identical batched RHS exactly 2*repetitions times.  The
    // alternating block order reduces drift without timing assertions or
    // workspace construction.
    if (vertices <= 8192) repetitions = 64
    else if (vertices <= 32768) repetitions = 32
    else repetitions = 16

    timer_clear(4)
    timer_on(4)
    for (repetition=1; repetition<=repetitions; repetition++) {
        applied = cmgtest__apply(first,rhs)
    }
    timer_off(4)

    timer_clear(5)
    timer_on(5)
    for (repetition=1; repetition<=repetitions; repetition++) {
        workspace_applied = cmgtest__workspace_apply(first,workspace,rhs)
    }
    timer_off(5)

    timer_clear(6)
    timer_on(6)
    for (repetition=1; repetition<=repetitions; repetition++) {
        workspace_applied = cmgtest__workspace_apply(first,workspace,rhs)
    }
    timer_off(6)

    timer_clear(7)
    timer_on(7)
    for (repetition=1; repetition<=repetitions; repetition++) {
        applied = cmgtest__apply(first,rhs)
    }
    timer_off(7)

    timer_result = timer_value(4)
    ordinary_seconds = timer_result[1,1]
    timer_result = timer_value(7)
    ordinary_seconds = ordinary_seconds+timer_result[1,1]
    timer_result = timer_value(5)
    workspace_seconds = timer_result[1,1]
    timer_result = timer_value(6)
    workspace_seconds = workspace_seconds+timer_result[1,1]
    timed_applications = 2*repetitions
    assert(applied.status == "CONVERGED")
    assert(workspace_applied.status == "CONVERGED")
    workspace_mreldif = mreldif(workspace_applied.value,applied.value)
    assert(workspace_mreldif <= 1e-13)
    assert(ordinary_seconds > 0 & workspace_seconds > 0)
    workspace_time_ratio = workspace_seconds/ordinary_seconds
    assert(!missing(workspace_time_ratio) & workspace_time_ratio > 0)

    diagnostics = cmgtest__diagnostics(first)
    assert(diagnostics.status == "CONVERGED")
    assert(diagnostics.hierarchy_status == "CONVERGED")
    assert(diagnostics.fine_vertices == vertices)
    assert(diagnostics.fine_edges == graph.n_edge)
    assert(diagnostics.n_level == first.n_level)
    predicted_peak = first.structural_bytes+first.dense_factor_bytes+
        workspace.allocated_bytes+options.action_scratch_bytes

    st_numscalar("cmgscale_vertices",vertices)
    st_numscalar("cmgscale_edges",graph.n_edge)
    st_numscalar("cmgscale_columns",columns)
    st_numscalar("cmgscale_coarse_max",coarse_max)
    st_numscalar("cmgscale_levels",first.n_level)
    st_numscalar("cmgscale_terminal_vertices",terminal_vertices)
    st_numscalar("cmgscale_min_reduction",minimum_reduction)
    st_numscalar("cmgscale_edge_complexity",first.edge_complexity)
    st_numscalar("cmgscale_vertex_complexity",first.vertex_complexity)
    st_numscalar("cmgscale_structural_bytes",first.structural_bytes)
    st_numscalar("cmgscale_dense_bytes",first.dense_factor_bytes)
    st_numscalar("cmgscale_workspace_bytes",workspace.allocated_bytes)
    st_numscalar("cmgscale_predicted_peak",predicted_peak)
    st_numscalar("cmgscale_setup",setup_seconds)
    st_numscalar("cmgscale_rebuild",rebuild_seconds)
    st_numscalar("cmgscale_apply",apply_seconds)
    st_numscalar("cmgscale_relative_residual",relative_residual)
    st_numscalar("cmgscale_symmetry_error",abs(left-right)/scale)
    st_numscalar("cmgscale_workspace_mreldif",workspace_mreldif)
    st_numscalar("cmgscale_timed_applications",timed_applications)
    st_numscalar("cmgscale_ordinary_seconds",ordinary_seconds)
    st_numscalar("cmgscale_workspace_seconds",workspace_seconds)
    st_numscalar("cmgscale_workspace_ratio",workspace_time_ratio)
}

cmgscale__run(strtoreal(st_local("vertices")),
    strtoreal(st_local("batch_columns")),
    strtoreal(st_local("coarse_max")))
end

clear
quietly set obs 1
generate str8 stata_version = "`c(stata_version)'"
generate str8 stata_flavor = c(flavor)
generate byte stata_mp = c(MP)
generate str8 scale = `"`scale'"'
generate double processors = c(processors)
generate double vertices = scalar(cmgscale_vertices)
generate double edges = scalar(cmgscale_edges)
generate double batch_columns = scalar(cmgscale_columns)
generate double coarse_max = scalar(cmgscale_coarse_max)
generate double levels = scalar(cmgscale_levels)
generate double terminal_vertices = scalar(cmgscale_terminal_vertices)
generate double minimum_reduction = scalar(cmgscale_min_reduction)
generate double edge_complexity = scalar(cmgscale_edge_complexity)
generate double vertex_complexity = scalar(cmgscale_vertex_complexity)
generate double structural_bytes = scalar(cmgscale_structural_bytes)
generate double dense_factor_bytes = scalar(cmgscale_dense_bytes)
generate double workspace_bytes = scalar(cmgscale_workspace_bytes)
generate double predicted_peak_bytes = scalar(cmgscale_predicted_peak)
generate double setup_seconds = scalar(cmgscale_setup)
generate double rebuild_seconds = scalar(cmgscale_rebuild)
generate double two_apply_seconds = scalar(cmgscale_apply)
generate double relative_residual = scalar(cmgscale_relative_residual)
generate double symmetry_relative_error = scalar(cmgscale_symmetry_error)
generate double workspace_mreldif = scalar(cmgscale_workspace_mreldif)
generate double timed_applications = scalar(cmgscale_timed_applications)
generate double ordinary_seconds = scalar(cmgscale_ordinary_seconds)
generate double workspace_seconds = scalar(cmgscale_workspace_seconds)
generate double workspace_time_ratio = scalar(cmgscale_workspace_ratio)
export delimited using `"`output_csv'"', replace

di as result "CMG HIERARCHY SCALE STRESS PASS"
di as text "vertices=" %12.0fc scalar(cmgscale_vertices) ///
    " edges=" %12.0fc scalar(cmgscale_edges) ///
    " levels=" %4.0f scalar(cmgscale_levels) ///
    " terminal=" %8.0fc scalar(cmgscale_terminal_vertices)
di as text "setup=" %9.3f scalar(cmgscale_setup) ///
    "s rebuild=" %9.3f scalar(cmgscale_rebuild) ///
    "s two_apply=" %9.3f scalar(cmgscale_apply) "s"
di as text "timed applications/path=" ///
    %6.0f scalar(cmgscale_timed_applications) ///
    " ordinary=" %9.3f scalar(cmgscale_ordinary_seconds) ///
    "s workspace=" %9.3f scalar(cmgscale_workspace_seconds) ///
    "s ratio=" %7.4f scalar(cmgscale_workspace_ratio)
exit 0
