version 18.0

clear all
set more off
set varabbrev off

args scenario workers_arg firms_arg rhs_arg seed_arg output_dir
local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local rhs_count = real("`rhs_arg'")
local benchmark_seed = real("`seed_arg'")

if !inlist("`scenario'", "easy", "moderate", "weak") | ///
    missing(`workers') | missing(`firms') | missing(`rhs_count') | ///
    missing(`benchmark_seed') | `workers' < 2*`firms' | `firms' < 8 | ///
    `rhs_count' < 1 {
    di as error "invalid lockstep benchmark arguments"
    exit 198
}

capture confirm file "kss_bc/kss_bc.mata"
if _rc {
    di as error "run lockstep_solver_benchmark.do from the source root"
    exit 601
}
quietly do "kss_bc/kss_bc.mata"

local degree = cond("`scenario'" == "easy", 4, ///
    cond("`scenario'" == "moderate", 3, 2))
set seed `benchmark_seed'
set obs `=`degree'*`workers''
generate long worker = floor((_n-1)/`degree') + 1
generate byte link = mod(_n-1,`degree')
generate long firm = .
if "`scenario'" == "weak" {
    replace firm = mod(worker-1+link,`firms') + 1
}
else if "`scenario'" == "moderate" {
    replace firm = mod(worker-1+cond(link==2,17,link),`firms') + 1
}
else {
    replace firm = mod(worker-1,`firms') + 1 if link == 0
    replace firm = runiformint(1,`firms') if link > 0
}
generate double frequency = 1 + mod(worker+firm+link,3)

mata:
worker = st_data(.,"worker")
firm = st_data(.,"firm")
frequency = st_data(.,"frequency")
design = kssbc__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")
rseed(`benchmark_seed')
rhs = rnormal(design.worker_levels+design.firm_levels-1,`rhs_count',0,1)

timer_clear(70)
timer_on(70)
b0 = kssbc__fe_solve_matrix_b0(design,rhs,1e-8,20000)
timer_off(70)
b0_seconds = timer_value(70)[1,1]

timer_clear(71)
timer_on(71)
b1 = kssbc__fe_solve_matrix(design,rhs,1e-8,20000)
timer_off(71)
b1_seconds = timer_value(71)[1,1]

assert(b0.status == "CONVERGED")
assert(b1.status == "CONVERGED")
assert(max(b0.rhs_relres) <= 1e-7)
assert(max(b1.rhs_relres) <= 1e-7)
assert(mreldif(b0.coefficient,b1.coefficient) <= 2e-5)
benchmark = (`workers',`firms',rows(worker),`rhs_count',`benchmark_seed',
    b0_seconds,b1_seconds,b0_seconds/b1_seconds,
    b0.iterations,b1.iterations,max(b0.rhs_relres),max(b1.rhs_relres),
    b0.schur_batches,b1.schur_batches,b1.schur_actions,
    b1.preconditioner_applications,b1.schur_seconds,
    b1.preconditioner_seconds,b1.pcg_seconds,
    mreldif(b0.coefficient,b1.coefficient))
st_matrix("lockstep_benchmark",benchmark)
st_matrix("lockstep_rhs",
    (b0.rhs_iterations' , b0.rhs_relres' ,
     b1.rhs_iterations' , b1.rhs_relres'))
end

matrix colnames lockstep_benchmark = workers firms stored_rows rhs seed ///
    b0_seconds b1_seconds speedup b0_max_iterations b1_max_iterations ///
    b0_max_relres b1_max_relres b0_schur_batches b1_schur_batches ///
    b1_schur_actions b1_precond_applications b1_schur_seconds ///
    b1_precond_seconds b1_pcg_seconds coefficient_mreldif
matrix colnames lockstep_rhs = b0_iterations b0_relres b1_iterations b1_relres

preserve
clear
svmat double lockstep_benchmark, names(col)
generate str16 scenario = "`scenario'"
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
order scenario stata_version stata_flavor
export delimited using "`output_dir'/lockstep_`scenario'.csv", replace
restore

preserve
clear
svmat double lockstep_rhs, names(col)
generate long rhs = _n
generate str16 scenario = "`scenario'"
order scenario rhs
export delimited using "`output_dir'/lockstep_`scenario'_rhs.csv", replace
restore

matrix list lockstep_benchmark
di as result "KSS_BC LOCKSTEP BENCHMARK PASS: `scenario'"
