*! kss_bc graph selector 0.2.0-dev 15aug2026
*! API 19 Optimization III numeric mode

version 18.0

mata:
mata set matastrict on
mata set matalnum off

real scalar kssbc_graph__api_level()
{
    return(19)
}

string scalar kssbc_graph__build_id()
{
    return("kss-bc-graph-api19-numopt2")
}

struct kssbc_graph__bridge_result
{
    string scalar status
    string scalar message
    real colvector deletion_id
    real scalar edges
}

struct kssbc_graph__bridge_result scalar kssbc_graph__bridges(
    real colvector worker,
    real colvector firm,
    real colvector deletion_id,
    real colvector active)
{
    struct kssbc_graph__bridge_result scalar out
    real colvector selected, edge_order, sorted_index, edge_code, edge_first
    real colvector edge_worker, edge_firm, edge_deletion, degree
    real colvector adjacency_start, adjacency_finish, cursor, neighbor
    real colvector adjacency_edge, discovery, low, dfs_parent, parent_edge
    real colvector next_arc, stack, bridge
    real scalar workers, firms, nodes, row, edge, node, other, arc
    real scalar clock, top, root, parent

    out.status = "INVALID_GRAPH_INPUT"
    out.message = "deletion-unit multigraph inputs are invalid"
    out.deletion_id = J(0,1,.)
    out.edges = 0
    if (rows(worker) == 0 | cols(worker) != 1 |
        rows(firm) != rows(worker) | cols(firm) != 1 |
        rows(deletion_id) != rows(worker) | cols(deletion_id) != 1 |
        rows(active) != rows(worker) | cols(active) != 1 |
        hasmissing(worker) | hasmissing(firm) | hasmissing(deletion_id) |
        hasmissing(active)) return(out)

    selected = selectindex(active :== 1)
    if (rows(selected) == 0) {
        out.status = "CONVERGED"
        out.message = "empty multigraph has no deletion-unit bridges"
        return(out)
    }
    workers = max(worker)
    firms = max(firm)
    nodes = workers+firms

    // One graph edge is one distinct deletion ID.  Sorting by deletion ID,
    // rather than worker-firm coordinate, preserves parallel deletion units.
    edge_order = order(deletion_id[selected],1)
    sorted_index = selected[edge_order]
    edge_code = J(rows(selected),1,1)
    for (row=2; row<=rows(selected); row++) {
        edge_code[row] = edge_code[row-1] +
            (deletion_id[sorted_index[row]] !=
             deletion_id[sorted_index[row-1]])
    }
    edge_first = sorted_index[panelsetup(edge_code,1)[.,1]]
    edge_worker = worker[edge_first]
    edge_firm = firm[edge_first]
    edge_deletion = deletion_id[edge_first]
    out.edges = rows(edge_first)

    degree = J(nodes,1,0)
    for (edge=1; edge<=out.edges; edge++) {
        degree[edge_worker[edge]] = degree[edge_worker[edge]]+1
        node = workers+edge_firm[edge]
        degree[node] = degree[node]+1
    }
    adjacency_start = J(nodes,1,.)
    cursor = J(nodes,1,.)
    row = 1
    for (node=1; node<=nodes; node++) {
        adjacency_start[node] = row
        cursor[node] = row
        row = row+degree[node]
    }
    adjacency_finish = adjacency_start+degree:-1
    neighbor = J(2*out.edges,1,.)
    adjacency_edge = J(2*out.edges,1,.)
    for (edge=1; edge<=out.edges; edge++) {
        node = edge_worker[edge]
        other = workers+edge_firm[edge]
        arc = cursor[node]
        neighbor[arc] = other
        adjacency_edge[arc] = edge
        cursor[node] = cursor[node]+1
        arc = cursor[other]
        neighbor[arc] = node
        adjacency_edge[arc] = edge
        cursor[other] = cursor[other]+1
    }

    // Iterative Tarjan search.  The parent edge, not the parent vertex, is
    // skipped.  A parallel edge therefore supplies the required back edge.
    discovery = J(nodes,1,0)
    low = J(nodes,1,0)
    dfs_parent = J(nodes,1,0)
    parent_edge = J(nodes,1,0)
    next_arc = adjacency_start
    stack = J(nodes,1,0)
    bridge = J(out.edges,1,0)
    clock = 0
    for (root=1; root<=nodes; root++) {
        if (degree[root] == 0 | discovery[root] != 0) continue
        top = 1
        stack[top] = root
        clock = clock+1
        discovery[root] = clock
        low[root] = clock
        while (top > 0) {
            node = stack[top]
            if (next_arc[node] <= adjacency_finish[node]) {
                arc = next_arc[node]
                next_arc[node] = next_arc[node]+1
                edge = adjacency_edge[arc]
                other = neighbor[arc]
                if (edge == parent_edge[node]) continue
                if (discovery[other] == 0) {
                    dfs_parent[other] = node
                    parent_edge[other] = edge
                    clock = clock+1
                    discovery[other] = clock
                    low[other] = clock
                    top = top+1
                    stack[top] = other
                }
                else low[node] = min((low[node],discovery[other]))
            }
            else {
                top = top-1
                parent = dfs_parent[node]
                if (parent != 0) {
                    if (low[node] > discovery[parent]) {
                        bridge[parent_edge[node]] = 1
                    }
                    low[parent] = min((low[parent],low[node]))
                }
            }
        }
    }
    out.deletion_id = select(edge_deletion,bridge :== 1)
    out.status = "CONVERGED"
    out.message = "deletion-unit multigraph bridge certificate computed"
    return(out)
}

real scalar kssbc_graph__deletion_count(
    real colvector deletion_id,
    real colvector active)
{
    real colvector selected

    selected = select(deletion_id,active :== 1)
    if (rows(selected) == 0) return(0)
    return(rows(uniqrows(sort(selected,1))))
}

void kssbc_graph__stata_prune(
    string scalar worker_name,
    string scalar firm_name,
    string scalar frequency_name,
    string scalar deletion_name,
    string scalar sample_name,
    string scalar deletion,
    string scalar keep_name,
    string scalar diagnostics_name,
    string scalar status_local,
    string scalar message_local)
{
    struct kssbc_component_result scalar component
    struct kssbc_articulation_result scalar articulation
    struct kssbc_graph__bridge_result scalar bridges, final_bridges
    real colvector worker, firm, frequency, deletion_id, sample, sample_index
    real colvector deletion_order, deletion_sorted, index, active
    real colvector worker_firms, remove_unit, diagnostics, legacy
    real matrix deletion_panel
    real scalar n, workers, group, begin, finish, row, removed
    real scalar initial_components, initial_component_rows, mover_input_rows
    real scalar graph_edges, articulation_removed, insufficient_removed
    real scalar pruning_iterations, maximum_components, retained_mass
    real scalar retained_rows, graph_seconds, retained_edges
    real scalar bridge_units_removed, bridge_rows_removed, bridge_iterations
    real scalar fixedpoint_iterations, iteration_bound

    // Observation deletion retains API 17 sample construction byte for byte.
    if (deletion == "observation") {
        kssbc__stata_prune_graph(worker_name,firm_name,frequency_name,
            deletion_name,sample_name,deletion,keep_name,diagnostics_name,
            status_local,message_local)
        if (st_local(status_local) == "CONVERGED") {
            legacy = st_matrix(diagnostics_name)
            diagnostics = legacy,(0,0,0,0,legacy[11],0)
            st_matrix(diagnostics_name,diagnostics)
        }
        return
    }

    sample = st_data(.,sample_name)
    sample_index = selectindex(sample :== 1)
    worker = st_data(sample_index,worker_name)
    firm = st_data(sample_index,firm_name)
    frequency = st_data(sample_index,frequency_name)
    deletion_id = st_data(sample_index,deletion_name)
    n = rows(worker)
    st_local(status_local,"INVALID_GRAPH_INPUT")
    st_local(message_local,"graph-pruning inputs are invalid")
    if (deletion != "match") {
        st_local(status_local,"UNSUPPORTED_DELETION")
        st_local(message_local,"deletion must be observation or match")
        return
    }
    if (n == 0 | rows(firm) != n | rows(frequency) != n |
        rows(deletion_id) != n | hasmissing(worker) | hasmissing(firm) |
        hasmissing(frequency) | hasmissing(deletion_id)) return
    if (min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0) return
    if (missing(kssbc__exact_physical_total(frequency))) {
        st_local(status_local,"PHYSICAL_TOTAL_LIMIT")
        st_local(message_local,
            "literal frequency total exceeds the exact binary64 integer range")
        return
    }
    timer_clear(89)
    timer_on(89)

    deletion_order = order(deletion_id,1)
    deletion_sorted = deletion_id[deletion_order]
    deletion_panel = panelsetup(deletion_sorted,1)
    for (group=1; group<=rows(deletion_panel); group++) {
        begin = deletion_panel[group,1]
        finish = deletion_panel[group,2]
        index = deletion_order[|begin\finish|]
        if (min(worker[index]) != max(worker[index]) |
            min(firm[index]) != max(firm[index])) {
            st_local(status_local,"CROSS_COORDINATE_MATCH")
            st_local(message_local,
                "each deletion ID must remain within one worker-firm coordinate")
            return
        }
    }

    workers = max(worker)
    active = J(n,1,1)
    component = kssbc__largest_component(worker,firm,frequency,active)
    if (component.ambiguous) {
        st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
        st_local(message_local,
            "multiple connected components tie for largest-component selection")
        return
    }
    if (sum(component.keep) == 0) {
        st_local(status_local,"NO_LEAVEOUT_COMPONENT")
        st_local(message_local,"no connected worker-firm component remains")
        return
    }
    active = active:*component.keep
    initial_components = component.components
    initial_component_rows = sum(active)

    worker_firms = kssbc__worker_firm_counts(worker,firm,active)
    active = active :* (worker_firms[worker] :> 1)
    mover_input_rows = sum(active)
    if (mover_input_rows == 0) {
        st_local(status_local,"NO_MOVER_SAMPLE")
        st_local(message_local,
            "no observations remain in the mover target population")
        return
    }

    component = kssbc__largest_component(worker,firm,frequency,active)
    if (component.ambiguous) {
        st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
        st_local(message_local,
            "multiple mover components tie for largest-component selection")
        return
    }
    active = active:*component.keep
    graph_edges = kssbc_graph__deletion_count(deletion_id,active)
    maximum_components = max((initial_components,component.components))
    articulation_removed = 0
    insufficient_removed = 0
    pruning_iterations = 0
    bridge_units_removed = 0
    bridge_rows_removed = 0
    bridge_iterations = 0
    fixedpoint_iterations = 0
    iteration_bound = workers+graph_edges+1

    while (sum(active) > 0) {
        component = kssbc__largest_component(worker,firm,frequency,active)
        if (component.ambiguous) {
            st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
            st_local(message_local,
                "fixed-point pruning creates tied largest components")
            return
        }
        maximum_components = max((maximum_components,component.components))
        active = active:*component.keep
        if (sum(active) == 0) break

        worker_firms = kssbc__worker_firm_counts(worker,firm,active)
        removed = sum(worker_firms :== 1)
        if (removed > 0) {
            active = active :* (worker_firms[worker] :> 1)
            insufficient_removed = insufficient_removed+removed
            pruning_iterations = pruning_iterations+1
            fixedpoint_iterations = fixedpoint_iterations+1
            if (fixedpoint_iterations > iteration_bound) break
            continue
        }

        articulation = kssbc__worker_articulations(worker,firm,active)
        if (articulation.count > 0) {
            active = active :* (articulation.bad_worker[worker] :== 0)
            articulation_removed = articulation_removed+articulation.count
            pruning_iterations = pruning_iterations+1
            fixedpoint_iterations = fixedpoint_iterations+1
            if (fixedpoint_iterations > iteration_bound) break
            continue
        }

        bridges = kssbc_graph__bridges(worker,firm,deletion_id,active)
        if (bridges.status != "CONVERGED") {
            st_local(status_local,bridges.status)
            st_local(message_local,bridges.message)
            return
        }
        if (rows(bridges.deletion_id) == 0) break
        remove_unit = J(max(deletion_id),1,0)
        remove_unit[bridges.deletion_id] = J(rows(bridges.deletion_id),1,1)
        bridge_units_removed = bridge_units_removed+rows(bridges.deletion_id)
        bridge_rows_removed = bridge_rows_removed+
            sum(active :& (remove_unit[deletion_id] :== 1))
        active = active :* (remove_unit[deletion_id] :== 0)
        bridge_iterations = bridge_iterations+1
        fixedpoint_iterations = fixedpoint_iterations+1
        if (fixedpoint_iterations > iteration_bound) break
    }

    if (fixedpoint_iterations > iteration_bound) {
        st_local(status_local,"GRAPH_ITERATION_FAILED")
        st_local(message_local,
            "sample fixed-point pruning exceeded its finite iteration bound")
        return
    }
    if (sum(active) == 0) {
        st_local(status_local,"NO_LEAVEOUT_COMPONENT")
        st_local(message_local,
            "no component remains after fixed-point graph pruning")
        return
    }
    component = kssbc__largest_component(worker,firm,frequency,active)
    if (component.ambiguous) {
        st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
        st_local(message_local,
            "final graph pruning creates tied largest components")
        return
    }
    active = active:*component.keep
    maximum_components = max((maximum_components,component.components))
    final_bridges = kssbc_graph__bridges(worker,firm,deletion_id,active)
    if (final_bridges.status != "CONVERGED" |
        rows(final_bridges.deletion_id) != 0) {
        st_local(status_local,"GRAPH_BRIDGE_CERTIFICATE_FAILED")
        st_local(message_local,
            "final retained deletion-unit multigraph still contains a bridge")
        return
    }

    st_store(sample_index,keep_name,active)
    retained_rows = sum(active)
    retained_mass = sum(frequency:*active)
    retained_edges = kssbc_graph__deletion_count(deletion_id,active)
    timer_off(89)
    graph_seconds = kssbc__timer_seconds(89)
    diagnostics = (n,retained_rows,sum(frequency),retained_mass,
        graph_edges,articulation_removed,maximum_components,
        mover_input_rows,initial_component_rows,insufficient_removed,
        pruning_iterations,graph_seconds,retained_edges,
        bridge_units_removed,bridge_rows_removed,bridge_iterations,
        fixedpoint_iterations,0)
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,"CONVERGED")
    st_local(message_local,
        "deletion-unit multigraph fixed point selected and certified")
}

end
