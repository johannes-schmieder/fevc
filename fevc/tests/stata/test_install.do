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
net install fevc, from(`"`package_source'"') replace
discard
mata: mata clear

capture findfile fevc.ado
assert _rc == 0
local installed_ado `"`r(fn)'"'
assert strpos(`"`installed_ado'"',`"`install_root'"') == 1
capture findfile vckss.mata
assert _rc == 0
capture findfile vckss_inference.mata
assert _rc == 0
capture findfile vckss_graph.mata
assert _rc == 0
capture findfile vckss_cmg.mata
assert _rc == 0
capture findfile vckss_solver.mata
assert _rc == 0
capture findfile vckss_rng.mata
assert _rc == 0
capture findfile vckss_scale.mata
assert _rc == 0
capture findfile vckss_resource.mata
assert _rc == 0
capture findfile vckss_scale_engine.mata
assert _rc == 0
capture findfile vckss_scale_runtime.mata
assert _rc == 0
capture findfile vckss_lifecycle.ado
assert _rc == 0
capture findfile fevc_run.ado
assert _rc == 0
capture findfile fevc.sthlp
assert _rc == 0
capture noisily help fevc
assert _rc == 0

clear
set obs 3
generate long sentinel = _n
quietly _datasignature
local caller_signature `"`r(datasignature)'"'
fevc_run exact_controls using fevc.sthlp
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "`e(backend_requested)'" == "auto"
assert "`e(backend_selected)'" == "mata"
assert e(backend_fallback) == 1
assert "`e(backend_fallback_reason)'" == "RUST_BACKEND_UNAVAILABLE"
assert "`e(backend_fallback_phase)'" == "preflight"
assert "`e(rng_requested)'" == "auto"
assert "`e(rng_selected)'" == "stata"
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

capture noisily fevc, version
assert _rc == 0
assert "`e(version)'" == "0.5.0-alpha.1"

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
fevc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert e(N) == 8
assert e(graph_final_bridge_units) == 0

clear
set obs 24
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time - 1.5
generate double c2 = time == 2
generate byte firm = .
generate double noise = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise
fevc y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) inference(highrank) ///
    inferencesimulations(100) inferenceseed(42) inferencebins(16) nodisplay
assert "`e(status)'" == "KSS_HIGHRANK_INFERENCE"
assert rowsof(e(V)) == 4

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
fevc y, worker(worker) firm(firm) deletion(match) ///
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
quietly do "`install_root'/v/vckss.mata"
quietly do "`install_root'/v/vckss_inference.mata"
quietly do "`install_root'/v/vckss_graph.mata"
quietly do "`install_root'/v/vckss_cmg.mata"
quietly do "`install_root'/v/vckss_rng.mata"
quietly do "`install_root'/v/vckss_scale.mata"
quietly do "`install_root'/v/vckss_resource.mata"
quietly do "`install_root'/v/vckss_solver.mata"
quietly do "`install_root'/v/vckss_scale_engine.mata"
quietly do "`install_root'/v/vckss_scale_runtime.mata"
mata: assert(vckss__api_level() == 21)
mata: assert(vckss_inference__api_level() == 1)
mata: assert(vckss_graph__api_level() == 21)
mata: assert(vckss_cmg__api_level() == 8)
mata: assert(vckss_solver__api_level() == 26)
mata: assert(vckss_rng__api_level() == 4)
mata: assert(vckss_scale__api_level() == 6)
mata: assert(vckss_scale__build_id() ==                         ///
    "vckss-scale-api6-prep-sem1-mata")
mata: assert(vckss_resource__api_level() == 10)
mata: assert(vckss_scale_engine__api_level() == 4)
mata: assert(vckss_scale_runtime__api_level() == 3)

di as result "FEVC INSTALL TEST PASS"
exit 0
