version 18.0
// Hand-constructed transport payloads: failures must retain points but reject corruption.
program define _all_probe_transport_point, eclass
    args algorithm
    ereturn clear
    tempname se
    matrix `se'=(1,1,1,sqrt(6))
    ereturn matrix numerical_mcse=`se'
    ereturn local algorithm `algorithm'
end
_all_probe_transport_point jla
capture mata: vckss_nmc__schema()
if _rc {
    quietly findfile fevc.mata
    quietly do `"`r(fn)'"'
}
tempname cond lev raw usable se meta rhs valid
quietly fevc__numerical init `cond' `lev' `raw' `usable' `se' `meta' `rhs'
matrix `cond'=I(3)
matrix `lev'=-2*I(3)
matrix `raw'=-I(3)
matrix `usable'=J(3,3,.)
matrix `se'=J(1,4,.)
matrix `meta'=(2,2,1,1,2,2,4,4096,0,1,.5,.1,0,.,.)
matrix `rhs'=(1,0,0 \ 1,1,0)
global VCKSS_NMC_STATUS unstable_nonpsd
mata: assert(vckss_nmc__stata_validate(2,1e-9))
matrix `meta'[1,9]=123
mata: assert(!vckss_nmc__stata_validate(2,1e-9))
matrix `meta'[1,9]=0
// A nonfinite derivative discovered after all solves still retains the point.
global VCKSS_NMC_STATUS nonfinite_derivative
matrix `lev'=J(3,3,.)
matrix `raw'=J(3,3,.)
mata: assert(vckss_nmc__stata_validate(2,1e-9))
// No certified replay is a valid empty payload, including first-direction failure.
mata: st_matrix(st_global("VCKSS_NMC_RHS"),J(0,3,.))
matrix `meta'[1,5]=0
matrix `meta'[1,6]=0
global VCKSS_NMC_STATUS nonsmooth_adjustment
mata: assert(vckss_nmc__stata_validate(2,1e-9))
matrix `meta'[1,13]=1e-10
mata: assert(!vckss_nmc__stata_validate(2,1e-9))
matrix `meta'[1,13]=0
matrix `meta'[1,6]=1
matrix `meta'[1,15]=0
global VCKSS_NMC_STATUS replay_failed
global VCKSS_NMC_FAILURE PCG_BREAKDOWN
mata: assert(vckss_nmc__stata_validate(2,1e-9))
// A failed batch can attempt more directions than its certified prefix.
matrix `meta'[1,6]=2
mata: assert(vckss_nmc__stata_validate(2,1e-9))
matrix `meta'[1,6]=3
mata: assert(!vckss_nmc__stata_validate(2,1e-9))
matrix `meta'[1,6]=1
global VCKSS_NMC_FAILURE USER_BREAK
mata: assert(!vckss_nmc__stata_validate(2,1e-9))
global VCKSS_NMC_FAILURE
// Exact receipt corruption cannot turn an iterative request into exact zero.
global VCKSS_NMC_STATUS exact_zero
foreach m in cond lev raw usable {
    matrix ``m''=J(3,3,0)
}
matrix `se'=J(1,4,0)
matrix `meta'=(0,0,0,0,0,0,0,0,0,.,.,.,0,0,.)
mata: assert(!vckss_nmc__stata_validate(2,1e-9))
_all_probe_transport_point exact
mata: assert(vckss_nmc__stata_validate(0,1e-9))
matrix `meta'[1,5]=.
mata: assert(!vckss_nmc__stata_validate(0,1e-9))
quietly fevc__numerical clear
program drop _all_probe_transport_point
di as result "PASS test_all_probe_transport.do"
