version 18.0
clear all
set more off
set varabbrev off

capture confirm file "fevc/vckss.mata"
if _rc {
    di as error "run the full-RHS certificate test from the repository root"
    exit 601
}

quietly do "fevc/vckss.mata"

mata:
/* Independent row-loop normal-equation actions.  These do not use the
   production transpose or grouped-reduction helpers. */
real matrix kssrhs_oracle__fe_lhs(
    struct vckss_fe_design scalar design,
    real matrix values)
{
    real scalar row
    real matrix out

    out = J(design.worker_levels+design.firm_levels,cols(values),0)
    for (row=1; row<=design.n; row++) {
        out[design.worker[row],.] = out[design.worker[row],.] +
            design.frequency[row]:*values[row,.]
        out[design.worker_levels+design.firm[row],.] =
            out[design.worker_levels+design.firm[row],.] +
            design.frequency[row]:*values[row,.]
    }
    return(out)
}

real matrix kssrhs_oracle__joint_lhs(
    struct vckss_joint_design scalar design,
    real matrix values)
{
    return(kssrhs_oracle__fe_lhs(design.base,values) \
        design.controls'*(design.base.frequency:*values))
}

real rowvector kssrhs_oracle__relres(
    real matrix left_hand_side,
    real matrix right_hand_side)
{
    real scalar column, denominator
    real rowvector out

    assert(rows(left_hand_side) == rows(right_hand_side))
    assert(cols(left_hand_side) == cols(right_hand_side))
    out = J(1,cols(right_hand_side),.)
    for (column=1; column<=cols(right_hand_side); column++) {
        denominator = sqrt(sum(right_hand_side[.,column]:^2))
        out[column] = sqrt(sum((left_hand_side[.,column]-
            right_hand_side[.,column]):^2))
        if (denominator > 0) out[column] = out[column]/denominator
    }
    return(out)
}

VCKSS_TEST_PACK_WIDTHS = J(1,0,.)

struct vckss_preconditioner_result scalar kssrhs_oracle__packed_apply(
    pointer scalar context,
    struct vckss_fe_design scalar design,
    real matrix residual)
{
    external real rowvector VCKSS_TEST_PACK_WIDTHS

    VCKSS_TEST_PACK_WIDTHS = VCKSS_TEST_PACK_WIDTHS,cols(residual)
    return(vckss__diagonal_apply(context,design,residual))
}

void test_full_rhs_certificate()
{
    external real rowvector VCKSS_TEST_PACK_WIDTHS
    real scalar tolerance, maxiter, rank_tolerance, workers, firms
    real scalar inferred_last, supplied_last, gate
    real colvector worker, firm, frequency, fit_values, leverage_values
    real colvector control
    real matrix fit_rhs, leverage_rhs, worker_target_rhs, firm_target_rhs
    real matrix full_rhs, packed_rhs, reduced_rhs, perturbed_rhs, manual_relres
    real matrix joint_rhs, reduced_joint_rhs, perturbed_joint_rhs
    real rowvector control_rhs
    struct vckss_fe_design scalar design
    struct vckss_joint_design scalar joint
    struct vckss_solver_backend scalar backend
    struct vckss_solve_result scalar solved, direct, packed, scalar_reference
    struct vckss_solve_result scalar legacy, rejected, joint_solved
    struct vckss_solve_result scalar joint_legacy, joint_rejected

    tolerance = 1e-12
    rank_tolerance = 1e-12
    maxiter = 10000
    gate = max((1e-11,10*tolerance))
    workers = 3
    firms = 3

    /* Complete K(3,3) keeps the coefficient system well conditioned. */
    worker = (1\1\1\2\2\2\3\3\3)
    firm = (1\2\3\1\2\3\1\2\3)
    frequency = J(rows(worker),1,1)
    design = vckss__fe_prepare(
        worker,firm,frequency,rank_tolerance)
    assert(design.status == "CONVERGED")
    backend = vckss__diagonal_backend()

    /* Exact mathematical worker margins are (1e16,1,-1e16), while the
       firm margins are (1e16,-1e16,1).  Naive compatibility inference loses
       the unit last-firm RHS even though the supplied grouped RHS retains it. */
    fit_values = (1e16\0\0\0\0\1\0\-1e16\0)
    fit_rhs = kssrhs_oracle__fe_lhs(design,fit_values)
    supplied_last = fit_rhs[workers+firms]
    inferred_last = colsum(fit_rhs[1..workers])-
        colsum(fit_rhs[(workers+1)..(workers+firms-1)])
    assert(supplied_last == 1)
    assert(inferred_last == 0)
    assert(supplied_last != inferred_last)

    /* A leverage-shaped column comes from one row-level sign direction. */
    leverage_values = (1\-1\1\-1\1\-1\1\1\-1)
    leverage_rhs = kssrhs_oracle__fe_lhs(design,leverage_values)

    /* Target solves separately populate the worker and firm score blocks.
       Their full score vectors satisfy the exact compatibility condition. */
    worker_target_rhs = (2\-5\3\0\0\0)
    firm_target_rhs = (0\0\0\4\-7\3)
    full_rhs = fit_rhs,leverage_rhs,worker_target_rhs,firm_target_rhs
    assert(rows(full_rhs) == workers+firms)
    assert(cols(full_rhs) == 4)

    solved = vckss__fe_solve_matrix(
        design,full_rhs,tolerance,maxiter)
    assert(solved.status == "CONVERGED")
    assert(min(solved.rhs_status:=="CONVERGED") == 1)
    assert(max(solved.rhs_relres) <= gate)
    manual_relres = kssrhs_oracle__relres(
        kssrhs_oracle__fe_lhs(design,solved.prediction),full_rhs)
    assert(max(abs(solved.rhs_relres-manual_relres)) <= 2e-15)

    /* Active-column packing must never send a structurally zero RHS through
       the backend.  Logical order and the four nonzero solutions remain
       unchanged when a zero column is interleaved. */
    packed_rhs = full_rhs[.,1..2],J(rows(full_rhs),1,0),full_rhs[.,3..4]
    VCKSS_TEST_PACK_WIDTHS = J(1,0,.)
    backend.apply = &kssrhs_oracle__packed_apply()
    packed = vckss__fe_solve_matrix_backend(
        design,packed_rhs,tolerance,maxiter,backend)
    assert(packed.status == "CONVERGED")
    assert(max(VCKSS_TEST_PACK_WIDTHS) <= 4)
    assert(min(VCKSS_TEST_PACK_WIDTHS) >= 1)
    assert(packed.rhs_status[3] == "CONVERGED")
    assert(packed.rhs_iterations[3] == 0)
    assert(max(abs(packed.coefficient[.,3])) == 0)
    assert(max(abs(packed.prediction[.,3])) == 0)
    assert(mreldif(packed.coefficient[.,(1,2,4,5)],
        solved.coefficient) <= 2e-15)
    backend = vckss__diagonal_backend()

    /* Exercise the explicit backend entry and the scalar reference with the
       same supplied full equations. */
    direct = vckss__fe_solve_matrix_backend(
        design,full_rhs,tolerance,maxiter,backend)
    scalar_reference = vckss__fe_solve_matrix_b0(
        design,full_rhs,tolerance,maxiter)
    assert(direct.status == "CONVERGED")
    assert(scalar_reference.status == "CONVERGED")
    assert(mreldif(direct.coefficient,solved.coefficient) <= 2e-14)
    assert(mreldif(
        scalar_reference.coefficient,solved.coefficient) <= 2e-12)
    manual_relres = kssrhs_oracle__relres(
        kssrhs_oracle__fe_lhs(
            design,scalar_reference.prediction),full_rhs)
    assert(max(abs(
        scalar_reference.rhs_relres-manual_relres)) <= 2e-15)

    /* The legacy form still solves the first W+F-1 coordinates and infers
       the omitted coordinate.  Supplying a bad last equation must therefore
       distinguish the full certificate from that legacy compatibility path. */
    reduced_rhs = full_rhs[1..(workers+firms-1),.]
    legacy = vckss__fe_solve_matrix(
        design,reduced_rhs,tolerance,maxiter)
    assert(legacy.status == "CONVERGED")
    perturbed_rhs = fit_rhs
    perturbed_rhs[workers+firms] = perturbed_rhs[workers+firms]+1e8
    rejected = vckss__fe_solve_matrix(
        design,perturbed_rhs,tolerance,maxiter)
    assert(rejected.status == "SOLVER_RESIDUAL_FAILED")
    assert(rejected.rhs_status[1] == "SOLVER_RESIDUAL_FAILED")
    assert(rejected.rhs_relres[1] > gate)
    legacy = vckss__fe_solve_matrix(
        design,perturbed_rhs[1..(workers+firms-1)],tolerance,maxiter)
    assert(legacy.status == "CONVERGED")

    rejected = vckss__fe_solve_b0(
        design,perturbed_rhs,tolerance,maxiter)
    assert(rejected.status == "SOLVER_RESIDUAL_FAILED")
    assert(rejected.rhs_relres > gate)

    /* Add one genuinely nonadditive control.  The full joint RHS places the
       control equation after all W+F supplied FE equations. */
    control = (1\0\0\0\0\0\0\0\0)
    joint = vckss__joint_prepare(
        design,control,tolerance,maxiter,rank_tolerance,backend)
    assert(joint.status == "CONVERGED")
    control_rhs = (
        (control'*(frequency:*fit_values))[1,1],
        (control'*(frequency:*leverage_values))[1,1],0,0)
    joint_rhs = full_rhs \ control_rhs
    assert(rows(joint_rhs) == workers+firms+1)
    joint_solved = vckss__joint_solve(
        joint,joint_rhs,tolerance,maxiter)
    assert(joint_solved.status == "CONVERGED")
    assert(min(joint_solved.rhs_status:=="CONVERGED") == 1)
    assert(max(joint_solved.rhs_relres) <= gate)
    manual_relres = kssrhs_oracle__relres(
        kssrhs_oracle__joint_lhs(joint,joint_solved.prediction),joint_rhs)
    assert(max(abs(joint_solved.rhs_relres-manual_relres)) <= 2e-15)

    /* Removing the supplied last-firm row produces the documented legacy
       W+F-1+C layout. */
    reduced_joint_rhs = full_rhs[1..(workers+firms-1),.] \ control_rhs
    joint_legacy = vckss__joint_solve(
        joint,reduced_joint_rhs,tolerance,maxiter)
    assert(joint_legacy.status == "CONVERGED")

    perturbed_joint_rhs = joint_rhs
    perturbed_joint_rhs[workers+firms,1] =
        perturbed_joint_rhs[workers+firms,1]+1e8
    joint_rejected = vckss__joint_solve(
        joint,perturbed_joint_rhs[.,1],tolerance,maxiter)
    assert(joint_rejected.status == "SOLVER_RESIDUAL_FAILED")
    assert(joint_rejected.rhs_status[1] == "SOLVER_RESIDUAL_FAILED")
    assert(joint_rejected.rhs_relres[1] > gate)
    joint_legacy = vckss__joint_solve(
        joint,reduced_joint_rhs[.,1],tolerance,maxiter)
    assert(joint_legacy.status == "CONVERGED")
}

test_full_rhs_certificate()
mata drop kssrhs_oracle__*()
end

di as result "PASS test_full_rhs_certificate.do"
exit 0
