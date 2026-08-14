version 18.0
clear
input double(y worker firm match)
1.0 1 1 11
1.2 1 2 12
1.4 2 1 21
1.8 2 2 22
2.0 3 1 31
2.5 3 3 33
end

kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert e(N) == 4
assert e(N_retained) == 4
assert e(N_mover_input) == 6
assert e(N_graph_dropped) == 2
assert e(graph_edges) == 6
assert e(graph_articulation_workers) == 1
assert e(graph_leaveout_components) == 1
assert "`e(connectedness_status)'" == "LEAVE_ONE_WORKER_CONNECTED"
assert "`e(sample_selection)'" == ///
    "MOVERS_MATLAB_LEAVEONEWORKER_COMPONENT"

// A bow-tie has no bridge edge, but its central worker is an articulation.
// This fixture separates the MATLAB compatibility rule from edge pruning.
clear
input double(y worker firm match)
1.01 1 1 101
1.02 1 2 102
1.03 1 3 103
1.04 1 4 104
1.21 2 1 201
1.22 2 2 202
1.31 3 1 301
1.32 3 2 302
1.43 4 3 403
1.44 4 4 404
1.53 5 3 503
1.54 5 4 504
end
capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "AMBIGUOUS_LARGEST_COMPONENT"

// A tie in the MATLAB-style largest-component criterion has no canonical
// ID-invariant winner.  Withhold instead of breaking the tie on encoded IDs.
clear
input double(y worker firm match)
1.01 1 1 11
1.02 1 2 12
1.03 2 1 21
1.04 2 2 22
2.01 3 3 33
2.02 3 4 34
2.03 4 3 43
2.04 4 4 44
end

capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "AMBIGUOUS_LARGEST_COMPONENT"

generate double relabeled_worker = cond(worker <= 2,worker+100,worker-2)
generate double relabeled_firm = cond(firm <= 2,firm+100,firm-2)
capture noisily kss_bc y, worker(relabeled_worker) firm(relabeled_firm) ///
    deletion(match) deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "AMBIGUOUS_LARGEST_COMPONENT"

di as result "PASS test_graph_pruning.do"
