version 18.0
clear all
set more off
args package_dir
if `"`package_dir'"'=="" exit 198
adopath ++ `"`package_dir'"'
fevc_rust probe
assert r(numerical_api)==2
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
            algorithm(jla) backend(rust) engine(generic) deletion(`deletion') ///
            nuisance(`nuisance') targetweight(mass) probes(33) batch(7) ///
            stayers(both) preconditioner(diagonal) seed(2026092911) mcse(conditional) nodisplay
        matrix point=e(kss)
        matrix conditional=e(numerical_mcse)
        assert "`e(numerical_mc_method)'"==""
        quietly fevc y x z [fw=f], worker(worker) firm(firm) ///
            algorithm(jla) backend(rust) engine(generic) deletion(`deletion') ///
            nuisance(`nuisance') targetweight(mass) probes(33) batch(7) ///
            stayers(both) preconditioner(diagonal) seed(2026092911) ///
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
foreach engine in generic {
    quietly fevc y [fw=f], worker(worker) firm(firm) algorithm(jla) ///
        backend(rust) engine(auto) deletion(match) stayers(both) ///
        targetweight(mass) probes(33) batch(auto) seed(2026092911) mcse(conditional) nodisplay
    matrix point=e(kss)
    matrix conditional=e(numerical_mcse)
    quietly fevc y [fw=f], worker(worker) firm(firm) algorithm(jla) ///
        backend(rust) engine(auto) deletion(match) stayers(both) ///
        targetweight(mass) probes(33) batch(auto) seed(2026092911) numericalmcse(all) nodisplay
    mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
    mata: assert(vckss_nmc__relative_equal(st_matrix("conditional"),st_matrix("e(numerical_mcse)")))
    assert e(mc_replay_rhs)==33
    assert `"`c(rngstate)'"'==`"`state'"'
}
quietly fevc y, worker(worker) firm(firm) algorithm(exact) backend(rust) ///
    numericalmcse(all) nodisplay
assert "`e(numerical_mc_status)'"=="exact_zero"
assert e(mc_replay_rhs)==0 & e(mc_replay_generator_work)==0
matrix zero=e(numerical_mcse_all)
mata: assert(all(st_matrix("zero"):==0))
capture noisily fevc y, worker(worker) firm(firm) backend(rust) numericalmcse(bad) nodisplay
assert _rc==198
assert "`e(status)'"=="WITHHELD"
capture noisily fevc y, worker(worker) firm(firm) backend(rust) numericalmcse(all) mcse(all) nodisplay
assert _rc==198
assert "`e(status)'"=="WITHHELD"
assert `"`c(rngstate)'"'==`"`state'"'
quietly datasignature
assert "`r(datasignature)'"=="`signature'"
assert "$VCKSS_NMC_MODE"=="" & "$VCKSS_NMC_STATUS"==""
di as result "PASS test_all_probe_native_frontend.do"

foreach route in auto cmg {
    quietly fevc y x z [fw=f], worker(worker) firm(firm) algorithm(jla) backend(rust) ///
        engine(generic) preconditioner(`route') batch(auto) probes(33) seed(2026092911) mcse(conditional) nodisplay
    matrix point=e(kss)
    matrix conditional=e(numerical_mcse)
    local selected `e(preconditioner_selected)'
    quietly fevc y x z [fw=f], worker(worker) firm(firm) algorithm(jla) backend(rust) ///
        engine(generic) preconditioner(`route') batch(auto) probes(33) seed(2026092911) numericalmcse(all) nodisplay
    mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
    mata: assert(vckss_nmc__relative_equal(st_matrix("conditional"),st_matrix("e(numerical_mcse)")))
    assert "`e(preconditioner_selected)'"=="`selected'"
    assert e(mc_replay_rhs)==33 & e(mc_replay_executed_rhs)==33
    assert "`e(full_cmg_model_schema)'"=="CMG-FULL-NUMERICAL-MODEL-V1"
}
foreach limit in 2 500 {
    quietly fevc y x z [fw=f], worker(worker) firm(firm) algorithm(auto) backend(rust) ///
        exact_limit(`limit') engine(auto) batch(auto) probes(33) seed(2026092911) mcse(conditional) nodisplay
    matrix point=e(kss)
    local algorithm `e(algorithm)'
    quietly fevc y x z [fw=f], worker(worker) firm(firm) algorithm(auto) backend(rust) ///
        exact_limit(`limit') engine(auto) batch(auto) probes(33) seed(2026092911) numericalmcse(all) nodisplay
    mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
    assert "`e(algorithm)'"=="`algorithm'"
    if `limit'==500 assert "`e(numerical_mc_status)'"=="exact_zero"
    else assert e(mc_replay_rhs)==33
}
assert `"`c(rngstate)'"'==`"`state'"'
foreach route in auto cmg {
    foreach nuisance in joint fixedoffset {
        quietly fevc y [fw=f], worker(worker) firm(firm) algorithm(jla) backend(rust) ///
            engine(auto) stayers(movers) deletion(match) nuisance(`nuisance') ///
            preconditioner(`route') batch(7) probes(33) seed(2026092911) mcse(conditional) nodisplay
        matrix point=e(kss)
        matrix conditional=e(numerical_mcse)
        local selected `e(preconditioner_selected)'
        assert "`e(engine_selected)'"=="compressed"
        quietly fevc y [fw=f], worker(worker) firm(firm) algorithm(jla) backend(rust) ///
            engine(auto) stayers(movers) deletion(match) nuisance(`nuisance') ///
            preconditioner(`route') batch(7) probes(33) seed(2026092911) numericalmcse(all) nodisplay
        mata: assert(vckss_nmc__relative_equal(st_matrix("point"),st_matrix("e(kss)")))
        mata: assert(vckss_nmc__relative_equal(st_matrix("conditional"),st_matrix("e(numerical_mcse)")))
        assert "`e(preconditioner_selected)'"=="`selected'"
        assert "`e(engine_selected)'"=="compressed" & e(mc_replay_rhs)==33
    }
}
fevc_rust snapshot
assert r(state)==0
estat diagnostics
foreach name in status executed error point_rhs {
    capture confirm scalar __vckss_nmc_`name'
    assert _rc
}
di as result "FEVC ALL PROBE NATIVE FRONTEND PASS"
