version 18.0
clear all
set more off
set varabbrev off

args repository_root core_path
if strtrim(`"`repository_root'"') == "" {
    di as error "usage: do test_hierarchy_scale.do repository_root [core_path]"
    exit 198
}

tempfile scale_result
if strtrim(`"`core_path'"') == "" {
    local core_path `"`repository_root'/varcomp_kss/cmg/generated/cmg_test.mata"'
}
do `"`repository_root'/varcomp_kss/cmg/benchmarks/hierarchy_scale_stress.do"' ///
    `"`repository_root'"' `"`scale_result'"' local 8192 4 128 `"`core_path'"'

import delimited using `"`scale_result'"', clear varnames(1)
assert _N == 1
assert vertices == 8192
assert vertices > 6144
assert edges > vertices
assert levels >= 3
assert terminal_vertices <= coarse_max
assert minimum_reduction >= .20
assert edge_complexity <= 3
assert vertex_complexity <= 4
assert structural_bytes > 0
assert dense_factor_bytes > 0
assert workspace_bytes > 0
assert predicted_peak_bytes >= structural_bytes + dense_factor_bytes
assert setup_seconds >= 0
assert rebuild_seconds >= 0
assert two_apply_seconds >= 0
assert relative_residual >= 0 & relative_residual <= 2
assert symmetry_relative_error <= 5e-11
assert workspace_mreldif <= 1e-13
assert timed_applications == 128
assert ordinary_seconds > 0
assert workspace_seconds > 0
assert reldif(workspace_time_ratio, ///
    workspace_seconds/ordinary_seconds) <= 1e-6
assert workspace_time_ratio > 0 & workspace_time_ratio < .

di as result "CMG HIERARCHY SCALE TEST PASS"
exit 0
