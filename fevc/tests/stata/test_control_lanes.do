version 18.0
clear all
set more off
args pkgroot
if `"`pkgroot'"' == "" local pkgroot "`c(pwd)'/fevc"
adopath ++ `"`pkgroot'"'
quietly do `"`pkgroot'/fevc.mata"'
quietly do `"`pkgroot'/fevc_resource.mata"'

// Independent allocation counts, including lane boundaries and q=0.
mata:
void control_lane_memory_test()
{
    real scalar n, q, lanes, expected, j
    real matrix x, transform
    real colvector frequency, copies
    struct vckss_control_basis_result scalar b, reference
    struct vckss_resource_model scalar resource
    for (n=1;n<=1025;n++) for (q=0;q<=32;q++) {
        lanes=min((n,256))
        assert(ceil(n/lanes)+2*lanes+1<=4*n)
        expected=0
        if (q>0) expected=8*n*(24+6*q)+128*8*q*q+
            16*8*lanes*q*q+8*8*lanes*q
        assert(vckss__control_prep_bytes(n,q)==expected)
        assert(vckss_resource__control_bytes(n,q)==expected)
    }
    // At batch one, control preparation dominates the generic solver buffers.
    resource=vckss_resource__model(100000,100000,50000,100000,50000,
        5000,500,5531,20,1,1,100000,100000,0,8*100000,2^40,.)
    assert(resource.status=="MODELED")
    assert(resource.generic_components.phase_scratch_bytes==
        vckss__control_prep_bytes(100000,32))
    x=J(32,1,1)#I(32)
    expected=vckss__control_prep_bytes(rows(x),cols(x))
    st_global("VCKSS_MEMORY_ACTIVE","1")
    st_global("VCKSS_MEMORY_ADVISORY","0")
    st_global("VCKSS_MEMORY_FORECAST","0")
    st_global("VCKSS_MEMORY_BYTES",strofreal(expected-1,"%21.0f"))
    b=vckss__canonical_controls(x,J(rows(x),1,1),1e-10)
    assert(b.status=="RESOURCE_LIMIT")
    assert(strtoreal(st_global("VCKSS_MEMORY_FORECAST"))==expected)
    st_global("VCKSS_MEMORY_BYTES",strofreal(expected,"%21.0f"))
    b=vckss__canonical_controls(x,J(rows(x),1,1),1e-10)
    assert(b.status=="CONVERGED")
    st_global("VCKSS_MEMORY_BYTES","1")
    st_global("VCKSS_MEMORY_ADVISORY","1")
    b=vckss__canonical_controls(x,J(rows(x),1,1),1e-10)
    assert(b.status=="CONVERGED")
    // Full supported rank, multiple blocks, a mixed nonsingular coordinate
    // change, and literal-copy expansion of an independently counted weight.
    frequency=1:+mod((1::rows(x)),3)
    reference=vckss__canonical_controls(x,frequency,1e-10)
    assert(reference.status=="CONVERGED")
    transform=I(32)
    for (j=1;j<=32;j++) transform[j,1+mod(j,32)]=.125
    b=vckss__canonical_controls(x*transform,frequency,1e-10)
    assert(b.status=="CONVERGED")
    assert(max(abs(b.controls-reference.controls))<=1e-8)
    b=vckss__canonical_controls(-x[,32..1],frequency,1e-10)
    assert(b.status=="CONVERGED")
    assert(max(abs(b.controls-reference.controls))<=1e-8)
    copies=J(0,1,.)
    for (j=1;j<=rows(x);j++) copies=copies\J(frequency[j],1,j)
    b=vckss__canonical_controls(x[copies,.],J(rows(copies),1,1),1e-10)
    assert(b.status=="CONVERGED")
    assert(max(abs(b.controls-reference.controls[copies,.]))<=1e-8)
}
control_lane_memory_test()
end
foreach key in ACTIVE ADVISORY FORECAST BYTES {
    macro drop VCKSS_MEMORY_`key'
}

// More than two lane blocks, positive integer weights, and physical copies.
set obs 600
generate long worker=ceil(_n/6)
generate long firm=1+mod(worker+7*floor(mod(_n-1,6)/2),31)
generate double z1=sin(.97*_n)
generate double z2=cos(.53*_n)
generate double y=.4*sin(worker)+.2*cos(firm)+.3*z1-.2*z2+sin(1.77*_n)
generate byte frequency=1+mod(_n,3)
generate double u1=z1-.5*z2
generate double u2=.25*z1+2*z2
generate double minus_z1=-z1
generate long order_key=_n
sort order_key
local caller_rng `"`c(rngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach method in exact jla {
    foreach deletion in observation match {
        local options worker(worker) firm(firm) backend(mata) stayers(movers) ///
            algorithm(`method') deletion(`deletion') probes(100) seed(377) nodisplay
        fevc y z1 z2 [fw=frequency], `options'
        matrix reference=e(results)
        assert e(N_stored)==600 & e(N)==1200 & e(N_physical)==1200
        if "`method'"=="jla" {
            assert e(complete_residual_max)<.
            assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
        }
        else assert e(inverse_relres)<1e-8
        foreach controls in "z2 z1" "minus_z1 z2" "u1 u2" {
            fevc y `controls' [fw=frequency], `options'
            mata: a=st_matrix("reference"); b=st_matrix("e(results)")
            mata: assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),.1*rowmax((a[4,.]',b[4,.]'))))'))
            assert e(N_stored)==600 & e(N)==1200 & e(N_physical)==1200
        }
        preserve
        gsort -order_key
        fevc y z1 z2 [fw=frequency], `options'
        mata: assert(max(abs(st_matrix("reference")[3,.]-st_matrix("e(results)")[3,.]))<=1e-8)
        expand frequency
        fevc y z1 z2, `options'
        mata: assert(max(abs(st_matrix("reference")[3,.]-st_matrix("e(results)")[3,.]))<=1e-8)
        assert e(N)==1200 & e(N_physical)==1200
        restore
    }
}
assert `"`c(rngstate)'"'==`"`caller_rng'"'
local sortedby : sortedby
assert "`sortedby'"=="order_key"
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
assert "$VCKSS_MEMORY_ACTIVE"=="" & "$VCKSS_MEMORY_FORECAST"==""

// An interrupt during the first compensated block must take the ordinary
// UserBreak cleanup path, including clearing the prior successful estimate.
mata: mata drop vckss__control_sum_step()
mata:
void vckss__control_sum_step(real matrix total, real matrix correction,
    real matrix term)
{
    _error(1)
}
end
foreach method in exact jla {
    foreach deletion in observation match {
        capture noisily fevc y z1 z2 [fw=frequency], worker(worker) firm(firm) ///
            backend(mata) stayers(movers) algorithm(`method') ///
            deletion(`deletion') probes(100) nodisplay
        assert _rc==1
        assert "`e(cmd)'"==""
        assert `"`c(rngstate)'"'==`"`caller_rng'"'
        local sortedby : sortedby
        assert "`sortedby'"=="order_key"
        quietly _datasignature
        assert `"`r(datasignature)'"'==`"`signature'"'
        assert "$VCKSS_MEMORY_ACTIVE"=="" & "$VCKSS_MEMORY_FORECAST"==""
        assert "$VCKSS_STAGE_SELECTION_TIMER"=="" & "$VCKSS_STAGE_VALIDATION_TIMER"==""
    }
}
mata: mata clear
quietly do `"`pkgroot'/tests/stata/test_load.do"'
display "PASS test_control_lanes.do"
