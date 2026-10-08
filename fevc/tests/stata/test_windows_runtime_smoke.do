version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

// Portable fixture for the Windows native-loader boundary. The full platform
// profile retains the independent centering and inference oracle suites.
set obs 120
generate long worker=ceil(_n/6)
generate byte firm=1+mod(floor((_n-1)/2),3)
generate int copies=1+mod(_n,3)
generate double y=5+sin(worker*.5)+firm*.2+sin(_n*.93)
generate double shifted=y+40
set seed 721946
local before `"`c(rngstate)'"'
foreach algorithm in exact jla {
    local common worker(worker) firm(firm) backend(rust) algorithm(`algorithm') engine(generic) ///
        deletion(match) stayers(both) rng(counter_v1) probes(64) seed(721946) ///
        preconditioner(auto) tolerance(1e-12) mcse(all) nodisplay
    quietly fevc y [fw=copies], `common'
    assert "`e(centering)'"=="mean"
    assert "`e(backend_selected)'"=="rust"
    assert "`e(status)'"=="KSS_POINT_ESTIMATES_ONLY"
    matrix point=e(kss)
    matrix numerical=e(mcse)
    quietly fevc y [fw=copies], `common' centering(mean)
    assert mreldif(point,e(kss))<1e-12
    assert mreldif(numerical,e(mcse))<1e-12
    quietly fevc shifted [fw=copies], `common' centering(mean)
    assert mreldif(point,e(kss))<1e-8
    assert mreldif(numerical,e(mcse))<1e-8
    assert `"`c(rngstate)'"'==`"`before'"'
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
display as result "FEVC WINDOWS POINT MEAN SMOKE PASS"
