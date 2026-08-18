*! KSS-STREAMLINE-1 deterministic diagnostic fixtures 17aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

struct kssbc_scale__diagnostic
{
    string scalar status
    string scalar message
    real scalar rows
    real scalar physical
    real scalar workers
    real scalar firms
    real scalar cells
    real scalar deletion_units
    real scalar components
    real scalar bridge_units
    real scalar minimum_weighted_degree
    real scalar maximum_weighted_degree
}

real scalar kssbc_scale_fixture__api_level()
{
    return(2)
}

string scalar kssbc_scale_fixture__build_id()
{
    return("kss-scale-fixtures-api2-replicated-blocks")
}

real scalar kssbc_scale__pair_count(
    string scalar design,
    real scalar copies)
{
    if (design == "replicated_blocks" | design == "well_connected") {
        return(copies*(copies-1)/2)
    }
    if (design == "ring") return(copies == 2 ? 1 : copies)
    return(.)
}

real matrix kssbc_scale__copy_pairs(
    string scalar design,
    real scalar copies)
{
    real matrix pairs
    real scalar pair, left, right

    if (design == "replicated_blocks" | design == "well_connected") {
        pairs = J(kssbc_scale__pair_count(design,copies),2,.)
        pair = 0
        for (left=1; left<copies; left++) {
            for (right=left+1; right<=copies; right++) {
                pair = pair+1
                pairs[pair,.] = (left,right)
            }
        }
        return(pairs)
    }
    if (design == "ring") {
        if (copies == 2) return((1,2))
        pairs = J(copies,2,.)
        for (left=1; left<copies; left++) {
            pairs[left,.] = (left,left+1)
        }
        pairs[copies,.] = (1,copies)
        return(pairs)
    }
    return(J(0,2,.))
}

// These are exact properties only of the weighted copy-level connector
// meta-graph.  They are descriptive fixture labels, not condition estimates
// for the full replicated worker-firm graph.
real matrix kssbc_scale__meta_metrics(
    string scalar design,
    real scalar copies)
{
    real scalar pairs, conductance, lambda2, lambda_max, condition
    real scalar half

    pairs = kssbc_scale__pair_count(design,copies)
    if (missing(pairs) | copies < 2 | copies != floor(copies)) {
        return(J(1,7,.))
    }
    half = floor(copies/2)
    if (design == "replicated_blocks" | design == "well_connected") {
        conductance = (copies-half)/(copies-1)
        lambda2 = copies/(copies-1)
        lambda_max = lambda2
    }
    else if (design == "ring") {
        if (copies == 2) {
            conductance = 1
            lambda2 = 2
            lambda_max = 2
        }
        else {
            conductance = 1/half
            lambda2 = 1-cos(2*pi()/copies)
            lambda_max = mod(copies,2) == 0 ? 2 : 1+cos(pi()/copies)
        }
    }
    else return(J(1,7,.))
    condition = lambda_max/lambda2
    return((pairs,2*pairs,4*pairs,conductance,lambda2,
        lambda_max,condition))
}

real scalar kssbc_scale__unique_pair_count(
    real colvector worker,
    real colvector firm)
{
    real colvector ordered
    real scalar row, count

    if (rows(worker) == 0) return(0)
    ordered = order((worker,firm),(1,2))
    count = 1
    for (row=2; row<=rows(ordered); row++) {
        count = count+
            (worker[ordered[row]] != worker[ordered[row-1]] |
             firm[ordered[row]] != firm[ordered[row-1]])
    }
    return(count)
}

struct kssbc_scale__diagnostic scalar kssbc_scale__diagnose(
    real colvector worker,
    real colvector firm,
    real colvector deletion_id,
    real colvector frequency)
{
    struct kssbc_scale__diagnostic scalar out
    real colvector ordered, sorted_deletion, edge_first, edge_last
    real colvector edge_worker, edge_firm, degree, adjacency_start
    real colvector adjacency_finish, cursor, neighbor, adjacency_edge
    real colvector discovery, low, dfs_parent, parent_edge, next_arc
    real colvector stack, bridge, worker_order, firm_order
    real matrix panels
    real colvector worker_mass, firm_mass
    real scalar workers, firms, nodes, edges, edge, node, other, arc
    real scalar row, clock, top, root, parent

    out.status = "INVALID_FIXTURE_INPUT"
    out.message = "fixture diagnostic inputs are invalid"
    out.rows = rows(worker)
    out.physical = .
    out.workers = .
    out.firms = .
    out.cells = .
    out.deletion_units = .
    out.components = .
    out.bridge_units = .
    out.minimum_weighted_degree = .
    out.maximum_weighted_degree = .

    if (rows(worker) == 0 | cols(worker) != 1 |
        rows(firm) != rows(worker) | cols(firm) != 1 |
        rows(deletion_id) != rows(worker) | cols(deletion_id) != 1 |
        rows(frequency) != rows(worker) | cols(frequency) != 1 |
        hasmissing(worker) | hasmissing(firm) | hasmissing(deletion_id) |
        hasmissing(frequency) | min(worker) < 1 | min(firm) < 1 |
        min(deletion_id) < 1 | min(frequency) < 1 |
        any(worker :!= floor(worker)) | any(firm :!= floor(firm)) |
        any(deletion_id :!= floor(deletion_id)) |
        any(frequency :!= floor(frequency))) return(out)

    out.physical = sum(frequency)
    out.cells = kssbc_scale__unique_pair_count(worker,firm)

    // A deletion unit is one multigraph edge and must occupy exactly one
    // coefficient coordinate.  Repeated rows and parallel deletion units
    // remain distinct from coefficient cells.
    ordered = order((deletion_id,worker,firm),(1,2,3))
    sorted_deletion = deletion_id[ordered]
    panels = panelsetup(sorted_deletion,1)
    edge_first = ordered[panels[.,1]]
    edge_last = ordered[panels[.,2]]
    if (any(worker[edge_first] :!= worker[edge_last]) |
        any(firm[edge_first] :!= firm[edge_last])) {
        out.status = "CROSS_CELL_DELETION_UNIT"
        out.message = "a deletion ID spans coefficient cells"
        return(out)
    }
    edge_worker = worker[edge_first]
    edge_firm = firm[edge_first]
    edges = rows(edge_first)
    out.deletion_units = edges

    workers = max(worker)
    firms = max(firm)
    nodes = workers+firms
    degree = J(nodes,1,0)
    for (edge=1; edge<=edges; edge++) {
        degree[edge_worker[edge]] = degree[edge_worker[edge]]+1
        node = workers+edge_firm[edge]
        degree[node] = degree[node]+1
    }
    if (any(degree :== 0)) {
        out.status = "FIXTURE_IDS_NOT_DENSE"
        out.message = "fixture worker and firm IDs must be dense"
        return(out)
    }
    out.workers = workers
    out.firms = firms

    worker_order = order(worker,1)
    panels = panelsetup(worker[worker_order],1)
    worker_mass = panelsum(frequency[worker_order],panels)
    firm_order = order(firm,1)
    panels = panelsetup(firm[firm_order],1)
    firm_mass = panelsum(frequency[firm_order],panels)
    out.minimum_weighted_degree = min(worker_mass\firm_mass)
    out.maximum_weighted_degree = max(worker_mass\firm_mass)

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
    out.components = 0
    for (root=1; root<=nodes; root++) {
        if (degree[root] == 0 | discovery[root] != 0) continue
        out.components = out.components+1
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
    out.bridge_units = sum(bridge)
    out.status = "CONVERGED"
    out.message = "fixture multigraph diagnostic completed"
    return(out)
}

void kssbc_scale__stata_diagnose(
    string scalar worker_name,
    string scalar firm_name,
    string scalar deletion_name,
    string scalar frequency_name,
    string scalar matrix_name,
    string scalar status_local,
    string scalar message_local)
{
    struct kssbc_scale__diagnostic scalar out
    real matrix diagnostics

    out = kssbc_scale__diagnose(
        st_data(.,worker_name),
        st_data(.,firm_name),
        st_data(.,deletion_name),
        st_data(.,frequency_name))
    diagnostics = (out.rows,out.physical,out.workers,out.firms,out.cells,
        out.deletion_units,out.components,out.bridge_units,
        out.minimum_weighted_degree,out.maximum_weighted_degree)
    st_matrix(matrix_name,diagnostics)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

void kssbc_scale__stata_connect(
    string scalar design,
    real scalar copies,
    real scalar base_workers,
    real scalar base_firms,
    real scalar base_units,
    string scalar worker_name,
    string scalar firm_name,
    string scalar deletion_name,
    string scalar outcome_name,
    string scalar frequency_name,
    string scalar target_name,
    string scalar copy_name,
    string scalar connector_name,
    string scalar metrics_name)
{
    real matrix pairs, metrics
    real colvector output_rows, worker, firm, deletion, zeros, ones
    real scalar pair, position, first_observation, connector_rows
    real scalar left_firm, right_firm, first_worker, first_deletion

    pairs = kssbc_scale__copy_pairs(design,copies)
    metrics = kssbc_scale__meta_metrics(design,copies)
    connector_rows = 4*rows(pairs)
    first_observation = st_nobs()+1
    st_addobs(connector_rows)
    output_rows = (first_observation::st_nobs())
    worker = J(connector_rows,1,.)
    firm = J(connector_rows,1,.)
    deletion = J(connector_rows,1,.)
    position = 0
    for (pair=1; pair<=rows(pairs); pair++) {
        left_firm = (pairs[pair,1]-1)*base_firms+1
        right_firm = (pairs[pair,2]-1)*base_firms+1
        first_worker = copies*base_workers+2*(pair-1)+1
        first_deletion = copies*base_units+4*(pair-1)+1
        worker[|position+1\position+4|] =
            (first_worker\first_worker\first_worker+1\first_worker+1)
        firm[|position+1\position+4|] =
            (left_firm\right_firm\left_firm\right_firm)
        deletion[|position+1\position+4|] =
            (first_deletion::first_deletion+3)
        position = position+4
    }
    zeros = J(connector_rows,1,0)
    ones = J(connector_rows,1,1)
    st_store(output_rows,worker_name,worker)
    st_store(output_rows,firm_name,firm)
    st_store(output_rows,deletion_name,deletion)
    st_store(output_rows,outcome_name,zeros)
    if (frequency_name != "") st_store(output_rows,frequency_name,ones)
    if (target_name != "") st_store(output_rows,target_name,ones)
    st_store(output_rows,copy_name,zeros)
    st_store(output_rows,connector_name,ones)
    st_matrix(metrics_name,metrics)
}

end
