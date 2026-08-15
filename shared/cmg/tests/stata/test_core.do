version 18.0
clear all
set more off
set varabbrev off

args repository_root
if strtrim(`"`repository_root'"') == "" {
    di as error "usage: do test_core.do repository_root"
    exit 198
}

mata: mata clear
do `"`repository_root'/shared/cmg/generated/cmg_test.mata"'

mata:
void cmgtest__assert_close(real matrix actual, real matrix expected,
    real scalar tolerance)
{
    assert(rows(actual) == rows(expected))
    assert(cols(actual) == cols(expected))
    if (rows(actual)*cols(actual) > 0) {
        assert(max(abs(actual:-expected)) <= tolerance)
    }
}

void cmgtest__assert_relative(real matrix actual, real matrix expected,
    real scalar tolerance)
{
    real scalar scale

    assert(rows(actual) == rows(expected))
    assert(cols(actual) == cols(expected))
    if (rows(actual)*cols(actual) > 0) {
        scale = max((1,max(abs(expected))))
        assert(max(abs(actual:-expected))/scale <= tolerance)
    }
}

void cmgtest__test_hybrid()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    real colvector worker, firm, weight
    real matrix schur, hybrid

    // Includes duplicate cells and workers of degrees one through four.
    worker = (1\1\2\2\2\3\3\3\3\4\4)
    firm =   (1\1\1\2\3\1\2\3\4\2\4)
    weight = (1\2\2\3\5\1\2\4\8\7\11)
    cells = cmgtest__cells_prepare(worker,firm,weight,(101\102\103\104),
        (201\202\203\204))
    assert(cells.status == "CONVERGED")
    assert(cells.n_cell == 10)
    graph = cmgtest__hybrid_build(cells)
    assert(graph.status == "CONVERGED")
    assert(graph.n_auxiliary == 1)
    assert(graph.n_edge <= cells.n_cell)
    assert(graph.n_vertex <= cells.n_firm+floor(cells.n_cell/4))
    schur = cmgtest__dense_schur(cells)
    hybrid = cmgtest__dense_hybrid_schur(graph)
    cmgtest__assert_close(hybrid,schur,2e-13)
    cmgtest__assert_close(rowsum(schur),J(cells.n_firm,1,0),2e-13)
}

void cmgtest__test_degree_one()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph

    cells = cmgtest__cells_prepare((1\2),(1\2),(2\3),(10\20),(30\40))
    assert(cells.status == "CONVERGED")
    graph = cmgtest__hybrid_build(cells)
    assert(graph.status == "CONVERGED")
    assert(graph.n_edge == 0)
    assert(graph.n_auxiliary == 0)
    cmgtest__assert_close(cmgtest__dense_schur(cells),J(2,2,0),0)
}

void cmgtest__test_chunked_hub_action()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    real matrix argument, actual, expected, contribution
    real colvector firm, weight
    real scalar vertices, auxiliary, column

    // Two columns and a 1 MiB scratch cap force the exceptional auxiliary-hub
    // group through more than one arc chunk.
    vertices = 20000
    firm = (1::vertices)
    weight = 1:+mod((1::vertices),7)
    cells = cmgtest__cells_prepare(J(vertices,1,1),firm,weight,1,firm)
    assert(cells.status == "CONVERGED")
    graph = cmgtest__hybrid_build(cells)
    assert(graph.status == "CONVERGED")
    assert(graph.n_edge == vertices)
    assert(graph.n_auxiliary == 1)
    assert(graph.n_vertex == vertices+1)
    auxiliary = graph.n_vertex

    argument = ((1::graph.n_vertex),sin((1::graph.n_vertex):/37))
    contribution = graph.weight:*
        (argument[graph.u,.]-argument[graph.v,.])
    expected = J(graph.n_vertex,cols(argument),0)
    expected[graph.u,.] = contribution
    expected[auxiliary,.] = -colsum(contribution)
    actual = cmgtest__graph_action(graph,argument,1024^2)
    cmgtest__assert_relative(actual,expected,2e-12)

    // A wider RHS block follows the same bounded row-chunk path; the core no
    // longer imposes an unrelated eight-column interpreter loop.
    argument = J(graph.n_vertex,19,.)
    for (column=1; column<=cols(argument); column++) {
        argument[.,column] = sin((1::graph.n_vertex):/(31+column))
    }
    contribution = graph.weight:*
        (argument[graph.u,.]-argument[graph.v,.])
    expected = J(graph.n_vertex,cols(argument),0)
    expected[graph.u,.] = contribution
    expected[auxiliary,.] = -colsum(contribution)
    actual = cmgtest__graph_action(graph,argument,1024^2)
    cmgtest__assert_relative(actual,expected,2e-12)
}

struct cmgtest__graph scalar cmgtest__path_graph(real scalar vertices)
{
    struct cmgtest__cells scalar cells
    real colvector worker, firm, weight
    real scalar edge

    worker = J(2*(vertices-1),1,.)
    firm = J(2*(vertices-1),1,.)
    weight = J(2*(vertices-1),1,1)
    for (edge=1; edge<vertices; edge++) {
        worker[2*edge-1] = edge
        worker[2*edge] = edge
        firm[2*edge-1] = edge
        firm[2*edge] = edge+1
    }
    cells = cmgtest__cells_prepare(worker,firm,weight,
        (1001::(1000+vertices-1)),(2001::(2000+vertices)))
    assert(cells.status == "CONVERGED")
    return(cmgtest__hybrid_build(cells))
}

struct cmgtest__cells scalar cmgtest__path_cells(real scalar vertices)
{
    real colvector worker, firm, weight
    real scalar edge

    worker = J(2*(vertices-1),1,.)
    firm = J(2*(vertices-1),1,.)
    weight = J(2*(vertices-1),1,1)
    for (edge=1; edge<vertices; edge++) {
        worker[2*edge-1] = edge
        worker[2*edge] = edge
        firm[2*edge-1] = edge
        firm[2*edge] = edge+1
    }
    return(cmgtest__cells_prepare(worker,firm,weight,
        (1::(vertices-1)),(1::vertices)))
}

void cmgtest__test_cycle()
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options, resource_options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__apply_result scalar applied, first, second, combined
    struct cmgtest__level scalar level, child
    real matrix dense, argument, action, cycle, prolongation
    real colvector r, s, component
    real scalar index, vertex

    graph = cmgtest__path_graph(24)
    component = cmgtest__components(graph)
    if (rows(component) != graph.n_vertex) {
        errprintf("component failure before hierarchy: graph=%s V=%g E=%g rows=%g\n",
            graph.status,graph.n_vertex,graph.n_edge,rows(component))
        exit(3498)
    }
    dense = cmgtest__dense_laplacian(graph)
    argument = (1::graph.n_vertex),(graph.n_vertex::1)
    action = cmgtest__graph_action(graph,argument,1024^2)
    cmgtest__assert_close(action,dense*argument,2e-13)

    options = cmgtest__options_default()
    resource_options = cmgtest__options_resource(56*1024^3,10000,16)
    assert(resource_options.coarse_max == 256)
    assert(resource_options.action_scratch_bytes == 1024*1024^2)
    resource_options = cmgtest__options_resource(56*1024^3,1000,16)
    assert(resource_options.coarse_max == 128)
    // Repeated-RHS KSS systems may spend bounded memory on an exact terminal
    // factor instead of attempting a hierarchy that cannot meet the fixed
    // reduction gate.  The 1,285-vertex case needs only about 13 MiB.
    resource_options = cmgtest__options_resource(56*1024^3,1285,601)
    assert(resource_options.coarse_max == 1285)
    assert(cmgtest__options_valid(resource_options))
    resource_options = cmgtest__options_resource(8*1024^3,1285,601)
    assert(resource_options.coarse_max == 128)
    // The registered all-mover KSS graph can add one hybrid auxiliary per
    // worker, so API 4 admits the graph-theoretic upper envelope while the
    // factor still fits the explicit dense budget.
    resource_options = cmgtest__options_resource(56*1024^3,1537,601)
    assert(resource_options.coarse_max == 1537)
    resource_options = cmgtest__options_resource(56*1024^3,6144,601)
    assert(resource_options.coarse_max == 6144)
    assert(cmgtest__options_valid(resource_options))
    resource_options = cmgtest__options_resource(56*1024^3,6145,601)
    assert(resource_options.coarse_max == 256)
    options.coarse_max = 3
    hierarchy = cmgtest__hierarchy_build(graph,options)
    if (hierarchy.status != "CONVERGED") {
        errprintf("hierarchy failure: %s: %s\n",
            hierarchy.status,hierarchy.message)
        exit(3498)
    }
    assert(hierarchy.n_level > 1)
    assert(hierarchy.edge_complexity <= options.max_edge_complexity)
    assert(hierarchy.vertex_complexity <= options.max_vertex_complexity)
    for (index=1; index<hierarchy.n_level; index++) {
        level = *hierarchy.level[index]
        child = *hierarchy.level[index+1]
        prolongation = J(level.graph.n_vertex,level.n_coarse,0)
        for (vertex=1; vertex<=level.graph.n_vertex; vertex++) {
            prolongation[vertex,level.aggregation[vertex]] = 1
        }
        cmgtest__assert_close(
            prolongation'*cmgtest__dense_laplacian(level.graph)*prolongation,
            cmgtest__dense_laplacian(child.graph),2e-13)
        assert(level.n_component == child.n_component)
    }

    applied = cmgtest__apply(hierarchy,I(graph.n_vertex))
    assert(applied.status == "CONVERGED")
    cycle = applied.value
    cmgtest__assert_close(cycle,cycle',1e-12)
    cmgtest__assert_close(rowsum(cycle),J(graph.n_vertex,1,0),1e-12)
    r = (1::graph.n_vertex)
    r = r:-mean(r)
    s = cos((1::graph.n_vertex))
    s = s:-mean(s)
    assert(quadcross(r,cycle*r) > 0)
    assert(quadcross(s,cycle*s) > 0)
    first = cmgtest__apply(hierarchy,r)
    second = cmgtest__apply(hierarchy,s)
    combined = cmgtest__apply(hierarchy,1.25:*r:-0.75:*s)
    assert(first.status == "CONVERGED")
    assert(second.status == "CONVERGED")
    assert(combined.status == "CONVERGED")
    cmgtest__assert_close(combined.value,
        1.25:*first.value:-0.75:*second.value,5e-13)
}

void cmgtest__test_pullbacks()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__apply_result scalar kss, ppml
    real matrix lap, projector, expected
    real rowvector weight_scale
    real scalar scale_index

    graph = cmgtest__path_graph(3)
    options = cmgtest__options_default()
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "CONVERGED")
    lap = cmgtest__dense_laplacian(graph)
    projector = I(3):-J(3,3,1/3)
    kss = cmgtest__apply_kss(hierarchy,I(3))
    assert(kss.status == "CONVERGED")
    cmgtest__assert_close(kss.value,kss.value',2e-13)
    cmgtest__assert_close(kss.value*lap,projector,2e-12)

    ppml = cmgtest__apply_ppml(hierarchy,I(2),3)
    assert(ppml.status == "CONVERGED")
    expected = invsym(lap[(1::2),(1::2)])
    cmgtest__assert_close(ppml.value,expected,2e-12)
    cmgtest__assert_close(ppml.value,ppml.value',2e-13)

    weight_scale = (1e-200,1e200)
    for (scale_index=1; scale_index<=cols(weight_scale); scale_index++) {
        cells = cmgtest__cells_prepare((1\1\2\2),(1\2\2\3),
            J(4,1,weight_scale[scale_index]),(1\2),(1\2\3))
        assert(cells.status == "CONVERGED")
        graph = cmgtest__hybrid_build(cells)
        hierarchy = cmgtest__hierarchy_build(graph,options)
        assert(hierarchy.status == "CONVERGED")
        kss = cmgtest__apply_kss(hierarchy,I(3))
        assert(kss.status == "CONVERGED")
        lap = cmgtest__dense_laplacian(graph)
        cmgtest__assert_close(kss.value*(graph.weight_scale:*lap),
            projector,3e-12)
    }
}

void cmgtest__test_routing()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__options scalar options
    struct cmgtest__preflight_result scalar preflight
    struct cmgtest__route_result scalar route
    real matrix pilot, second
    string rowvector converged, capped
    real scalar one_component, column

    assert(cmgtest__park_miller_step(1) == 16807)
    assert(cmgtest__park_miller_step(16807) == 282475249)
    pilot = cmgtest__pilot_rhs((40\10\30\20),(1\1\2\2))
    second = cmgtest__pilot_rhs((40\10\30\20),(1\1\2\2))
    cmgtest__assert_close(pilot,second,0)
    assert(rows(pilot) == 4 & cols(pilot) == 4)
    for (one_component=1; one_component<=2; one_component++) {
        for (column=1; column<=4; column++) {
            assert(sum(select(pilot[.,column],
                (1\1\2\2) :== one_component)) == 0)
        }
    }
    for (column=1; column<=4; column++) {
        assert(max(abs(pilot[.,column])) == 1)
    }

    options = cmgtest__options_default()
    cells = cmgtest__path_cells(300)
    preflight = cmgtest__preflight(cells,200,1,8*1024^3,options)
    assert(preflight.status == "CONVERGED")
    assert(preflight.route == "PILOT_DIAGONAL")
    preflight = cmgtest__preflight(cells,7,1,8*1024^3,options)
    assert(preflight.route == "DIAGONAL")

    converged = ("CONVERGED","CONVERGED","CONVERGED","CONVERGED")
    capped = ("ITERATION_LIMIT","ITERATION_LIMIT",
              "ITERATION_LIMIT","ITERATION_LIMIT")
    route = cmgtest__route_decide(200,"AUTO",converged,(10,11,12,13),
        J(1,0,""),J(1,0,.),100,.)
    assert(route.status == "CONVERGED" & route.route == "DIAGONAL")
    route = cmgtest__route_decide(200,"AUTO",capped,(32,32,32,32),
        converged,(7,7,7,7),100,75)
    assert(route.status == "CONVERGED" & route.route == "CMG")
    route = cmgtest__route_decide(200,"AUTO",capped,(32,32,32,32),
        converged,(7,7,7,7),100,90)
    assert(route.route == "DIAGONAL")
}

void cmgtest__test_batch_symmetry()
{
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy, second_hierarchy
    struct cmgtest__workspace scalar workspace, too_small
    struct cmgtest__diag_result scalar diagnostics
    struct cmgtest__apply_result scalar full, part, tiny, huge, baseline
    struct cmgtest__apply_result scalar pairs, workspace_full
    struct cmgtest__level scalar level, second_level
    real matrix rhs, partitioned, left_rhs, right_rhs
    real colvector one_left, one_right, applied_left, applied_right
    real rowvector widths
    real scalar column, width_index, cursor, stop, level_index
    real scalar left, right, denominator, curvature

    graph = cmgtest__path_graph(300)
    options = cmgtest__options_default()
    options.coarse_max = 8
    hierarchy = cmgtest__hierarchy_build(graph,options)
    second_hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "CONVERGED")
    assert(second_hierarchy.status == "CONVERGED")
    diagnostics = cmgtest__diagnostics(hierarchy)
    assert(diagnostics.status == "CONVERGED")
    assert(diagnostics.n_level == hierarchy.n_level)
    assert(diagnostics.fine_vertices == graph.n_vertex)
    assert(diagnostics.fine_edges == graph.n_edge)
    assert(rows(diagnostics.level_table) == hierarchy.n_level)
    workspace = cmgtest__workspace_init(hierarchy,32,1024^3)
    assert(workspace.status == "CONVERGED")
    assert(workspace.batch_capacity == 32)
    assert(workspace.allocated_bytes > 0)
    too_small = cmgtest__workspace_init(hierarchy,32,1024)
    assert(too_small.status == "WORKSPACE_MEMORY_LIMIT")
    assert(hierarchy.n_level == second_hierarchy.n_level)
    for (level_index=1; level_index<=hierarchy.n_level; level_index++) {
        level = *hierarchy.level[level_index]
        second_level = *second_hierarchy.level[level_index]
        cmgtest__assert_close(level.graph.u,second_level.graph.u,0)
        cmgtest__assert_close(level.graph.v,second_level.graph.v,0)
        cmgtest__assert_close(level.graph.weight,second_level.graph.weight,0)
        cmgtest__assert_close(level.aggregation,second_level.aggregation,0)
    }

    rhs = J(graph.n_vertex,32,.)
    for (column=1; column<=cols(rhs); column++) {
        rhs[.,column] = sin((1::graph.n_vertex):*
            (column/(cols(rhs)+1)))
        rhs[.,column] = rhs[.,column]:-mean(rhs[.,column])
    }
    full = cmgtest__apply(hierarchy,rhs)
    assert(full.status == "CONVERGED")
    workspace_full = cmgtest__workspace_apply(hierarchy,workspace,rhs)
    assert(workspace_full.status == "CONVERGED")
    cmgtest__assert_close(workspace_full.value,full.value,2e-12)
    widths = (1,2,3,4,7,8,9,16,31,32)
    for (width_index=1; width_index<=cols(widths); width_index++) {
        partitioned = J(rows(rhs),cols(rhs),.)
        cursor = 1
        while (cursor <= cols(rhs)) {
            stop = min((cols(rhs),cursor+widths[width_index]-1))
            part = cmgtest__workspace_apply(
                hierarchy,workspace,rhs[.,(cursor::stop)])
            assert(part.status == "CONVERGED")
            partitioned[.,(cursor::stop)] = part.value
            cursor = stop+1
        }
        cmgtest__assert_close(partitioned,full.value,2e-12)
    }

    baseline = cmgtest__apply(hierarchy,rhs[.,1])
    tiny = cmgtest__apply(hierarchy,1e-200:*rhs[.,1])
    huge = cmgtest__apply(hierarchy,1e200:*rhs[.,1])
    assert(baseline.status == "CONVERGED")
    assert(tiny.status == "CONVERGED")
    assert(huge.status == "CONVERGED")
    cmgtest__assert_relative(tiny.value:/1e-200,baseline.value,2e-12)
    cmgtest__assert_relative(huge.value:/1e200,baseline.value,2e-12)

    left_rhs = J(graph.n_vertex,64,.)
    right_rhs = J(graph.n_vertex,64,.)
    for (column=1; column<=64; column++) {
        left_rhs[.,column] = sin((1::graph.n_vertex):*
            ((column+0.25)/67))
        right_rhs[.,column] = cos((1::graph.n_vertex):*
            ((2*column+0.5)/131))
        left_rhs[.,column] = left_rhs[.,column]:-
            mean(left_rhs[.,column])
        right_rhs[.,column] = right_rhs[.,column]:-
            mean(right_rhs[.,column])
    }
    pairs = cmgtest__apply(hierarchy,(left_rhs,right_rhs))
    assert(pairs.status == "CONVERGED")
    for (column=1; column<=64; column++) {
        one_left = left_rhs[.,column]
        one_right = right_rhs[.,column]
        applied_left = pairs.value[.,column]
        applied_right = pairs.value[.,64+column]
        left = quadcross(one_left,applied_right)
        right = quadcross(applied_left,one_right)
        denominator = max((1,
            sqrt(quadcross(one_left,one_left))*
                sqrt(quadcross(applied_right,applied_right)),
            sqrt(quadcross(applied_left,applied_left))*
                sqrt(quadcross(one_right,one_right))))
        assert(abs(left-right)/denominator <= 1e-11)
        curvature = quadcross(one_left,applied_left)
        denominator = sqrt(quadcross(one_left,one_left))*
            sqrt(quadcross(applied_left,applied_left))
        assert(curvature >= 1e-14*denominator)
    }

    rhs[1,1] = .
    part = cmgtest__apply(hierarchy,rhs)
    assert(part.status == "INVALID_INPUT")
}

void cmgtest__test_component_relabel()
{
    struct cmgtest__cells scalar original_cells, relabeled_cells
    struct cmgtest__graph scalar graph, original_graph, relabeled_graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy, original_hierarchy
    struct cmgtest__hierarchy scalar relabeled_hierarchy
    struct cmgtest__apply_result scalar cycle, original_cycle, relabeled_cycle
    real colvector worker, firm, relabeled_worker, relabeled_firm, reverse
    real matrix pilot, relabeled_pilot
    real scalar edge, vertices

    graph = cmgtest__empty_graph()
    graph.n_firm = 6
    graph.n_vertex = 6
    graph.n_edge = 4
    graph.n_auxiliary = 0
    graph.weight_scale = 1
    graph.u = (1\2\4\5)
    graph.v = (2\3\5\6)
    graph.weight = (1\2\3\4)
    graph.key_primary = (1::6)
    graph.key_type = J(6,1,0)
    graph.auxiliary_worker = J(0,1,.)
    graph.status = "CONVERGED"
    graph = cmgtest__graph_finalize(graph)
    options = cmgtest__options_default()
    options.coarse_max = 2
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "CONVERGED")
    cycle = cmgtest__apply(hierarchy,I(6))
    assert(cycle.status == "CONVERGED")
    cmgtest__assert_close(cycle.value,cycle.value',2e-12)
    cmgtest__assert_close(cycle.value[(1::3),(4::6)],J(3,3,0),0)
    cmgtest__assert_close(cycle.value[(4::6),(1::3)],J(3,3,0),0)

    vertices = 24
    worker = J(2*(vertices-1),1,.)
    firm = J(2*(vertices-1),1,.)
    for (edge=1; edge<vertices; edge++) {
        worker[2*edge-1] = edge
        worker[2*edge] = edge
        firm[2*edge-1] = edge
        firm[2*edge] = edge+1
    }
    original_cells = cmgtest__cells_prepare(worker,firm,J(rows(worker),1,1),
        (1::(vertices-1)),(1::vertices))
    relabeled_worker = J(rows(worker),1,vertices):-worker
    relabeled_firm = J(rows(firm),1,vertices+1):-firm
    relabeled_cells = cmgtest__cells_prepare(
        relabeled_worker,relabeled_firm,J(rows(worker),1,1),
        ((vertices-1)::1),(vertices::1))
    original_graph = cmgtest__hybrid_build(original_cells)
    relabeled_graph = cmgtest__hybrid_build(relabeled_cells)
    options.coarse_max = 3
    original_hierarchy = cmgtest__hierarchy_build(original_graph,options)
    relabeled_hierarchy = cmgtest__hierarchy_build(relabeled_graph,options)
    assert(original_hierarchy.status == "CONVERGED")
    assert(relabeled_hierarchy.status == "CONVERGED")
    original_cycle = cmgtest__apply_kss(original_hierarchy,I(vertices))
    relabeled_cycle = cmgtest__apply_kss(relabeled_hierarchy,I(vertices))
    assert(original_cycle.status == "CONVERGED")
    assert(relabeled_cycle.status == "CONVERGED")
    reverse = (vertices::1)
    cmgtest__assert_close(
        relabeled_cycle.value[reverse,reverse],original_cycle.value,2e-12)

    pilot = cmgtest__pilot_rhs((1::vertices),J(vertices,1,1))
    relabeled_pilot = cmgtest__pilot_rhs((vertices::1),J(vertices,1,1))
    cmgtest__assert_close(relabeled_pilot[reverse,.],pilot,0)
}

void cmgtest__test_failures()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__apply_result scalar applied
    struct cmgtest__preflight_result scalar preflight
    real colvector worker, firm

    worker = (1\1\2\2\3\3\4\4\5)
    firm =   (1\2\2\3\3\4\4\5\6)
    cells = cmgtest__cells_prepare(worker,firm,J(9,1,1),
        (1\2\3\4\5),(1\2\3\4\5\6))
    graph = cmgtest__hybrid_build(cells)
    options = cmgtest__options_default()
    options.coarse_max = 2
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "CONVERGED")
    applied = cmgtest__apply(hierarchy,(0\0\0\0\0\1))
    assert(applied.status == "INCOMPATIBLE_SINGLETON")
    applied = cmgtest__apply(hierarchy,J(6,1,0))
    assert(applied.status == "CONVERGED")
    applied = cmgtest__apply_ppml(hierarchy,J(5,1,0),6)
    assert(applied.status == "PPML_REQUIRES_CONNECTED_GRAPH")

    options = cmgtest__options_default()
    options.dense_factor_bytes = 1024
    graph = cmgtest__path_graph(20)
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "DENSE_FACTOR_MEMORY_LIMIT")

    options = cmgtest__options_default()
    options.max_edge_complexity = 1
    options.coarse_max = 3
    graph = cmgtest__path_graph(24)
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "HIERARCHY_EDGE_LIMIT")

    options = cmgtest__options_default()
    cells = cmgtest__path_cells(10000)
    preflight = cmgtest__preflight(cells,200,1,1024^2,options)
    assert(preflight.status == "CONVERGED")
    assert(preflight.route == "DIAGONAL")
}

cmgtest__test_hybrid()
cmgtest__test_degree_one()
cmgtest__test_chunked_hub_action()
cmgtest__test_cycle()
cmgtest__test_pullbacks()
cmgtest__test_failures()
cmgtest__test_batch_symmetry()
cmgtest__test_component_relabel()
end

set seed 20260814
local rng_before `"`c(rngstate)'"'
mata: cmgtest__test_routing()
local rng_after `"`c(rngstate)'"'
assert `"`rng_before'"' == `"`rng_after'"'

di as result "CMG MATA CORE TEST PASS"
exit 0
