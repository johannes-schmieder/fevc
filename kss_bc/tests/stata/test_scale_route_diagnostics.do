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
quietly do "kss_bc/kss_bc_solver.mata"
mata: assert(kssbc_solver__pilot_api() == 1)

mata:
void test_pilot_evidence_helpers()
{
    struct kssbc_solve_result scalar solved
    struct kssbc_solver_pilot_evidence scalar evidence

    solved.status = "CONVERGED"
    solved.message = "synthetic pilot result"
    solved.rhs_status = J(1,4,"CONVERGED")
    solved.rhs_iterations = (2,4,6,8)
    solved.rhs_relres = (1e-12,2e-12,3e-12,4e-12)
    solved.schur_actions = 20
    solved.preconditioner_applications = 20
    evidence = kssbc_solver__pilot_evidence(
        solved,1,61,32,1e-10,1000,100,50,.,.,.,.,.,0)
    assert(rows(evidence.diagnostics) == 4)
    assert(cols(evidence.diagnostics) == 16)
    assert(min(evidence.diagnostics[.,3]) == 1)
    assert(min(evidence.diagnostics[.,4]) == 1)
    assert(evidence.diagnostics[4,5] == 8)
    assert(evidence.diagnostics[4,6] == 4e-12)
    assert(evidence.diagnostics[4,7] == 8)
    assert(evidence.diagnostics[4,8] == 8)
    assert(evidence.diagnostics[4,9] == 20)
    assert(evidence.diagnostics[4,10] == 20)
    assert(evidence.diagnostics[4,11] > 0)
    assert(max(evidence.diagnostics[.,16]) == 0)
    assert(min(evidence.failure_reason :== "passed all pilot gates") == 1)

    // The complete coefficient-equation residual is an independent gate.
    solved.status = "SOLVER_RESIDUAL_FAILED"
    solved.rhs_status[3] = "SOLVER_RESIDUAL_FAILED"
    solved.rhs_relres[3] = 2e-9
    evidence = kssbc_solver__pilot_evidence(
        solved,1,61,32,1e-10,1000,100,50,.,.,.,.,.,0)
    assert(evidence.diagnostics[3,14] == 0)
    assert(evidence.diagnostics[3,16] == 8)
    assert(evidence.diagnostics[3,7] == 6)
    assert(evidence.status[3] ==
        "SOLVER_RESIDUAL_FAILED/SOLVER_RESIDUAL_FAILED")
    assert(strpos(evidence.failure_reason[3],"original-system residual") > 0)

    // The projected B1 work gate remains visible pilot by pilot.
    solved.status = "CONVERGED"
    solved.rhs_status = J(1,4,"CONVERGED")
    solved.rhs_relres = J(1,4,1e-12)
    solved.rhs_iterations = J(1,4,95)
    evidence = kssbc_solver__pilot_evidence(
        solved,1,601,128,1e-10,8201888,117529,10603,.,.,.,.,.,0)
    assert(max(evidence.diagnostics[.,11]) > 8e9)
    assert(max(evidence.diagnostics[.,15]) == 0)
    assert(max(evidence.diagnostics[.,16]) == 10)

    // Malformed diagnostic vectors fail closed and remain inspectable.
    solved.rhs_status = J(1,3,"CONVERGED")
    evidence = kssbc_solver__pilot_evidence(
        solved,1,601,32,1e-10,100,20,10,.,.,.,.,.,0)
    assert(min(evidence.status :== "MALFORMED_DIAGNOSTICS") == 1)
    assert(min(evidence.diagnostics[.,16]) == 2)
    assert(min(evidence.failure_reason :==
        "pilot result vectors do not each contain four entries") == 1)
}

void test_pre_rng_no_route_evidence()
{
    real scalar firms, workers, degree
    real colvector worker, link, firm, frequency, target, deletion_id, y
    struct kssbc_route_result scalar routed

    // This multilevel case mirrors the predecessor's failure class: both B1
    // and CMG pilots are attempted, neither route qualifies, and estimator
    // RNG is never initialized.  maxiter(1) makes the local rejection bounded
    // and deterministic rather than recreating the restricted 2x input.
    firms = 6200
    workers = 24800
    degree = 3
    worker = floor(((1::(degree*workers)):-1)/degree):+1
    link = mod((1::(degree*workers)):-1,degree)
    firm = mod(worker:-1:+(link:==2):*17:+(link:!=2):*link,firms):+1
    frequency = J(rows(worker),1,1)
    target = J(rows(worker),1,1)
    deletion_id = 1::rows(worker)
    y = sin(worker:/37)+cos(firm:/19)+link:/101
    routed = kssbc_solver__jla_routed(
        y,worker,firm,J(rows(worker),0,.),frequency,target,deletion_id,
        "match","joint",200,8,8675309,1e-10,1,1e-10,1e-10,10000,
        "AUTO",4*1024^3)
    assert(routed.status == "NO_REALISTIC_SOLVER_ROUTE")
    assert(rows(routed.pilot_diagnostics) == 8)
    assert(cols(routed.pilot_diagnostics) == 16)
    assert(min(routed.pilot_diagnostics[.,3]) == 1)
    assert(min(routed.pilot_diagnostics[.,9]) > 0)
    assert(min(routed.pilot_diagnostics[.,10]) > 0)
    assert(min(routed.pilot_diagnostics[.,11]) > 0)
    assert(max(routed.pilot_diagnostics[.,4]) == 0)
    assert(min(routed.pilot_diagnostics[.,16]) > 0)
    assert(min(routed.pilot_status :!= "NOT_RUN") == 1)
    st_matrix("scale_route_pilots",routed.pilot_diagnostics)
}

test_pilot_evidence_helpers()
end

set seed 20260816
local rng_before `"`c(rngstate)'"'
mata: test_pre_rng_no_route_evidence()
assert `"`c(rngstate)'"' == `"`rng_before'"'

matrix colnames scale_route_pilots = backend pilot attempted passed ///
    iterations complete_residual rhs_schur_actions rhs_precond_apps ///
    backend_schur_actions backend_precond_apps projected_work ///
    status_gate iteration_gate residual_gate work_gate failure_reason_code
assert scale_route_pilots[1,1] == 1
assert scale_route_pilots[5,1] == 2

di as result "PASS test_scale_route_diagnostics.do"
exit 0
