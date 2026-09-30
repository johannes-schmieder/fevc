version 18.0
clear
set obs 120
gen long worker=ceil(_n/6)
gen long firm=1+mod(floor((_n-1)/3),2)
replace firm=1 if worker>=19
gen double x=sin(_n*.27)+cos(_n*.81)
gen double z=cos(_n*.33)+sin(_n*.51)
gen double y=sin(worker*.5)+firm*.2+.4*x-.3*z+sin(_n*.93)
gen int f=1+mod(_n,3)
gen double mass=f*(1+.1*mod(_n,4))
gen long probe=_n
sort worker firm probe
local state `"`c(rngstate)'"'
local rng `c(rng)'
local stream `c(rngstream)'
quietly datasignature
local signature `r(datasignature)'
foreach deletion in observation match {
    foreach nuisance in joint fixedoffset {
        quietly fevc y x z [fw=f], worker(worker) firm(firm) ///
            algorithm(jla) backend(mata) engine(generic) deletion(`deletion') ///
            nuisance(`nuisance') targetweight(mass) probes(33) batch(7) ///
            probeorder(probe) preconditioner(diagonal) seed(2026092911) mcse(conditional) nodisplay
        matrix point=e(kss)
        matrix conditional=e(numerical_mcse)
        assert "`e(numerical_mc_method)'"==""
        quietly fevc y x z [fw=f], worker(worker) firm(firm) ///
            algorithm(jla) backend(mata) engine(generic) deletion(`deletion') ///
            nuisance(`nuisance') targetweight(mass) probes(33) batch(7) ///
            probeorder(probe) preconditioner(diagonal) seed(2026092911) ///
            numericalmcse(all) nodisplay
        mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
        mata: assert(vckss_nmc__relative_equal(st_matrix("conditional"),st_matrix("e(numerical_mcse)")))
        assert "`e(numerical_mc_method)'"=="crossfit_if_v1"
        assert e(mc_leverage_probes)==33 & e(mc_target_probes)==33
        assert e(mc_target_fold_a)==17 & e(mc_target_fold_b)==16
        assert e(mc_replay_rhs)==33
        assert e(mc_replay_max_residual)<=max(1e-11,10*e(tolerance))
        capture confirm matrix e(V)
        assert _rc
        assert `"`c(rngstate)'"'==`"`state'"'
        assert "`c(rng)'"=="`rng'" & `c(rngstream)'==`stream'
    }
}
foreach engine in generic compressed {
    quietly fevc y [fw=f], worker(worker) firm(firm) algorithm(jla) ///
        backend(mata) engine(`engine') deletion(match) stayers(movers) ///
        targetweight(mass) probes(33) batch(7) seed(2026092911) mcse(conditional) nodisplay
    matrix point=e(kss)
    matrix conditional=e(numerical_mcse)
    quietly fevc y [fw=f], worker(worker) firm(firm) algorithm(jla) ///
        backend(mata) engine(`engine') deletion(match) stayers(movers) ///
        targetweight(mass) probes(33) batch(7) seed(2026092911) numericalmcse(all) nodisplay
    mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
    mata: assert(vckss_nmc__relative_equal(st_matrix("conditional"),st_matrix("e(numerical_mcse)")))
    assert e(mc_replay_rhs)==33
    assert `"`c(rngstate)'"'==`"`state'"'
}
quietly fevc y, worker(worker) firm(firm) algorithm(exact) backend(mata) ///
    numericalmcse(all) nodisplay
assert "`e(numerical_mc_status)'"=="exact_zero"
assert e(mc_replay_rhs)==0 & e(mc_replay_generator_work)==0
matrix zero=e(numerical_mcse_all)
mata: assert(all(st_matrix("zero"):==0))
capture noisily fevc y, worker(worker) firm(firm) backend(mata) numericalmcse(bad) nodisplay
assert _rc==198
assert "`e(status)'"=="WITHHELD"
capture noisily fevc y, worker(worker) firm(firm) backend(mata) numericalmcse(all) mcse(all) nodisplay
assert _rc==198
assert "`e(status)'"=="WITHHELD"
assert `"`c(rngstate)'"'==`"`state'"'
quietly datasignature
assert "`r(datasignature)'"=="`signature'"
assert "$VCKSS_NMC_MODE"=="" & "$VCKSS_NMC_STATUS"==""
di as result "PASS test_all_probe_frontend.do"

// An older plugin preserves its point route under the implicit default.
capture quietly fevc_rust probe
local probe_rc=_rc
if !`probe_rc' {
if r(numerical_api)<2 {
    local common worker(worker) firm(firm) algorithm(jla) backend(auto) ///
        engine(generic) probes(33) seed(2026092911) nodisplay
    quietly fevc y x z [fw=f], `common' mcse(off)
    matrix point=e(kss)
    local selected `e(backend_selected)'
    quietly fevc y x z [fw=f], `common'
    assert "`e(backend_selected)'"=="`selected'"
    mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
    if "`selected'"=="rust" {
        assert "`e(numerical_mc_status)'"=="unavailable_capability"
        capture quietly fevc y x z [fw=f], `common' mcse(all)
        assert _rc==498 & "`e(withholding_status)'"=="STALE_NUMERICAL_RUNTIME"
    }
    assert `"`c(rngstate)'"'==`"`state'"'
}
}
