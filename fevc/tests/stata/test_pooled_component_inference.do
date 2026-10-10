version 18.0
clear all
set more off
set varabbrev off
args pkgroot backend model
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
if "`backend'"=="" local backend mata
if "`model'"=="" local model structured_leverage
adopath ++ `"`pkgroot'"'
set seed 98132
set obs 256
generate long worker=ceil(_n/16)
generate byte firm=floor(mod(_n-1,16)/2)+1
generate double y=worker*.2-firm*.3+rnormal()
set obs 512
replace worker=17+floor((_n-257)/8) in 257/512
replace firm=mod(worker,8)+1 in 257/512
replace y=worker*.2-firm*.3+rnormal() in 257/512
generate long fw=1+mod(_n,3)
generate double tw=fw*(1+mod(_n,7)/10)
generate double control=sin(_n/7)+mod(_n,5)/3
generate double shifted=y+100
local route backend(mata) algorithm(exact)
if "`backend'"=="rust" local route backend(rust) rng(counter_v1) algorithm(jla) engine(generic) preconditioner(diagonal) probes(200) mcse(off)
local common worker(worker) firm(firm) deletion(match) nuisance(fixedoffset) targetweight(tw) `route' nodisplay
local inference inferencemodel(`model') inferencesimulations(100) inferenceseed(1791)
local rng0 `"`c(rngstate)'"'
fevc y control [fw=fw], `common'
matrix point=e(b)
fevc y control [fw=fw], `common' `inference' inference(highrank)
assert mreldif(point,e(b))<1e-9
assert `"`rng0'"'==`"`c(rngstate)'"'
assert e(inference_mover_units)==128
quietly summarize fw in 257/512, meanonly
assert e(inference_stayer_units)==r(sum)
assert e(inference_independent_units)==128+r(sum)
assert e(inference_nuisance_omitted)==1
assert "`e(inference_model)'"=="`model'"
matrix basepoint=e(b)
matrix baseinterval=e(component_inference)
fevc shifted control [fw=fw], `common' `inference' inference(highrank)
assert mreldif(basepoint,e(b))<1e-9
assert mreldif(baseinterval,e(component_inference))<1e-8
fevc y control [fw=fw], `common' `inference' inference(q1)
capture matrix list e(V)
assert _rc!=0
assert rowsof(e(q1_inference))==4
matrix failure=e(q1_failure_diagnostics)
forvalues row=1/4 {
    assert missing(failure[`row',6]) | failure[`row',6]<1e-8
}
assert `"`rng0'"'==`"`c(rngstate)'"'
// Admission failures preserve data and RNG, and release any native generation.
quietly datasignature
local data0 `"`r(datasignature)'"'
capture noisily fevc y control [fw=fw], `common' `inference' inference(highrank) memory_gib(.000001) memorycheck(error)
assert _rc!=0
assert `"`rng0'"'==`"`c(rngstate)'"'
quietly datasignature
assert `"`data0'"'==`"`r(datasignature)'"'
if "`backend'"=="rust" {
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
    // Design order, thread count and solver choice preserve finite draws.
    preserve
    generate long original_order=_n
    gsort -original_order
    local alternate : subinstr local common "preconditioner(diagonal)" "preconditioner(cmg)"
    fevc y control [fw=fw], `alternate' `inference' inference(highrank) batch(16) nativethreads(2)
    assert mreldif(basepoint,e(b))<1e-8
    assert mreldif(baseinterval,e(component_inference))<1e-8
    restore
    // Old transports must fail before preparation or RNG even on valid data.
    capture program drop fevc__rust_public_call
    program define fevc__rust_public_call, rclass
        gettoken command rest : 0
        fevc_rust `command' `rest'
        return add
        if "`command'"=="probe" return scalar component_mixed_api=0
    end
    capture noisily fevc y control [fw=fw], `common' `inference' inference(highrank)
    assert _rc==498
    assert "`e(withholding_status)'"=="MIXED_COMPONENT_NATIVE_REQUIRED"
    assert `"`rng0'"'==`"`c(rngstate)'"'
    program drop fevc__rust_public_call
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
if "`backend'"=="mata" {
    // Literal expansion must reproduce exact point and the common variance
    // covariance; this fixture fixes original mover blocks through worker/firm.
    replace tw=tw/fw
    expand fw
    replace fw=1
    fevc y control [fw=fw], `common' `inference' inference(highrank)
    assert mreldif(basepoint,e(b))<1e-9
    assert mreldif(baseinterval,e(component_inference))<1e-8
    // Independently form the observation-space residual moment Gram only in
    // this oracle. Production contracts it entirely in coefficient space.
    mata:
    X=st_data(.,("worker","firm"))
    w=X[.,1]; f=X[.,2]; n=rows(X); nw=max(w); nf=max(f)
    X=J(n,nw+nf-1,0)
    for(i=1;i<=n;i++) {
        X[i,w[i]]=1
        if(f[i]<nf) X[i,nw+f[i]]=1
    }
    yy=st_data(.,"y"); ff=J(n,1,1); stay=w:>16
    units_id=(w:-1):*nf+f
    A=invsym(X'*X)
    u=vckss_inf__units(X,yy,yy-X*A*X'*yy,ff,units_id,stay)
    A=invsym(u.design'*u.design)
    P=u.design*A*u.design'; M=I(rows(P))-P
    h=diagonal(P)
    d=(h,h:^2,h:^3)
    fit=vckss_inf__structured(u.design,A,u.residual,h,d,u.mass,
        u.movers,"structured_leverage",1e-10)
    assert(fit.status=="CONVERGED")
    exact=fit.basis'*(M:^2)*fit.basis
    assert(mreldif(exact,fit.gram)<1e-10)
    gamma=qrsolve(exact,fit.basis'*(u.residual:^2))
    assert(mreldif(gamma,fit.coefficients)<1e-9)
    assert(mreldif(vckss_inf__midranks((3\1\1\2)),(.75\-.5\-.5\.25))<1e-14)
    end
}
display as result "FEVC POOLED COMPONENT INFERENCE PASS: `backend'"
