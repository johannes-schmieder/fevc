version 18.0
clear all
set more off
set varabbrev off

capture confirm file "kss_bc/kss_bc.mata"
if _rc {
    di as error "run the scale route-diagnostics test from the repository root"
    exit 601
}

quietly do "kss_bc/kss_bc.mata"
quietly do "kss_bc/kss_bc_cmg.mata"
quietly do "kss_bc/kss_bc_rng.mata"
quietly do "kss_bc/kss_bc_solver.mata"
mata: assert(kssbc_solver__api_level() == 24)

set rng kiss32
set seed 20260817
local rng_before `"`c(rngstate)'"'

mata:
void test_structural_routes()
{
    real colvector worker, firm, frequency, target, deletion_id, y
    struct kssbc_route_result scalar automatic, diagonal, cmg

    worker = (1 \ 1 \ 1 \ 2 \ 2 \ 2 \ 3 \ 3 \ 3)
    firm = (1 \ 2 \ 3 \ 1 \ 2 \ 3 \ 1 \ 2 \ 3)
    frequency = J(9,1,1)
    target = J(9,1,1)
    deletion_id = 1::9
    y = (1 \ 2 \ 4 \ 3 \ 7 \ 8 \ 6 \ 9 \ 13)

    automatic = kssbc_solver__jla_routed(
        y,worker,firm,J(9,0,.),frequency,target,deletion_id,
        "match","joint",4,2,8675309,1e-8,1000,1e-10,1e-10,10000,
        "AUTO",1024^3)
    assert(automatic.status == "CONVERGED")
    assert(automatic.route == "DIAGONAL")
    assert(strtrim(automatic.reason) != "")
    assert(cols(automatic.diagnostics) == 26)
    assert(missing(automatic.diagnostics[18..21]))
    assert(missing(automatic.diagnostics[23..24]))
    assert(min(automatic.pilot_status :== "NOT_RUN") == 1)
    assert(max(automatic.pilot_diagnostics[.,3]) == 0)

    diagonal = kssbc_solver__jla_routed(
        y,worker,firm,J(9,0,.),frequency,target,deletion_id,
        "match","joint",4,2,8675309,1e-8,1000,1e-10,1e-10,10000,
        "DIAGONAL",1024^3)
    assert(diagonal.status == "CONVERGED")
    assert(diagonal.route == "DIAGONAL")
    assert(mreldif(diagonal.estimator.plugin,
        automatic.estimator.plugin) < 1e-12)
    assert(mreldif(diagonal.estimator.corrected,
        automatic.estimator.corrected) < 1e-12)

    cmg = kssbc_solver__jla_routed(
        y,worker,firm,J(9,0,.),frequency,target,deletion_id,
        "match","joint",4,2,8675309,1e-8,1000,1e-10,1e-10,10000,
        "CMG",1024^3)
    assert(cmg.status == "CONVERGED")
    assert(cmg.route == "CMG")
    assert(min(cmg.pilot_status :== "NOT_RUN") == 1)
    assert(max(cmg.pilot_diagnostics[.,3]) == 0)
    assert(cmg.estimator.solver_max_residual <= 1e-7)
    assert(mreldif(cmg.estimator.corrected,
        diagonal.estimator.corrected) < 1e-6)
}
assert(kssbc_rng__guard_begin() == 0)
test_structural_routes()
assert(kssbc_rng__guard_restore() == 0)
end

assert `"`c(rngstate)'"' == `"`rng_before'"'
di as result "PASS test_scale_route_diagnostics.do"
exit 0
