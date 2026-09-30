version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
local backends mata
capture quietly fevc_rust probe
if !_rc {
    if r(numerical_api)==2 local backends mata rust
}
set obs 120
gen long worker=ceil(_n/6)
gen long firm=1+mod(floor((_n-1)/3),2)
replace firm=1 if worker>=19
gen double x=sin(_n*.27)+cos(_n*.81)
gen double y=sin(worker*.5)+firm*.2+.4*x+sin(_n*.93)
gen int copies=1+mod(_n,3)
gen double mass=copies*(1+.1*mod(_n,4))
sort worker firm
local caller_rng `"`c(rngstate)'"'
quietly datasignature
local caller_data `r(datasignature)'
foreach backend of local backends {
    foreach engine in generic compressed {
        local controls
        if "`engine'"=="generic" local controls x
        local common worker(worker) firm(firm) algorithm(jla) backend(`backend') ///
            engine(`engine') stayers(movers) deletion(match) probes(33) batch(7) ///
            targetweight(mass) seed(2026092911) preconditioner(diagonal) nodisplay
        quietly fevc y `controls' [fw=copies], `common' mcse(off)
        matrix point=e(kss)
        assert "`e(mcse_mode)'"=="off" & "`e(numerical_mc_status)'"=="off"
        assert e(numerical_mcse_available)==0 & e(numerical_mcse_all_available)==0
        assert e(mcse_available)==0 & "`e(mcse_method)'"=="none"
        assert "`e(mcse_status)'"=="off"
        mata: assert(all(missing(st_matrix("e(mcse)"))))
        mata: assert(all(missing(st_matrix("e(mcse_cov_raw)"))))
        mata: assert(all(missing(st_matrix("e(mcse_cov)"))))
        mata: assert(all(missing(st_matrix("e(results)")[4,.])))
        capture confirm scalar e(mc_replay_rhs)
        assert _rc
        quietly fevc y `controls' [fw=copies], `common' numericalmcse(off)
        assert "`e(mcse_mode)'"=="off" & "`e(numerical_mc_status)'"=="off"
        assert mreldif(point,e(kss))<1e-11
        foreach mode in default all alias conditional {
            local option
            if "`mode'"=="all" local option mcse(all)
            if "`mode'"=="alias" local option numericalmcse(all)
            if "`mode'"=="conditional" local option mcse(conditional)
            quietly fevc y `controls' [fw=copies], `common' `option'
            assert mreldif(point,e(kss))<1e-11
            assert mreldif(e(mcse),e(numerical_mcse))==0
            matrix selected=e(mcse)
            matrix results=e(results)
            matrix result_mcse=results[4,1..4]
            assert mreldif(selected,result_mcse)==0
            local rows : rownames results
            assert "`rows'"=="plugin bias_correction corrected mcse"
            assert colsof(e(mcse_cov_raw))==4 & rowsof(e(mcse_cov_raw))==4
            if "`mode'"=="conditional" {
                assert "`e(mcse_mode)'"=="conditional"
                assert "`e(numerical_mc_method)'"==""
                assert "`e(mcse_method)'"=="conditional_target_v1"
                assert "`e(mcse_status)'"=="conditional"
                assert e(mcse_available)==e(mcse_conditional_available)
                assert mreldif(e(mcse),e(mcse_conditional))==0
                mata: assert(all(missing(st_matrix("e(mcse_cov_raw)"))))
            }
            else {
                assert "`e(mcse_mode)'"=="all"
                assert "`e(numerical_mc_method)'"=="crossfit_if_v1"
                assert e(mc_replay_rhs)==33
                assert "`e(mcse_method)'"=="crossfit_if_v1"
                assert "`e(mcse_status)'"=="`e(numerical_mc_status)'"
                assert e(mcse_available)==e(numerical_mcse_all_available)
                assert mreldif(e(mcse),e(numerical_mcse_all))==0
                // Independent linear map for the fourth target, vw+vf+2*cov.
                matrix map=(1,0,0 \ 0,1,0 \ 0,0,1 \ 1,1,2)
                matrix expected=map*e(numerical_mccov_all_raw)*map'
                assert mreldif(expected,e(mcse_cov_raw))<1e-11
                if e(mcse_available) {
                    forvalues j=1/4 {
                        assert abs(selected[1,`j']^2-el(e(mcse_cov),`j',`j'))<1e-11
                    }
                }
                if "`mode'"=="default" matrix covariance=e(numerical_mccov_all_raw)
                else assert mreldif(covariance,e(numerical_mccov_all_raw))<1e-11
            }
            assert `"`c(rngstate)'"'==`"`caller_rng'"'
        }
    }
    quietly fevc y x [fw=copies], worker(worker) firm(firm) algorithm(jla) ///
        backend(`backend') engine(generic) stayers(both) deletion(match) ///
        targetweight(mass) probes(33) batch(7) seed(2026092911) nodisplay
    matrix combined=e(results)
    matrix hybrid=e(stayer_hybrid_results)
    local combined_rows : rownames combined
    local hybrid_rows : rownames hybrid
    assert "`combined_rows'"=="plugin bias_correction corrected mcse"
    assert "`hybrid_rows'"=="`combined_rows'"
    assert mreldif(combined,hybrid)==0
    quietly fevc y x, worker(worker) firm(firm) algorithm(exact) backend(`backend') nodisplay
    assert "`e(numerical_mc_status)'"=="exact_zero"
    mata: assert(all(st_matrix("e(numerical_mcse_all)"):==0))
    assert e(mcse_available)==1 & "`e(mcse_method)'"=="exact"
    mata: assert(all(st_matrix("e(mcse)"):==0))
    mata: assert(all(st_matrix("e(mcse_cov_raw)"):==0))
    matrix combined=e(results)
    matrix hybrid=e(stayer_hybrid_results)
    local combined_rows : rownames combined
    local hybrid_rows : rownames hybrid
    assert "`combined_rows'"=="plugin bias_correction corrected mcse"
    assert "`hybrid_rows'"=="`combined_rows'"
    matrix exact_point=e(kss)
    quietly fevc y x, worker(worker) firm(firm) algorithm(exact) backend(`backend') mcse(conditional) nodisplay
    assert "`e(mcse_mode)'"=="conditional" & "`e(mcse_method)'"=="exact"
    assert "`e(mcse_status)'"=="exact_zero" & e(mcse_available)==1
    mata: assert(all(st_matrix("e(mcse)"):==0))
    mata: assert(all(missing(st_matrix("e(mcse_cov_raw)"))))
    assert mreldif(exact_point,e(kss))==0
    quietly fevc y x, worker(worker) firm(firm) algorithm(exact) backend(`backend') mcse(off) nodisplay
    assert "`e(mcse_status)'"=="off" & e(mcse_available)==0
    assert mreldif(exact_point,e(kss))==0
    capture quietly fevc y, worker(worker) firm(firm) backend(`backend') mcse(all) numericalmcse(all) nodisplay
    assert _rc==198 & "`e(status)'"=="WITHHELD"
}
quietly datasignature
assert "`r(datasignature)'"=="`caller_data'"
assert `"`c(rngstate)'"'==`"`caller_rng'"'
assert "$VCKSS_NMC_MODE"=="" & "$VCKSS_NMC_REQUESTED"=="" & "$VCKSS_NMC_EXPLICIT"==""
// A missing optional capability cannot change the selected point backend.
program define _fevc_mcse_stub, eclass
    ereturn clear
    ereturn local algorithm jla
end
program drop fevc_rust
program define fevc_rust, rclass
    return scalar numerical_api=1
end
quietly fevc__numerical init a b c d f g h
quietly fevc__numerical request "" none ""
quietly fevc__numerical native 1 jla
assert "$VCKSS_NMC_MODE"=="off" & "$VCKSS_NMC_UNAVAILABLE"=="unavailable_capability"
quietly _fevc_mcse_stub
quietly fevc__numerical post
assert "`e(mcse_mode)'"=="all" & "`e(numerical_mc_status)'"=="unavailable_capability"
assert e(mcse_available)==0 & "`e(mcse_status)'"=="unavailable_capability"
assert "`e(mcse_method)'"=="crossfit_if_v1"
mata: assert(all(missing(st_matrix("e(mcse)"))))
quietly fevc__numerical clear
quietly fevc__numerical init a b c d f g h
quietly fevc__numerical request all none ""
capture quietly fevc__numerical native 1 jla
assert _rc==498
quietly fevc__numerical clear
program drop fevc_rust _fevc_mcse_stub
clear all
di as result "PASS test_mcse_modes.do"
