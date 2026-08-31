version 18.0
clear all
set more off
set varabbrev off

args suite
if "`suite'" == "" local suite quick
if !inlist("`suite'", "quick", "full") {
    di as error "unknown fevc test suite: `suite'"
    exit 198
}

local oldpwd `"`c(pwd)'"'
capture confirm file "fevc/fevc.ado"
if _rc {
    capture confirm file "../../fevc.ado"
    if _rc {
        di as error "run from the repository root or fevc/tests/stata"
        exit 601
    }
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/fevc"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'
do `"`pkgroot'/tests/stata/test_exact_fixture.do"'
do `"`pkgroot'/tests/stata/test_stayers_hybrid.do"'
do `"`pkgroot'/tests/stata/test_backend_routing.do"' `"`pkgroot'"'
capture quietly fevc_rust probe
if !_rc {
    do `"`pkgroot'/tests/stata/test_rust_exact_controls.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_generic_jla.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_planned_v4.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_planned_compressed.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_planned_compressed_post.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_full_cmg_v2.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_public_exact.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_public_generic.do"' `"`pkgroot'"'
    do `"`pkgroot'/tests/stata/test_rust_projection.do"' `"`pkgroot'"'
}
else {
    di as txt "FEVC RUST ROUTE TESTS SKIPPED: no loadable developer artifact"
}
do `"`pkgroot'/tests/stata/test_output.do"'
do `"`pkgroot'/tests/stata/test_help_examples.do"'
do `"`pkgroot'/tests/stata/test_failures.do"'
do `"`pkgroot'/tests/stata/test_graph_pruning.do"'
do `"`pkgroot'/tests/stata/test_graph_multigraph.do"'
do `"`pkgroot'/tests/stata/test_probe_order.do"'
do `"`pkgroot'/tests/stata/test_control_anchor.do"'
do `"`pkgroot'/tests/stata/test_routing_api.do"'
do `"`pkgroot'/tests/stata/test_batch_invariance.do"'
do `"`pkgroot'/tests/stata/test_batch_memory_limit.do"'
do `"`pkgroot'/tests/stata/test_perf_exact_terminal.do"'
do `"`pkgroot'/tests/stata/test_solver_prod_fixes.do"'
do `"`pkgroot'/tests/stata/test_full_rhs_certificate.do"'
do `"`pkgroot'/tests/stata/test_scale_match_formula.do"'
do `"`pkgroot'/tests/stata/test_scale_compression.do"'
do `"`pkgroot'/tests/stata/test_scale_semantic_atoms.do"'
do `"`pkgroot'/tests/stata/test_scale_resource.do"'
do `"`pkgroot'/tests/stata/test_scale_lifecycle.do"'
do `"`pkgroot'/tests/stata/test_scale_fixtures.do"'
do `"`pkgroot'/tests/stata/test_scale_scc_driver.do"'
do `"`pkgroot'/tests/stata/test_scale_route_diagnostics.do"'
do `"`pkgroot'/tests/stata/test_scale_engine.do"'
do `"`pkgroot'/tests/stata/test_scale_engine_reductions.do"'
do `"`pkgroot'/tests/stata/test_scale_command.do"'

if "`suite'" == "full" {
    do `"`pkgroot'/tests/stata/test_lockstep_pcg.do"'
    do `"`pkgroot'/tests/stata/test_forced_cmg.do"'
    do `"`pkgroot'/tests/stata/test_frequency.do"'
    do `"`pkgroot'/tests/stata/test_semantics.do"'
    do `"`pkgroot'/tests/stata/test_jla_fixture.do"'
    do `"`pkgroot'/tests/stata/test_jla_convergence.do"'
    do `"`pkgroot'/tests/stata/test_perf_batch_runtime.do"'
    do `"`pkgroot'/tests/stata/test_scale_rng.do"'
    // This intentionally replaces only the JLA Mata bridge and must run
    // after every ordinary-estimator test.
    do `"`pkgroot'/tests/stata/test_forced_cmg_e2e.do"'
}

// This intentionally replaces the semantic build token and must run last.
do `"`pkgroot'/tests/stata/test_stale_runtime.do"'

quietly cd `"`oldpwd'"'
di as result "FEVC TEST SUITE PASS: `suite'"
exit 0
