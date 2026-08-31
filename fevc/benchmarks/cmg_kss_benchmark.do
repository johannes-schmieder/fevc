version 18.0

clear all
set more off
set varabbrev off

args scenario workers_arg firms_arg rhs_arg seed_arg memory_gib_arg output_dir
local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local rhs_count = real("`rhs_arg'")
local benchmark_seed = real("`seed_arg'")
local memory_gib = real("`memory_gib_arg'")
if !inlist("`scenario'", "easy", "moderate", "weak") | ///
    missing(`workers') | missing(`firms') | missing(`rhs_count') | ///
    missing(`benchmark_seed') | missing(`memory_gib') | ///
    `workers' < 2*`firms' | `firms' < 8 | `rhs_count' < 1 | ///
    `memory_gib' < 1 {
    di as error "invalid KSS/CMG benchmark arguments"
    exit 198
}

capture confirm file "fevc/vckss_cmg.mata"
if _rc exit 601
quietly do "fevc/vckss.mata"
quietly do "fevc/vckss_cmg.mata"
quietly do "fevc/tests/support/vckss_cmg_adapter.mata"

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
design = vckss__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")
rseed(`benchmark_seed')
rhs = rnormal(design.worker_levels+design.firm_levels-1,`rhs_count',0,1)

timer_clear(70)
timer_on(70)
hierarchy = vckss__cmg_hierarchy(
    design,(1::design.worker_levels),(1::design.firm_levels),
    `memory_gib'*1024^3,`rhs_count')
timer_off(70)
setup_seconds = timer_value(70)[1,1]

timer_clear(71)
timer_on(71)
b1 = vckss__fe_solve_matrix(design,rhs,1e-8,20000)
timer_off(71)
b1_seconds = timer_value(71)[1,1]
assert(b1.status == "CONVERGED")
assert(max(b1.rhs_relres) <= 1e-7)

st_local("cmg_status",hierarchy.status)
st_local("cmg_message",hierarchy.message)
timer_clear(72)
timer_on(72)
cmg = vckss__fe_solve_matrix_cmg(
    design,hierarchy,rhs,1e-8,20000)
timer_off(72)
cmg_seconds = timer_value(72)[1,1]
if (hierarchy.status == "CONVERGED") {
    assert(cmg.status == "CONVERGED")
    assert(max(cmg.rhs_relres) <= 1e-7)
    assert(mreldif(b1.coefficient,cmg.coefficient) <= 2e-5)
    benchmark = (`workers',`firms',rows(worker),`rhs_count',`benchmark_seed',
        `memory_gib',1,setup_seconds,b1_seconds,cmg_seconds,
        b1_seconds/cmg_seconds,b1_seconds/(setup_seconds+cmg_seconds),
        b1.iterations,cmg.iterations,max(b1.rhs_relres),max(cmg.rhs_relres),
        b1.schur_seconds,b1.preconditioner_seconds,b1.pcg_seconds,
        cmg.schur_seconds,cmg.preconditioner_seconds,cmg.pcg_seconds,
        b1.schur_actions,cmg.schur_actions,
        b1.preconditioner_applications,cmg.preconditioner_applications,
        hierarchy.n_level,hierarchy.edge_complexity,
        hierarchy.vertex_complexity,hierarchy.structural_bytes,
        hierarchy.dense_factor_bytes,
        mreldif(b1.coefficient,cmg.coefficient))
    st_matrix("cmg_kss_rhs",
        (b1.rhs_iterations',b1.rhs_relres',
         cmg.rhs_iterations',cmg.rhs_relres'))
}
else {
    benchmark = (`workers',`firms',rows(worker),`rhs_count',`benchmark_seed',
        `memory_gib',0,setup_seconds,b1_seconds,.,.,.,
        b1.iterations,.,max(b1.rhs_relres),.,
        b1.schur_seconds,b1.preconditioner_seconds,b1.pcg_seconds,
        .,.,.,b1.schur_actions,.,b1.preconditioner_applications,.,
        hierarchy.n_level,hierarchy.edge_complexity,
        hierarchy.vertex_complexity,hierarchy.structural_bytes,
        hierarchy.dense_factor_bytes,.)
    st_matrix("cmg_kss_rhs",
        (b1.rhs_iterations',b1.rhs_relres',
         J(`rhs_count',2,.)))
}
st_matrix("cmg_kss_benchmark",benchmark)
end

matrix colnames cmg_kss_benchmark = workers firms stored_rows rhs seed ///
    memory_gib cmg_converged setup_seconds b1_seconds cmg_seconds pcg_speedup ///
    setup_inclusive_speedup b1_max_iterations cmg_max_iterations ///
    b1_max_relres cmg_max_relres b1_schur_seconds ///
    b1_precond_seconds b1_pcg_seconds cmg_schur_seconds ///
    cmg_precond_seconds cmg_pcg_seconds b1_schur_actions ///
    cmg_schur_actions b1_precond_applications cmg_precond_applications ///
    hierarchy_levels edge_complexity vertex_complexity structural_bytes ///
    dense_factor_bytes coefficient_mreldif
matrix colnames cmg_kss_rhs = b1_iterations b1_relres ///
    cmg_iterations cmg_relres

preserve
clear
svmat double cmg_kss_benchmark, names(col)
generate str16 scenario = "`scenario'"
generate str32 cmg_status = "`cmg_status'"
generate str80 cmg_message = "`cmg_message'"
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
order scenario cmg_status cmg_message stata_version stata_flavor
export delimited using "`output_dir'/cmg_kss_`scenario'.csv", replace
restore

preserve
clear
svmat double cmg_kss_rhs, names(col)
generate long rhs = _n
generate str16 scenario = "`scenario'"
generate str32 cmg_status = "`cmg_status'"
order scenario cmg_status rhs
export delimited using "`output_dir'/cmg_kss_`scenario'_rhs.csv", replace
restore

matrix list cmg_kss_benchmark
di as result "FEVC FORCED-CMG BENCHMARK COMPLETE: `scenario' (`cmg_status')"
