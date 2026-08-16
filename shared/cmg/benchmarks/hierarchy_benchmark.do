version 18.0
clear all
set more off
set varabbrev off

args repository_root vertices batch_columns output_csv graph_family core_path
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    exit 198
}
capture confirm integer number `vertices'
if _rc | `vertices' < 2 exit 198
capture confirm integer number `batch_columns'
if _rc | `batch_columns' < 1 exit 198
if strtrim(`"`graph_family'"') == "" local graph_family "path"
if !inlist(`"`graph_family'"',"path","star","irregular","expander") exit 198
if strtrim(`"`core_path'"') == "" {
    local core_path `"`repository_root'/shared/cmg/generated/cmg_test.mata"'
}

mata: mata clear
do `"`core_path'"'

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

struct cmgtest__graph scalar cmghier__family(
    string scalar family,
    real scalar vertices)
{
    struct cmgtest__graph scalar graph
    real colvector u, v, weight
    real rowvector offset
    real scalar edge, cursor, one_offset, vertex, neighbor, temporary

    if (family == "path") return(cmghier__path(vertices))
    if (family == "star") {
        u = J(vertices-1,1,1)
        v = (2::vertices)
        weight = 1:+mod(v,11):/16
    }
    else if (family == "irregular") {
        u = J(2*vertices-3,1,.)
        v = J(2*vertices-3,1,.)
        weight = J(2*vertices-3,1,.)
        cursor = 0
        for (edge=1; edge<vertices; edge++) {
            cursor = cursor+1
            u[cursor] = edge
            v[cursor] = edge+1
            weight[cursor] = 1+mod(edge,13)
        }
        for (edge=3; edge<=vertices; edge++) {
            cursor = cursor+1
            u[cursor] = 1
            v[cursor] = edge
            weight[cursor] = 2^(-mod(edge,20))
        }
    }
    else {
        if (vertices < 64) return(cmgtest__empty_graph())
        offset = (1,7,31)
        u = J(vertices*cols(offset),1,.)
        v = J(vertices*cols(offset),1,.)
        weight = J(vertices*cols(offset),1,.)
        cursor = 0
        for (one_offset=1; one_offset<=cols(offset); one_offset++) {
            for (vertex=1; vertex<=vertices; vertex++) {
                neighbor = 1+mod(vertex-1+offset[one_offset],vertices)
                cursor = cursor+1
                u[cursor] = vertex
                v[cursor] = neighbor
                if (u[cursor] > v[cursor]) {
                    temporary = u[cursor]
                    u[cursor] = v[cursor]
                    v[cursor] = temporary
                }
                weight[cursor] =
                    1+mod(vertex+offset[one_offset],5)/16
            }
        }
        if (rows(uniqrows(sort((u,v),(1,2)))) != rows(u)) {
            return(cmgtest__empty_graph())
        }
    }
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

void cmghier__run(string scalar family, real scalar vertices,
    real scalar columns)
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__diag_result scalar diagnostics
    struct cmgtest__apply_result scalar applied
    real matrix rhs, timer_result
    real scalar column, setup_seconds, apply_seconds

    graph = cmghier__family(family,vertices)
    if (graph.status != "CONVERGED") {
        errprintf("hierarchy benchmark graph failed: %s: %s\n",
            graph.status,graph.message)
        exit(3498)
    }
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
    diagnostics = cmgtest__diagnostics(hierarchy)
    assert(diagnostics.status == "CONVERGED")

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
    st_numscalar("cmgh_attempted_levels",hierarchy.attempted_n_level)
    st_numscalar("cmgh_fallback_levels",sum(
        hierarchy.attempted_method :== "NORMALIZED_HEAVY_FALLBACK"))
    if (hierarchy.n_level > 1) {
        st_numscalar("cmgh_min_reduction",min(
            hierarchy.attempted_level_table[
                (1::(hierarchy.n_level-1)),6]))
    }
    else st_numscalar("cmgh_min_reduction",.)
}

cmghier__run(st_local("graph_family"),strtoreal(st_local("vertices")),
    strtoreal(st_local("batch_columns")))
end

clear
quietly set obs 1
generate str8 stata_version = "`c(stata_version)'"
generate str12 graph_family = `"`graph_family'"'
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
generate double attempted_levels = scalar(cmgh_attempted_levels)
generate double fallback_levels = scalar(cmgh_fallback_levels)
generate double minimum_reduction = scalar(cmgh_min_reduction)
export delimited using `"`output_csv'"', replace
di as result "CMG HIERARCHY BENCHMARK PASS"
exit 0
