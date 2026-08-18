version 18.0

// Each deletion ID is an edge.  The two parallel deletion units at (1,1)
// protect one another; only deletion unit 3 is a bridge.
mata:
bridge = vckss_graph__bridges((1\1\2),(1\1\1),(1\2\3),J(3,1,1))
assert(bridge.status == "CONVERGED")
assert(bridge.edges == 3)
assert(bridge.deletion_id == 3)

// Every edge in this path is returned in one certificate, so callers can
// remove the complete bridge set simultaneously rather than order by IDs.
bridge = vckss_graph__bridges(
    (1\1\2\2),(1\2\2\3),(11\12\22\23),J(4,1,1))
assert(bridge.status == "CONVERGED")
assert(bridge.edges == 4)
assert(bridge.deletion_id == (11\12\22\23))
end

clear
input double(y worker firm deletion_unit)
1.01 1 1 11
1.02 1 2 12
1.21 2 1 21
1.22 2 2 22
1.31 3 1 31
1.33 3 3 33
end
varcomp_kss y, worker(worker) firm(firm) deletion(match) ///
    deletionid(deletion_unit) algorithm(exact) nodisplay
assert e(graph_final_bridge_units) == 0
assert e(graph_retained_edges) == 4
assert e(graph_fixedpoint_iterations) >= e(graph_pruning_iterations)
assert "`e(sample_selection)'" == "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT"
assert "`e(connectedness_status)'" == "DELETION_UNIT_BRIDGE_FREE"

di as result "PASS test_graph_multigraph.do"
