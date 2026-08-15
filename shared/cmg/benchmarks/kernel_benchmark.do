version 18.0
clear all
set more off
set varabbrev off

args repository_root vertices batch_columns repetitions output_csv
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    exit 198
}
foreach value in vertices batch_columns repetitions {
    capture confirm integer number ``value''
    if _rc | ``value'' < 1 exit 198
}
if `vertices' < 2 exit 198

mata: mata clear
do `"`repository_root'/shared/cmg/generated/cmg_test.mata"'

mata:
struct cmgbench__arcs
{
    real colvector vertex
    real colvector edge
    real colvector sign
    real matrix panel
    real colvector unique_vertex
    real scalar predicted_bytes
}

struct cmgtest__graph scalar cmgbench__path(real scalar vertices)
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

struct cmgbench__arcs scalar cmgbench__prepare(
    struct cmgtest__graph scalar graph)
{
    struct cmgbench__arcs scalar out
    real colvector permutation
    real matrix key

    out.vertex = graph.u\graph.v
    out.edge = (1::graph.n_edge)\(1::graph.n_edge)
    out.sign = J(graph.n_edge,1,1)\J(graph.n_edge,1,-1)
    key = (out.vertex,out.edge)
    permutation = order(key,(1,2))
    out.vertex = out.vertex[permutation]
    out.edge = out.edge[permutation]
    out.sign = out.sign[permutation]
    out.panel = panelsetup(out.vertex,1)
    out.unique_vertex = out.vertex[out.panel[.,1]]
    out.predicted_bytes = 8*(3*rows(out.edge)+2*rows(out.panel))
    return(out)
}

real matrix cmgbench__action(
    struct cmgtest__graph scalar graph,
    struct cmgbench__arcs scalar arcs,
    real matrix argument)
{
    real matrix out, contribution

    contribution = arcs.sign :* graph.weight[arcs.edge] :*
        (argument[graph.u[arcs.edge],.]-argument[graph.v[arcs.edge],.])
    out = J(graph.n_vertex,cols(argument),0)
    out[arcs.unique_vertex,.] = panelsum(contribution,arcs.panel)
    return(out)
}

void cmgbench__run(real scalar vertices, real scalar columns,
    real scalar repetitions)
{
    struct cmgtest__graph scalar graph
    struct cmgbench__arcs scalar arcs
    real matrix argument, production, directed, timer_result
    real scalar column, repetition, bounded_seconds, unbounded_seconds
    real scalar relative_error, scale

    graph = cmgbench__path(vertices)
    assert(graph.status == "CONVERGED")
    arcs = cmgbench__prepare(graph)
    argument = J(vertices,columns,.)
    for (column=1; column<=columns; column++) {
        argument[.,column] = sin((1::vertices):*(column/(columns+1)))
    }
    production = cmgtest__graph_action(graph,argument,64*1024^2)
    directed = cmgbench__action(graph,arcs,argument)
    scale = max((1,max(abs(directed))))
    relative_error = max(abs(production-directed))/scale
    assert(relative_error <= 2e-13)

    timer_clear(1)
    timer_on(1)
    for (repetition=1; repetition<=repetitions; repetition++) {
        production = cmgtest__graph_action(graph,argument,64*1024^2)
    }
    timer_off(1)
    timer_result = timer_value(1)
    bounded_seconds = timer_result[1,1]

    timer_clear(2)
    timer_on(2)
    for (repetition=1; repetition<=repetitions; repetition++) {
        directed = cmgbench__action(graph,arcs,argument)
    }
    timer_off(2)
    timer_result = timer_value(2)
    unbounded_seconds = timer_result[1,1]

    st_numscalar("cmg_vertices",vertices)
    st_numscalar("cmg_edges",graph.n_edge)
    st_numscalar("cmg_columns",columns)
    st_numscalar("cmg_repetitions",repetitions)
    st_numscalar("cmg_bounded_seconds",bounded_seconds)
    st_numscalar("cmg_unbounded_seconds",unbounded_seconds)
    st_numscalar("cmg_unbounded_over_bounded",
        unbounded_seconds/bounded_seconds)
    st_numscalar("cmg_relative_error",relative_error)
    st_numscalar("cmg_production_bytes",graph.predicted_bytes)
    st_numscalar("cmg_directed_extra_bytes",arcs.predicted_bytes)
}

cmgbench__run(strtoreal(st_local("vertices")),
    strtoreal(st_local("batch_columns")),
    strtoreal(st_local("repetitions")))
end

clear
quietly set obs 1
generate str8 stata_version = "`c(stata_version)'"
generate double processors = c(processors)
generate double vertices = scalar(cmg_vertices)
generate double edges = scalar(cmg_edges)
generate double batch_columns = scalar(cmg_columns)
generate double repetitions = scalar(cmg_repetitions)
generate double bounded_directed_seconds = scalar(cmg_bounded_seconds)
generate double unbounded_directed_seconds = scalar(cmg_unbounded_seconds)
generate double unbounded_over_bounded = scalar(cmg_unbounded_over_bounded)
generate double relative_error = scalar(cmg_relative_error)
generate double production_structural_bytes = scalar(cmg_production_bytes)
generate double directed_extra_bytes = scalar(cmg_directed_extra_bytes)
export delimited using `"`output_csv'"', replace
di as result "CMG KERNEL BENCHMARK PASS"
exit 0
