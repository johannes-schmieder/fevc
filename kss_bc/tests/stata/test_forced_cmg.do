version 18.0

clear
set more off

capture confirm file "shared/cmg/generated/kssbc_cmg_core.mata"
if _rc {
    di as error "run the forced-CMG test from the repository root"
    exit 601
}
capture mata: kssbc__api_level()
if _rc quietly do "kss_bc/kss_bc.mata"
quietly do "shared/cmg/generated/kssbc_cmg_core.mata"
quietly do "kss_bc/tests/support/kss_cmg_adapter.mata"
mata: assert(kssbc_cmg__api_level() == 2)

local workers = 1200
local firms = 300
set obs `=2*`workers''
generate long worker = floor((_n-1)/2) + 1
generate long firm = mod(worker-1,`firms') + 1
replace firm = mod(firm,`firms') + 1 if mod(_n,2) == 0
generate double frequency = 1 + mod(worker+firm,3)

mata:
worker = st_data(.,"worker")
firm = st_data(.,"firm")
frequency = st_data(.,"frequency")
design = kssbc__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")
hierarchy = kssbc__cmg_hierarchy(
    design,(1::design.worker_levels),(1::design.firm_levels),
    4*1024^3,8)
assert(hierarchy.status == "CONVERGED")

rseed(20260815)
rhs = rnormal(design.worker_levels+design.firm_levels-1,8,0,1)
diagonal = kssbc__fe_solve_matrix(design,rhs,1e-8,20000)
cmg = kssbc__fe_solve_matrix_cmg(
    design,hierarchy,rhs,1e-8,20000)
assert(diagonal.status == "CONVERGED")
assert(cmg.status == "CONVERGED")
assert(max(cmg.rhs_relres) <= 1e-7)
assert(mreldif(diagonal.coefficient,cmg.coefficient) <= 2e-5)
assert(cmg.iterations < diagonal.iterations)
assert(cmg.preconditioner_applications > 0)
assert(cmg.preconditioner_seconds > 0)
end

di as result "PASS test_forced_cmg.do"
