version 18.0
clear all
set more off

capture confirm file "kss_bc/kss_bc.mata"
if _rc {
    capture confirm file "../../kss_bc.mata"
    if _rc exit 601
    quietly do "../../kss_bc.mata"
}
else quietly do "kss_bc/kss_bc.mata"

set obs 800
generate long worker = floor((_n-1)/2)+1
generate long firm = mod(worker-1,80)+1
replace firm = mod(firm,80)+1 if mod(_n,2)==0
generate double frequency = 1+mod(worker+firm,3)

mata:
struct kssbc_preconditioner_result scalar test__exact_apply(
    pointer scalar context,
    struct kssbc_fe_design scalar design,
    real matrix residual)
{
    struct kssbc_preconditioner_result scalar out
    real matrix laplacian, quotient_inverse

    context = context
    out.status = "CONVERGED"
    out.message = "test exact quotient inverse"
    laplacian = kssbc__fe_schur_action(design,I(design.firm_levels))
    quotient_inverse = invsym(laplacian+
        J(design.firm_levels,design.firm_levels,1/design.firm_levels))
    out.value = quotient_inverse*residual
    out.value = out.value-
        J(design.firm_levels,1,1)*(colsum(out.value):/design.firm_levels)
    return(out)
}

struct kssbc_solver_backend scalar test__exact_backend(
    string scalar route,
    pointer scalar apply)
{
    struct kssbc_solver_backend scalar out

    out.route = route
    out.context = NULL
    out.apply = apply
    out.exact_inverse = 1
    return(out)
}

struct kssbc_preconditioner_result scalar test__false_exact_apply(
    pointer scalar context,
    struct kssbc_fe_design scalar design,
    real matrix residual)
{
    struct kssbc_preconditioner_result scalar out

    context = context
    design = design
    out.status = "CONVERGED"
    out.message = "deliberately false exact claim"
    out.value = residual
    return(out)
}

worker = st_data(.,"worker")
firm = st_data(.,"firm")
frequency = st_data(.,"frequency")
design = kssbc__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")
rseed(20260815)
rhs = rnormal(design.worker_levels+design.firm_levels-1,6,0,1)

backend = test__exact_backend("TEST_EXACT",&test__exact_apply())
direct = kssbc__fe_solve_matrix_backend(design,rhs,1e-10,20000,backend)
ordinary = kssbc__fe_solve_matrix(design,rhs,1e-10,20000)
assert(direct.status == "CONVERGED")
assert(ordinary.status == "CONVERGED")
assert(max(direct.rhs_iterations) == 1)
assert(direct.schur_batches == 0)
assert(direct.preconditioner_batches == 1)
assert(max(direct.rhs_relres) <= 1e-10)
assert(mreldif(direct.coefficient,ordinary.coefficient) <= 2e-9)
assert(mreldif(direct.prediction,
    kssbc__fe_predict(design,direct.coefficient)) < 1e-14)

backend = test__exact_backend("TEST_FALSE_EXACT",&test__false_exact_apply())
rejected = kssbc__fe_solve_matrix_backend(
    design,rhs,1e-10,20000,backend)
assert(rejected.status == "SOLVER_RESIDUAL_FAILED")
assert(rows(rejected.prediction) == 0)
end

di as result "PASS test_perf_exact_terminal.do"
exit 0
