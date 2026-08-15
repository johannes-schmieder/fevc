version 18.0
clear all
set more off
set varabbrev off

args repository_root vertices batch_columns output_csv
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    exit 198
}
capture confirm integer number `vertices'
if _rc | `vertices' < 2 exit 198
capture confirm integer number `batch_columns'
if _rc | `batch_columns' < 1 exit 198

mata: mata clear
do `"`repository_root'/shared/cmg/generated/cmg_test.mata"'

mata:
struct cmgtest__graph scalar cmghier__path(real scalar vertices)
{
    struct cmgtest__graph scalar graph

    graph = cmgtest__empty_graph()
    graph.n_firm = vertices
    graph.n_vertex = vertices
    graph.n_edge = vertices-1
    graph.n_auxiliary = 0
    graph.weight_scale = 1
    graph.u = (1::(vertices-1))
    graph.v = (2::vertices)
    graph.weight = J(vertices-1,1,1)
    graph.key_primary = (1::vertices)
    graph.key_type = J(vertices,1,0)
    graph.auxiliary_worker = J(0,1,.)
    graph.status = "CONVERGED"
    return(cmgtest__graph_finalize(graph))
}

void cmghier__run(real scalar vertices, real scalar columns)
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__apply_result scalar applied
    real matrix rhs, timer_result
    real scalar column, setup_seconds, apply_seconds

    graph = cmghier__path(vertices)
    options = cmgtest__options_default()
    timer_clear(1)
    timer_on(1)
    hierarchy = cmgtest__hierarchy_build(graph,options)
    timer_off(1)
    timer_result = timer_value(1)
    setup_seconds = timer_result[1,1]
    if (hierarchy.status != "CONVERGED") {
        errprintf("hierarchy benchmark failed: %s: %s\n",
            hierarchy.status,hierarchy.message)
        exit(3498)
    }
    rhs = J(vertices,columns,.)
    for (column=1; column<=columns; column++) {
        rhs[.,column] = sin((1::vertices):*(column/(columns+1)))
        rhs[.,column] = rhs[.,column]:-mean(rhs[.,column])
    }
    timer_clear(2)
    timer_on(2)
    applied = cmgtest__apply(hierarchy,rhs)
    timer_off(2)
    timer_result = timer_value(2)
    apply_seconds = timer_result[1,1]
    assert(applied.status == "CONVERGED")

    st_numscalar("cmgh_vertices",vertices)
    st_numscalar("cmgh_edges",graph.n_edge)
    st_numscalar("cmgh_columns",columns)
    st_numscalar("cmgh_setup",setup_seconds)
    st_numscalar("cmgh_apply",apply_seconds)
    st_numscalar("cmgh_levels",hierarchy.n_level)
    st_numscalar("cmgh_edge_complexity",hierarchy.edge_complexity)
    st_numscalar("cmgh_vertex_complexity",hierarchy.vertex_complexity)
    st_numscalar("cmgh_structural_bytes",hierarchy.structural_bytes)
    st_numscalar("cmgh_dense_bytes",hierarchy.dense_factor_bytes)
}

cmghier__run(strtoreal(st_local("vertices")),
    strtoreal(st_local("batch_columns")))
end

clear
quietly set obs 1
generate str8 stata_version = "`c(stata_version)'"
generate double processors = c(processors)
generate double vertices = scalar(cmgh_vertices)
generate double edges = scalar(cmgh_edges)
generate double batch_columns = scalar(cmgh_columns)
generate double setup_seconds = scalar(cmgh_setup)
generate double apply_seconds = scalar(cmgh_apply)
generate double levels = scalar(cmgh_levels)
generate double edge_complexity = scalar(cmgh_edge_complexity)
generate double vertex_complexity = scalar(cmgh_vertex_complexity)
generate double structural_bytes = scalar(cmgh_structural_bytes)
generate double dense_factor_bytes = scalar(cmgh_dense_bytes)
export delimited using `"`output_csv'"', replace
di as result "CMG HIERARCHY BENCHMARK PASS"
exit 0
