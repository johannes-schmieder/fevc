version 18.0

mata:
mata set matastrict on
mata set matalnum on

struct vckss_sep__bridge_result
{
    real colvector bad_row
    real scalar count
}

struct vckss_sep__bridge_result scalar vckss_sep__match_bridges(
    real colvector worker,
    real colvector firm,
    real colvector active)
{
    struct vckss_sep__bridge_result scalar out
    real colvector selected, edge_order, sorted_index, edge_code, edge_first
    real colvector edge_worker, edge_firm, degree, adjacency_start
    real colvector adjacency_finish, cursor, neighbor, adjacency_edge
    real colvector discovery, low, dfs_parent, parent_edge, next_arc
    real colvector stack, bridge
    real scalar workers, firms, nodes, edges, row, edge, node, other
    real scalar arc, clock, top, root, parent

    out.bad_row = J(rows(worker),1,0)
    out.count = 0
    selected = selectindex(active :== 1)
    if (rows(selected) == 0) return(out)
    workers = max(worker)
    firms = max(firm)
    nodes = workers+firms

    edge_order = order((worker[selected],firm[selected]),(1,2))
    sorted_index = selected[edge_order]
    edge_code = J(rows(selected),1,1)
    for (row=2; row<=rows(selected); row++) {
        edge_code[row] = edge_code[row-1] +
            (worker[sorted_index[row]] != worker[sorted_index[row-1]] |
            firm[sorted_index[row]] != firm[sorted_index[row-1]])
    }
    edge_first = sorted_index[panelsetup(edge_code,1)[.,1]]
    edge_worker = worker[edge_first]
    edge_firm = firm[edge_first]
    edges = rows(edge_first)

    degree = J(nodes,1,0)
    for (edge=1; edge<=edges; edge++) {
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
    neighbor = J(2*edges,1,.)
    adjacency_edge = J(2*edges,1,.)
    for (edge=1; edge<=edges; edge++) {
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

    discovery = J(nodes,1,0)
    low = J(nodes,1,0)
    dfs_parent = J(nodes,1,0)
    parent_edge = J(nodes,1,0)
    next_arc = adjacency_start
    stack = J(nodes,1,0)
    bridge = J(edges,1,0)
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
    for (row=1; row<=rows(sorted_index); row++) {
        if (bridge[edge_code[row]]) out.bad_row[sorted_index[row]] = 1
    }
    out.count = sum(bridge)
    return(out)
}

void vckss_sep__stata_prune_bridges(
    string scalar worker_name,
    string scalar firm_name,
    string scalar frequency_name,
    string scalar sample_name,
    string scalar keep_name,
    string scalar diagnostics_name,
    string scalar status_local,
    string scalar message_local)
{
    struct vckss_sep__bridge_result scalar bridges
    struct vckss_component_result scalar component
    real colvector sample, sample_index, worker, firm, frequency, active
    real matrix diagnostics

    sample = st_data(.,sample_name)
    sample_index = selectindex(sample :== 1)
    worker = st_data(sample_index,worker_name)
    firm = st_data(sample_index,firm_name)
    frequency = st_data(sample_index,frequency_name)
    st_local(status_local,"INVALID_GRAPH_INPUT")
    st_local(message_local,"match-bridge inputs are invalid")
    if (rows(worker) == 0 | rows(firm) != rows(worker) |
        rows(frequency) != rows(worker) | hasmissing(worker) |
        hasmissing(firm) | hasmissing(frequency) | min(frequency) <= 0) return

    active = J(rows(worker),1,1)
    bridges = vckss_sep__match_bridges(worker,firm,active)
    if (bridges.count > 0) {
        active = active :* (bridges.bad_row :== 0)
        component = vckss__largest_component(worker,firm,frequency,active)
        if (component.ambiguous) {
            st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
            st_local(message_local,
                "bridge removal creates tied largest components")
            return
        }
        if (sum(component.keep) == 0) {
            st_local(status_local,"NO_BRIDGE_FREE_COMPONENT")
            st_local(message_local,
                "no component remains after match-bridge removal")
            return
        }
        active = active:*component.keep
    }
    st_store(sample_index,keep_name,active)
    diagnostics = (bridges.count,sum(active),sum(frequency:*active))
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,"CONVERGED")
    st_local(message_local,"match-bridge audit completed")
}

end
