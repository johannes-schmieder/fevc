version 18.0
clear all
set more off
set varabbrev off

args suite
if "`suite'" == "" local suite quick
if !inlist("`suite'", "quick", "full") {
    di as error "unknown kss_bc test suite: `suite'"
    exit 198
}

local oldpwd `"`c(pwd)'"'
capture confirm file "kss_bc/kss_bc.ado"
if _rc {
    capture confirm file "../../kss_bc.ado"
    if _rc {
        di as error "run from the repository root or kss_bc/tests/stata"
        exit 601
    }
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/kss_bc"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'
do `"`pkgroot'/tests/stata/test_exact_fixture.do"'
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

if "`suite'" == "full" {
    do `"`pkgroot'/tests/stata/test_lockstep_pcg.do"'
    do `"`pkgroot'/tests/stata/test_forced_cmg.do"'
    do `"`pkgroot'/tests/stata/test_frequency.do"'
    do `"`pkgroot'/tests/stata/test_semantics.do"'
    do `"`pkgroot'/tests/stata/test_jla_fixture.do"'
    do `"`pkgroot'/tests/stata/test_jla_convergence.do"'
    do `"`pkgroot'/tests/stata/test_perf_batch_runtime.do"'
    // This intentionally replaces only the JLA Mata bridge and must run
    // after every ordinary-estimator test.
    do `"`pkgroot'/tests/stata/test_forced_cmg_e2e.do"'
}

// This intentionally replaces the semantic build token and must run last.
do `"`pkgroot'/tests/stata/test_stale_runtime.do"'

quietly cd `"`oldpwd'"'
di as result "KSS_BC TEST SUITE PASS: `suite'"
exit 0
