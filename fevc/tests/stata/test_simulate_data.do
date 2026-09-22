version 18.0

clear
set obs 3
generate long sentinel = _n
quietly _datasignature
local original `"`r(datasignature)'"'
local caller_rng = c(rng)
local caller_state = c(rngstate)
local caller_sortstate = c(sortrngstate)
foreach options in "simulate_data(ex1)" "simulate_data(missing) clear" ///
    "simulate_data(ex1) clear seed(-1)" "simulate_data(ex1) clear seed(1.5)" ///
    "simulate_data(ex1) clear seed(.)" "simulate_data(ex1) clear seed(2147483648)" ///
    "simulate_data(ex1) clear probes(5)" "simulate_data(ex5) clear seed(1)" {
    capture noisily fevc, `options'
    assert _rc != 0
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`original'"'
    assert c(rng)=="`caller_rng'"
    assert c(rngstate)=="`caller_state'"
    assert c(sortrngstate)=="`caller_sortstate'"
}

// The public branch must work before any estimator runtime is loaded.
capture mata: vckss__api_level()
local had_runtime = (_rc==0)
foreach rng in mt64 kiss32 mt64s {
    set rng `rng'
    set seed 142
    local before = c(rngstate)
    local stream = c(rngstream)
    local sort_before = c(sortrngstate)
    quietly fevc, simulate_data(ex2) clear
    assert r(N)==1200 & r(workers)==200 & r(firms)==61
    assert r(seed)==20260820
    assert c(rng)=="`rng'" & c(rngstream)==`stream'
    assert c(rngstate)=="`before'"
    assert c(sortrngstate)=="`sort_before'"
    quietly _datasignature
    local signature `"`r(datasignature)'"'
    if "`rng'"=="mt64" local reference `"`signature'"'
    else assert `"`signature'"'==`"`reference'"'
}
if !`had_runtime' {
    capture mata: vckss__api_level()
    assert _rc != 0
}
set rng `caller_rng'
set rngstate `caller_state'
set sortrngstate `caller_sortstate'

foreach example in ex1 ex2 ex3 ex4 ex5 {
    quietly fevc, simulate_data(`example') clear
    tempname truth oracle
    matrix `truth' = r(truth)
    assert colsof(`truth')==4 & rowsof(`truth')==1
    if "`example'"=="ex1" assert r(N)==60000 & r(workers)==10000 & r(firms)==3001
    if "`example'"=="ex3" assert r(N)==180 & r(workers)==60 & r(firms)==15
    if "`example'"=="ex4" assert r(N)==440 & r(workers)==110 & r(firms)==10
    if "`example'"=="ex5" assert r(N)==24 & r(workers)==6 & r(firms)==4 & missing(r(seed))
    assert !missing(worker_fe,firm_fe,error,log_wage)
    local massvar
    if "`example'"=="ex3" local massvar target_mass
    // Independent centered, weighted Gram oracle for the generated population.
    mata: X = st_data(.,("worker_fe","firm_fe")); w = J(rows(X),1,1)
    if "`massvar'"!="" mata: w = st_data(.,"`massvar'")
    mata: w = w/sum(w); X = X :- (w' * X); C = quadcross(X,w,X)
    mata: st_matrix("`oracle'",(C[1,1],C[2,2],C[1,2],sum(C)))
    mata: assert(max(abs(st_matrix("`truth'")-st_matrix("`oracle'")))<1e-10)
    if inlist("`example'","ex1","ex2") {
        assert abs(log_wage-(2+worker_fe+firm_fe+.30*productivity+.15*(period==2)+error))<1e-12
    }
    quietly _datasignature
    local first `"`r(datasignature)'"'
    quietly fevc, simulate_data(`example') clear
    quietly _datasignature
    assert `"`r(datasignature)'"'==`"`first'"'
}

quietly fevc, simulate_data(ex2) clear seed(123)
assert r(seed)==123
quietly _datasignature
local seeded `"`r(datasignature)'"'
quietly fevc, simulate_data(ex2) clear seed(123)
quietly _datasignature
assert `"`r(datasignature)'"'==`"`seeded'"'
quietly fevc, simulate_data(ex2) clear seed(124)
quietly _datasignature
assert `"`r(datasignature)'"'!=`"`seeded'"'

// Direct generation replaces data; the clickable runner's outer preserve survives.
preserve
quietly _datasignature
local before `"`r(datasignature)'"'
quietly fevc, simulate_data(ex2) clear
quietly fevc log_wage productivity i.period, worker(worker_id) firm(firm_id) algorithm(exact)
assert e(N)==1200
assert e(sample)==1
restore
quietly _datasignature
assert `"`r(datasignature)'"'==`"`before'"'

// Inject a generation failure to exercise rollback after data and RNG mutation.
program drop fevc__simulate_data
findfile fevc__simulate_data.ado
run `"`r(fn)'"'
program drop fevc__simulate_build
program define fevc__simulate_build, rclass
    clear
    set obs 2
    generate damaged = runiform()
    sort damaged
    exit 459
end
local before_state = c(rngstate)
local before_sort = c(sortrngstate)
capture noisily fevc, simulate_data(ex1) clear
assert _rc==459
assert c(rngstate)=="`before_state'"
assert c(sortrngstate)=="`before_sort'"
quietly _datasignature
assert `"`r(datasignature)'"'==`"`before'"'
program drop fevc__simulate_build
program drop fevc__simulate_data

clear
fevc, simulate_data(ex2)
assert _N==1200

di as result "PASS test_simulate_data.do"
