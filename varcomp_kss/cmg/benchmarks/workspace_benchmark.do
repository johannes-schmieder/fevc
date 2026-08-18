version 18.0
clear all
set more off
set varabbrev off

args repository_root vertices batch_columns repetitions output_csv
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    exit 198
}
capture confirm integer number `vertices'
if _rc | `vertices' < 2 exit 198
capture confirm integer number `batch_columns'
if _rc | `batch_columns' < 1 exit 198
capture confirm integer number `repetitions'
if _rc | `repetitions' < 1 exit 198

mata: mata clear
do `"`repository_root'/varcomp_kss/cmg/generated/cmg_test.mata"'

mata:
struct cmgtest__graph scalar cmgws__path(real scalar vertices)
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

void cmgws__run(real scalar vertices, real scalar columns,
    real scalar repetitions)
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__workspace scalar workspace
    struct cmgtest__apply_result scalar ordinary, reused
    real matrix rhs, timing
    real scalar column, repeat, init_seconds, ordinary_seconds, reused_seconds

    graph = cmgws__path(vertices)
    options = cmgtest__options_default()
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "CONVERGED")
    rhs = J(vertices,columns,.)
    for (column=1; column<=columns; column++) {
        rhs[.,column] = sin((1::vertices):*(column/(columns+1)))
        rhs[.,column] = rhs[.,column]:-mean(rhs[.,column])
    }

    timer_clear(1)
    timer_on(1)
    workspace = cmgtest__workspace_init(hierarchy,columns,4*1024^3)
    timer_off(1)
    timing = timer_value(1)
    init_seconds = timing[1,1]
    assert(workspace.status == "CONVERGED")

    // Warm both paths once before timing.
    ordinary = cmgtest__apply(hierarchy,rhs)
    reused = cmgtest__workspace_apply(hierarchy,workspace,rhs)
    assert(ordinary.status == "CONVERGED")
    assert(reused.status == "CONVERGED")
    assert(max(abs(ordinary.value:-reused.value)) <= 2e-12)

    timer_clear(2)
    timer_on(2)
    for (repeat=1; repeat<=repetitions; repeat++) {
        ordinary = cmgtest__apply(hierarchy,rhs)
    }
    timer_off(2)
    timing = timer_value(2)
    ordinary_seconds = timing[1,1]/repetitions

    timer_clear(3)
    timer_on(3)
    for (repeat=1; repeat<=repetitions; repeat++) {
        reused = cmgtest__workspace_apply(hierarchy,workspace,rhs)
    }
    timer_off(3)
    timing = timer_value(3)
    reused_seconds = timing[1,1]/repetitions

    st_numscalar("cmgws_vertices",vertices)
    st_numscalar("cmgws_edges",graph.n_edge)
    st_numscalar("cmgws_columns",columns)
    st_numscalar("cmgws_repetitions",repetitions)
    st_numscalar("cmgws_init",init_seconds)
    st_numscalar("cmgws_ordinary",ordinary_seconds)
    st_numscalar("cmgws_reused",reused_seconds)
    st_numscalar("cmgws_bytes",workspace.allocated_bytes)
    st_numscalar("cmgws_peak",workspace.predicted_peak_bytes)
}

cmgws__run(strtoreal(st_local("vertices")),
    strtoreal(st_local("batch_columns")),
    strtoreal(st_local("repetitions")))
end

clear
quietly set obs 1
generate str8 stata_version = "`c(stata_version)'"
generate double processors = c(processors)
generate double vertices = scalar(cmgws_vertices)
generate double edges = scalar(cmgws_edges)
generate double batch_columns = scalar(cmgws_columns)
generate double repetitions = scalar(cmgws_repetitions)
generate double workspace_init_seconds = scalar(cmgws_init)
generate double ordinary_apply_seconds = scalar(cmgws_ordinary)
generate double workspace_apply_seconds = scalar(cmgws_reused)
generate double workspace_bytes = scalar(cmgws_bytes)
generate double predicted_peak_bytes = scalar(cmgws_peak)
export delimited using `"`output_csv'"', replace
di as result "CMG WORKSPACE BENCHMARK PASS"
exit 0
