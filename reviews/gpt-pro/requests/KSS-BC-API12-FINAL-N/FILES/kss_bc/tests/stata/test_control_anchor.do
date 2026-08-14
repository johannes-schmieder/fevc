version 18.0
clear
set more off
set varabbrev off

// An exact score at the fuzzy eligibility boundary used by API 11 could
// choose different first anchors after an invertible basis transformation.
// API 12 must fail closed unless both accepted canonical matrices agree.
mata:
Q = (sqrt(.7999999),0 \ 0,sqrt(.8) \ sqrt(.2000001),0 \ 0,sqrt(.2))
T = (1,2 \ 0,1)
frequency = J(4,1,1)
base = kssbc__canonical_controls(Q,frequency,1e-10)
changed = kssbc__canonical_controls(Q*T,frequency,1e-10)
assert(base.status == "AMBIGUOUS_CONTROL_BASIS")
assert(changed.status == "AMBIGUOUS_CONTROL_BASIS")
end

// Reviewer K's compact boundary witness. In native Stata API 11 accepted
// both bases at seed 2 and mreldif(e(results)) was 3.308e-5. The transformed
// basis has determinant one but its anchor decision is not certifiable at the
// registered numerical margin.
set obs 14
generate long worker = 2
replace worker = 1 in 1/4
replace worker = 1 in 9/11
generate byte firm = .
replace firm = 1 in 1/2
replace firm = 2 in 3/4
replace firm = 1 in 5/6
replace firm = 2 in 7/8
replace firm = 3 in 9
replace firm = 4 in 10
replace firm = 5 in 11
replace firm = 3 in 12
replace firm = 4 in 13
replace firm = 5 in 14
generate long frequency = 1
replace frequency = 8 in 9/10
replace frequency = 11 in 11
replace frequency = 14 in 12
replace frequency = 2 in 13
replace frequency = 3 in 14
generate double z1 = 0
generate double z2 = 0
local z1s .7037651560862372 -.2552775470643382 ///
    -.1128966060152190 -.26212880006292877 -.08287286955710015 ///
    -.06938638543245550 .08296261314678174 -.5826882952311562
local z2s -.01213100064390886 .6559466556462534 ///
    .28729193595054625 -.15584457130166096 -.1733117487431483 ///
    -.0772968683597004 -.5616086968926074 -.33368629034981906
forvalues row = 1/8 {
    local value : word `row' of `z1s'
    quietly replace z1 = `value' in `row'
    local value : word `row' of `z2s'
    quietly replace z2 = `value' in `row'
}
generate double u1 = 8000*z1
generate double u2 = z1 + .000125*z2
generate double y = 10*(_n-1) + sin(_n-1)

mata:
Q = st_data(.,("z1","z2"))
QT = st_data(.,("u1","u2"))
frequency = st_data(.,"frequency")
base = kssbc__canonical_controls(Q,frequency,1e-10)
changed = kssbc__canonical_controls(QT,frequency,1e-10)
assert(base.status == "CONVERGED")
assert(changed.status == "AMBIGUOUS_CONTROL_BASIS")
end

foreach nuisance_mode in joint fixedoffset {
    capture noisily kss_bc y z1 z2 [fw=frequency], worker(worker) ///
        firm(firm) deletion(observation) algorithm(exact) ///
        nuisance(`nuisance_mode') nodisplay
    assert _rc == 0
    capture noisily kss_bc y u1 u2 [fw=frequency], worker(worker) ///
        firm(firm) deletion(observation) algorithm(exact) ///
        nuisance(`nuisance_mode') nodisplay
    assert _rc == 498
    assert "`e(withholding_status)'" == "AMBIGUOUS_CONTROL_BASIS"

    foreach selected_algorithm in jla auto {
        local dispatch "algorithm(`selected_algorithm')"
        if "`selected_algorithm'" == "auto" {
            local dispatch "`dispatch' exact_limit(2)"
        }
        foreach batch_size in 1 2 {
            capture noisily kss_bc y z1 z2 [fw=frequency], ///
                worker(worker) firm(firm) deletion(observation) ///
                `dispatch' nuisance(`nuisance_mode') probes(2) ///
                batch(`batch_size') seed(2) tolerance(1e-4) nodisplay
            assert _rc == 0
            assert "`e(algorithm)'" == "jla"

            capture noisily kss_bc y u1 u2 [fw=frequency], ///
                worker(worker) firm(firm) deletion(observation) ///
                `dispatch' nuisance(`nuisance_mode') probes(2) ///
                batch(`batch_size') seed(2) tolerance(1e-4) nodisplay
            assert _rc == 498
            assert "`e(withholding_status)'" == "AMBIGUOUS_CONTROL_BASIS"
        }
    }
}

di as result "PASS test_control_anchor.do"
