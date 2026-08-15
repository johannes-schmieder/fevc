version 18.0

clear
set more off

// A cycle-like mobility graph needs more than a trivial number of PCG steps
// and gives every RHS the same fixed quotient operator.
local workers = 600
local firms = 120
set obs `=2*`workers''
generate long worker = floor((_n-1)/2) + 1
generate long firm = mod(worker-1,`firms') + 1
replace firm = mod(firm,`firms') + 1 if mod(_n,2) == 0
generate double frequency = 1 + mod(worker+firm,3)

mata:
worker = st_data(. , "worker")
firm = st_data(. , "firm")
frequency = st_data(. , "frequency")
design = kssbc__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")

rseed(20260815)
rhs = rnormal(design.worker_levels+design.firm_levels-1,8,0,1)
rhs[.,8] = J(rows(rhs),1,0)

b0 = kssbc__fe_solve_matrix_b0(design,rhs,1e-10,20000)
b1 = kssbc__fe_solve_matrix(design,rhs,1e-10,20000)
assert(b0.status == "CONVERGED")
assert(b1.status == "CONVERGED")
assert(cols(b1.rhs_iterations) == cols(rhs))
assert(cols(b1.rhs_relres) == cols(rhs))
assert(b1.rhs_iterations[8] == 0)
assert(b1.rhs_relres[8] == 0)
assert(max(b1.rhs_relres) <= 1e-9)
assert(mreldif(b0.coefficient,b1.coefficient) <= 2e-8)

// One lockstep graph traversal serves all still-active columns.  The
// RHS-equivalent action count remains explicit as a separate diagnostic.
assert(b1.schur_batches < b0.schur_batches)
assert(b1.schur_actions >= b1.schur_batches)
assert(b1.preconditioner_applications >= b1.preconditioner_batches)
assert(b1.schur_seconds >= 0)
assert(b1.preconditioner_seconds >= 0)
assert(b1.pcg_seconds >= b1.schur_seconds)
end

di as result "PASS test_lockstep_pcg.do"
