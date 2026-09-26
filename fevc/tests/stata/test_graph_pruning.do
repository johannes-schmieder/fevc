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

fevc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert e(N) == 4
assert e(N_retained) == 4
assert e(N_mover_input) == 6
assert e(N_graph_dropped) == 2
assert e(graph_edges) == 6
assert e(graph_articulation_workers) == 1
assert e(graph_leaveout_components) == 1
assert "`e(connectedness_status)'" == "DELETION_UNIT_BRIDGE_FREE"
assert "`e(sample_selection)'" == ///
    "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT"
assert e(graph_retained_edges) == 4
assert e(graph_final_bridge_units) == 0

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
capture noisily fevc y, worker(worker) firm(firm) deletion(match) ///
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

capture noisily fevc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "AMBIGUOUS_LARGEST_COMPONENT"

generate double relabeled_worker = cond(worker <= 2,worker+100,worker-2)
generate double relabeled_firm = cond(firm <= 2,firm+100,firm-2)
capture noisily fevc y, worker(relabeled_worker) firm(relabeled_firm) ///
    deletion(match) deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "AMBIGUOUS_LARGEST_COMPONENT"

// PREP-MAP-1 redensifies only Stata's complete-case numeric group codes.
// Removing the middle lexical level therefore closes a real gap while
// matching retained-sample egen group() exactly for public string IDs.
clear
input str1 worker_s str1 firm_s double y
"a" "a" 1.01
"a" "z" 1.02
"z" "a" 1.03
"z" "z" 1.04
"m" "m" 9.99
end
generate str2 deletion_s = worker_s+firm_s
generate double frequency = 1
generate byte touse = 1
generate byte graph_keep = 0
generate long retained_worker = .
generate long retained_firm = .
quietly egen long initial_worker = group(worker_s) if touse
quietly egen long initial_firm = group(firm_s) if touse
quietly egen long graph_deletion = group(deletion_s) if touse
tempname map_diagnostics
local map_status
local map_message
local map_worker_levels
local map_firm_levels
mata: vckss_graph__stata_prune(                                 ///
    "initial_worker", "initial_firm", "frequency",             ///
    "graph_deletion", "touse", "match", "graph_keep",         ///
    "`map_diagnostics'", "map_status", "map_message",          ///
    "retained_worker", "retained_firm",                         ///
    "map_worker_levels", "map_firm_levels")
assert "`map_status'" == "CONVERGED"
assert real("`map_worker_levels'") == 2
assert real("`map_firm_levels'") == 2
assert `map_diagnostics'[1,19] >= 0
assert `map_diagnostics'[1,20] == 2
assert `map_diagnostics'[1,21] == 2
assert graph_keep == (_n <= 4)
quietly egen long oracle_worker = group(worker_s) if graph_keep
quietly egen long oracle_firm = group(firm_s) if graph_keep
assert retained_worker == oracle_worker if graph_keep
assert retained_firm == oracle_firm if graph_keep
assert missing(retained_worker) & missing(retained_firm) if !graph_keep

fevc y, worker(worker_s) firm(firm_s) deletion(match)      ///
    deletionid(deletion_s) algorithm(exact) stayers(movers) nodisplay
assert e(N_retained) == 4
assert e(worker_levels) == 2
assert e(firm_levels) == 2
matrix map_counts = e(prep_boundary_counts)
assert map_counts[1,3] == 0

// The eligible compressed public path consumes the graph-returned dense
// string maps directly.  Without probeorder(), PREP-SEM-1 imports only the
// six numerical columns needed to reproduce Stata's group/sort oracle.
generate double target = frequency*(1+mod(_n,3)/4)
local map_rng `"`c(rngstate)'"'
fevc y [fw=frequency], worker(worker_s) firm(firm_s)       ///
    deletion(match) deletionid(deletion_s) targetweight(target)   ///
    algorithm(jla) engine(compressed) probes(8) batch(3)          ///
    seed(20260819) stayers(movers) nodisplay
assert "`e(engine_selected)'" == "compressed"
assert e(N_retained) == 4
assert e(worker_levels) == 2
assert e(firm_levels) == 2
matrix map_counts = e(prep_boundary_counts)
assert map_counts[1,3] == 0
assert map_counts[1,4] == 0
assert map_counts[1,5] == 3
assert map_counts[1,8] == 2
assert map_counts[1,10] == 6
assert map_counts[1,11] == e(N_retained)
quietly count if e(sample) != graph_keep
assert r(N) == 0
assert `"`c(rngstate)'"' == `"`map_rng'"'

di as result "PASS test_graph_pruning.do"
