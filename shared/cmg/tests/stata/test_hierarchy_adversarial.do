version 18.0
clear all
set more off
set varabbrev off

args repository_root core_path
if strtrim(`"`repository_root'"') == "" {
    di as error "usage: do test_hierarchy_adversarial.do repository_root [core_path]"
    exit 198
}
if strtrim(`"`core_path'"') == "" {
    local core_path `"`repository_root'/shared/cmg/generated/cmg_test.mata"'
}

mata: mata clear
do `"`core_path'"'

mata:
struct cmgtest__graph scalar cmghadv__graph(
    real scalar vertices,
    real colvector u,
    real colvector v,
    real colvector weight)
{
    struct cmgtest__graph scalar graph

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

struct cmgtest__graph scalar cmghadv__path(real scalar vertices)
{
    return(cmghadv__graph(vertices,(1::(vertices-1)),(2::vertices),
        J(vertices-1,1,1)))
}

struct cmgtest__graph scalar cmghadv__star(real scalar vertices)
{
    return(cmghadv__graph(vertices,J(vertices-1,1,1),(2::vertices),
        1:+mod((2::vertices),11):/16))
}

struct cmgtest__graph scalar cmghadv__barbell(real scalar half)
{
    real colvector u, v, weight
    real scalar left, right, cursor, edges

    edges = half*(half-1)+1
    u = J(edges,1,.)
    v = J(edges,1,.)
    weight = J(edges,1,1)
    cursor = 0
    for (left=1; left<half; left++) {
        for (right=left+1; right<=half; right++) {
            cursor = cursor+1
            u[cursor] = left
            v[cursor] = right
        }
    }
    for (left=half+1; left<2*half; left++) {
        for (right=left+1; right<=2*half; right++) {
            cursor = cursor+1
            u[cursor] = left
            v[cursor] = right
        }
    }
    cursor = cursor+1
    u[cursor] = half
    v[cursor] = half+1
    weight[cursor] = 1/64
    assert(cursor == edges)
    return(cmghadv__graph(2*half,u,v,weight))
}

struct cmgtest__graph scalar cmghadv__irregular(real scalar vertices)
{
    real colvector u, v, weight
    real scalar edge, cursor, edges

    edges = 2*vertices-3
    u = J(edges,1,.)
    v = J(edges,1,.)
    weight = J(edges,1,.)
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
    assert(cursor == edges)
    return(cmghadv__graph(vertices,u,v,weight))
}

struct cmgtest__graph scalar cmghadv__expander(real scalar vertices)
{
    real colvector u, v, weight
    real rowvector offset
    real scalar one_offset, vertex, neighbor, cursor, edges, temporary

    offset = (1,7,31)
    edges = vertices*cols(offset)
    u = J(edges,1,.)
    v = J(edges,1,.)
    weight = J(edges,1,.)
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
            weight[cursor] = 1+mod(vertex+offset[one_offset],5)/16
        }
    }
    assert(rows(uniqrows(sort((u,v),(1,2)))) == edges)
    return(cmghadv__graph(vertices,u,v,weight))
}

void cmghadv__check(string scalar family, struct cmgtest__graph scalar graph,
    real scalar require_fallback)
{
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar first, second
    struct cmgtest__diag_result scalar diagnostics
    struct cmgtest__apply_result scalar applied
    struct cmgtest__level scalar level, second_level
    real colvector r, s
    real scalar index, left, right, denominator

    assert(graph.status == "CONVERGED")
    options = cmgtest__options_default()
    options.coarse_max = 8
    first = cmgtest__hierarchy_build(graph,options)
    second = cmgtest__hierarchy_build(graph,options)
    if (first.status != "CONVERGED") {
        errprintf("%s hierarchy failed: %s: %s\n",family,
            first.status,first.message)
        exit(3498)
    }
    assert(second.status == "CONVERGED")
    assert(first.n_level == second.n_level)
    assert(first.attempted_n_level == first.n_level)
    diagnostics = cmgtest__diagnostics(first)
    assert(diagnostics.status == "CONVERGED")
    assert(diagnostics.hierarchy_status == "CONVERGED")
    assert(diagnostics.attempted_n_level == first.n_level)
    assert(diagnostics.attempted_status[first.n_level] == "CONVERGED")
    if (require_fallback) {
        assert(any(diagnostics.attempted_method :==
            "NORMALIZED_HEAVY_FALLBACK"))
    }
    for (index=1; index<first.n_level; index++) {
        assert(first.attempted_level_table[index,6] >=
            options.min_reduction)
        assert(first.attempted_status[index] == "COMMITTED")
        level = *first.level[index]
        second_level = *second.level[index]
        assert(max(abs(level.aggregation-second_level.aggregation)) == 0)
    }

    r = sin((1::graph.n_vertex):/17)
    s = cos((1::graph.n_vertex):/29)
    r = r:-mean(r)
    s = s:-mean(s)
    applied = cmgtest__apply(first,(r,s))
    assert(applied.status == "CONVERGED")
    left = quadcross(r,applied.value[.,2])
    right = quadcross(applied.value[.,1],s)
    denominator = max((1,abs(left),abs(right)))
    assert(abs(left-right)/denominator <= 1e-11)
    assert(quadcross(r,applied.value[.,1]) > 0)
    assert(quadcross(s,applied.value[.,2]) > 0)
}

void cmghadv__typed_failures()
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__diag_result scalar diagnostics

    graph = cmghadv__star(65)
    options = cmgtest__options_default()
    options.coarse_max = 2
    options.min_reduction = 0.99
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "HIERARCHY_STALLED")
    assert(hierarchy.attempted_n_level == 1)
    diagnostics = cmgtest__diagnostics(hierarchy)
    assert(diagnostics.status == "CONVERGED")
    assert(diagnostics.hierarchy_status == "HIERARCHY_STALLED")
    assert(diagnostics.hierarchy_message == hierarchy.message)
    assert(diagnostics.attempted_status[1] == "HIERARCHY_STALLED")
    assert(diagnostics.attempted_method[1] ==
        "NORMALIZED_HEAVY_FALLBACK")
    assert(diagnostics.attempted_level_table[1,5] > 0)

    options = cmgtest__options_default()
    options.coarse_max = 2
    options.max_levels = 1
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "HIERARCHY_LEVEL_LIMIT")
    diagnostics = cmgtest__diagnostics(hierarchy)
    assert(diagnostics.status == "CONVERGED")
    assert(diagnostics.hierarchy_status == "HIERARCHY_LEVEL_LIMIT")
    assert(diagnostics.attempted_n_level == 1)
    assert(diagnostics.attempted_status[1] == "HIERARCHY_LEVEL_LIMIT")

    options.max_levels = 97
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "INVALID_OPTIONS")
    assert(hierarchy.attempted_n_level == 0)
}

void cmghadv__run()
{
    struct cmgtest__options scalar options

    assert(cmgtest__api_level() == 5)
    options = cmgtest__options_default()
    assert(options.max_levels == 96)
    options = cmgtest__options_resource(56*1024^3,6144,601)
    assert(options.coarse_max == 6144)
    options = cmgtest__options_resource(56*1024^3,6145,601)
    assert(options.coarse_max == 256)
    cmghadv__check("star",cmghadv__star(257),1)
    cmghadv__check("path",cmghadv__path(257),0)
    cmghadv__check("barbell",cmghadv__barbell(20),0)
    cmghadv__check("irregular",cmghadv__irregular(257),0)
    cmghadv__check("expander",cmghadv__expander(256),0)
    cmghadv__typed_failures()
}

cmghadv__run()
end

di as result "CMG ADVERSARIAL HIERARCHY TEST PASS"
exit 0
