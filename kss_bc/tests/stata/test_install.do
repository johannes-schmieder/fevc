version 18.0
clear all
set more off
set varabbrev off

args package_source install_root
if strtrim(`"`package_source'"') == "" | strtrim(`"`install_root'"') == "" {
    di as error "usage: do test_install.do package_source install_root"
    exit 198
}

sysdir set PLUS `"`install_root'/"'
net install kss_bc, from(`"`package_source'"') replace
discard
mata: mata clear

capture findfile kss_bc.ado
assert _rc == 0
local installed_ado `"`r(fn)'"'
assert strpos(`"`installed_ado'"',`"`install_root'"') == 1
capture findfile kss_bc.mata
assert _rc == 0
capture findfile kss_bc_graph.mata
assert _rc == 0
capture findfile kss_bc_cmg.mata
assert _rc == 0
capture findfile kss_bc_solver.mata
assert _rc == 0
capture findfile kss_bc_rng.mata
assert _rc == 0
capture findfile kss_bc_scale.mata
assert _rc == 0
capture findfile kss_bc_resource.mata
assert _rc == 0
capture findfile kss_bc_scale_engine.mata
assert _rc == 0
capture findfile kss_bc_scale_runtime.mata
assert _rc == 0
capture findfile kss_bc_lifecycle.ado
assert _rc == 0
capture findfile kss_bc.sthlp
assert _rc == 0

capture noisily kss_bc, version
assert _rc == 0
assert "`e(version)'" == "0.2.0-dev"

clear
input double(y worker firm match)
1.0 1 1 11
1.1 1 1 11
1.2 1 2 12
1.3 1 2 12
1.4 2 1 21
1.5 2 1 21
1.6 2 2 22
1.7 2 2 22
end
kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert e(N) == 8
assert e(graph_final_bridge_units) == 0

// The normal installed path must load and execute the supported CMG backend,
// not merely place its source files on disk.
clear
local workers = 1200
local firms = 300
local degree = 3
set obs `=`degree'*`workers''
generate long worker = floor((_n-1)/`degree')+1
generate byte link = mod(_n-1,`degree')
generate long firm = mod(worker-1+cond(link==2,17,link),`firms')+1
generate double y = sin(worker/37)+cos(firm/19)+link/101
kss_bc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) preconditioner(cmg) probes(4) batch(4) ///
    memory_gib(4) seed(8675309) tolerance(1e-10) nodisplay
assert "`e(status)'" == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
assert "`e(engine_selected)'" == "compressed"
assert "`e(preconditioner_selected)'" == "CMG"
assert e(route_hierarchy_levels) >= 1
assert e(route_terminal_vertices) > 0 & e(route_terminal_vertices) <= 6144
assert e(solver_max_residual) <= 1e-9

discard
mata: mata clear
quietly do "`install_root'/k/kss_bc.mata"
quietly do "`install_root'/k/kss_bc_graph.mata"
quietly do "`install_root'/k/kss_bc_cmg.mata"
quietly do "`install_root'/k/kss_bc_rng.mata"
quietly do "`install_root'/k/kss_bc_scale.mata"
quietly do "`install_root'/k/kss_bc_resource.mata"
quietly do "`install_root'/k/kss_bc_solver.mata"
quietly do "`install_root'/k/kss_bc_scale_engine.mata"
quietly do "`install_root'/k/kss_bc_scale_runtime.mata"
mata: assert(kssbc__api_level() == 19)
mata: assert(kssbc_graph__api_level() == 18)
mata: assert(kssbc_cmg__api_level() == 5)
mata: assert(kssbc_solver__api_level() == 22)
mata: assert(kssbc_rng__api_level() == 2)
mata: assert(kssbc_scale__api_level() == 2)
mata: assert(kssbc_resource__api_level() == 5)
mata: assert(kssbc_scale_engine__api_level() == 1)
mata: assert(kssbc_scale_runtime__api_level() == 1)

di as result "KSS_BC INSTALL TEST PASS"
exit 0
