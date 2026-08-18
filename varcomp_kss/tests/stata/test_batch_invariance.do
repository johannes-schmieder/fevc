version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "varcomp_kss/varcomp_kss.ado"
if _rc {
    capture confirm file "../../varcomp_kss.ado"
    if _rc exit 601
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/varcomp_kss"'
adopath ++ `"`pkgroot'"'

set obs 24
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time-1.5
generate double c2 = time==2
generate byte firm = .
generate long match = .
generate double noise = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5+.3*worker-.2*firm+.4*c1-.15*c2+noise
generate long frequency = 1+mod(_n,3)
generate double target = 1+mod(_n,5)/10
generate long observation_key = _n

local probes = 80
local caller_batch = 7
foreach deletion_mode in match observation {
    local delete_options "deletion(`deletion_mode')"
    if "`deletion_mode'" == "match" {
        local delete_options "`delete_options' deletionid(match)"
    }
    foreach nuisance_mode in joint fixedoffset {
        varcomp_kss y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
            `delete_options' targetweight(target) probeorder(observation_key) ///
            algorithm(jla) nuisance(`nuisance_mode') preconditioner(diagonal) ///
            memory_gib(4) ///
            probes(`probes') ///
            batch(1) seed(8675309) tolerance(1e-10) nodisplay
        matrix batch_reference = e(results)
        local rng_reference `"`c(rngstate)'"'
        scalar residual_reference = e(solver_max_residual)

        varcomp_kss y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
            `delete_options' targetweight(target) probeorder(observation_key) ///
            algorithm(jla) nuisance(`nuisance_mode') preconditioner(diagonal) ///
            memory_gib(4) ///
            probes(`probes') ///
            batch(`caller_batch') seed(8675309) tolerance(1e-10) nodisplay
        assert `"`c(rngstate)'"' == `"`rng_reference'"'
        assert mreldif(batch_reference,e(results)) < 1e-14
        assert e(solver_max_residual) <= 1e-10
        assert residual_reference <= 1e-10
        assert abs(el(e(correction),1,4)- ///
            (el(e(correction),1,1)+el(e(correction),1,2)+ ///
            2*el(e(correction),1,3))) < 2e-12

        varcomp_kss y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
            `delete_options' targetweight(target) probeorder(observation_key) ///
            algorithm(jla) nuisance(`nuisance_mode') preconditioner(diagonal) ///
            memory_gib(4) ///
            probes(`probes') ///
            batch(137) seed(8675309) tolerance(1e-10) nodisplay
        assert `"`c(rngstate)'"' == `"`rng_reference'"'
        assert mreldif(batch_reference,e(results)) < 1e-14
        assert e(solver_max_residual) <= 1e-10
    }
}

quietly cd `"`oldpwd'"'
di as result "PASS test_batch_invariance.do"
exit 0
