version 18
clear
set more off
args package_dir profile plugindir
if `"`package_dir'"'=="" local package_dir `"`c(pwd)'/fevc"'
adopath ++ `"`package_dir'"'
if `"`plugindir'"'!="" adopath ++ `"`plugindir'"'
set obs 40
gen long worker=ceil(_n/6)
gen long firm=mod(_n-1,3)+1
gen long actualfirm=firm
gen long t=mod(_n-1,6)+1
replace worker=99 in 37/40
replace firm=1 in 37/40
replace actualfirm=10 in 37/38
replace actualfirm=11 in 39/40
replace t=_n-36 in 37/40
egen long match=group(worker actualfirm)
gen double y=.2*worker-.15*firm+sin(_n)/10
gen double target=1/6
replace target=1/4 if worker==99
fevc y, worker(worker) firm(firm) deletion(match) deletionid(match) ///
    stayers(movers) nuisance(joint) targetweight(target) algorithm(exact) ///
    backend(mata) rng(stata) nodisplay
gen byte km=e(sample)
assert km==1 if worker==99
assert km==1 if worker!=99
fevc y, worker(worker) firm(firm) deletion(match) deletionid(match) ///
    stayers(both) nuisance(joint) targetweight(target) algorithm(exact) ///
    backend(mata) rng(stata) nodisplay
assert e(sample)==1
assert e(stayer_hybrid_N_stayers)==0
assert e(deletion_units)==20
matrix fk=e(kss)
mata:
w=st_data(.,"worker"); f=st_data(.,"firm"); d=st_data(.,"match"); y=st_data(.,"y"); p=st_data(.,"target");p=p/sum(p)
ids=uniqrows(sort(w,1)); X=J(rows(w),rows(ids)+2,0)
for(i=1;i<=rows(ids);i++) X[.,i]=w:==ids[i]
X[.,rows(ids)+1]=f:==2;X[.,rows(ids)+2]=f:==3
RA=X;RA[.,(rows(ids)+1)..cols(X)]=J(rows(X),2,0)
RF=X-RA; H=diag(p)-p*p';IQ=invsym(cross(X,X));th=IQ*cross(X,y)
AA=RA'*H*RA; FF=RF'*H*RF; AF=(RA'*H*RF+RF'*H*RA)/2
A=(vec(AA),vec(FF),vec(AF)); plugin=J(1,3,0);bc=J(1,3,0)
for(k=1;k<=3;k++){
 Q=colshape(A[.,k],cols(X));plugin[k]=th'*Q*th
 ids=uniqrows(sort(d,1));b=0
 for(i=1;i<=rows(ids);i++){
  use=selectindex(d:!=ids[i]); m=selectindex(d:==ids[i]);Xm=X[m,.]
  assert(rank(X[use,.])==cols(X))
  tm=qrsolve(X[use,.],y[use]);r=y[m]-Xm*tm
  b=b+y[m]'*(Xm*IQ*Q*IQ*Xm')*r
 }
 bc[k]=plugin[k]-b
}
st_matrix("oracle",bc);st_numscalar("gap",max(abs(bc-st_matrix("fk")[1,1..3])))
end
matrix list oracle
matrix list fk
assert scalar(gap)<1e-8

display "PASS pooled original reproducer"


// This independent oracle expands literal copies and refits the deleted
// design with QR. It uses neither FEVC block inverse actions nor graph code.
capture mata: mata drop pooled_oracle()
mata:
real rowvector pooled_oracle(string scalar controls, string scalar nuisance,
    string scalar samplevar, string scalar deletionvar, string scalar stayervar)
{
    real colvector ix, w, f, d, y, a, freq, ids, copy, u, b, keep, block, yy, px
    real matrix X, D, F, Z, H, V, Q, A, coef, PZ, L, score, PV, sy, se
    real rowvector plugin, correction
    real scalar i, j, n, k, p
    ix=selectindex(st_data(.,samplevar):==1)
    freq=st_data(ix,"frequency")
    copy=J(sum(freq),1,.)
    k=1
    for(i=1;i<=rows(ix);i++) {
        copy[|k\k+freq[i]-1|]=J(freq[i],1,i); k=k+freq[i]
    }
    // Import stored observations once. Expand in Mata, not by passing
    // repeated observation indices to Stata's data-import interface.
    w=st_data(ix,"worker"); w=w[copy]
    f=st_data(ix,"firm"); f=f[copy]
    d=st_data(ix,deletionvar); d=d[copy]
    y=st_data(ix,"y"); y=y[copy]
    a=st_data(ix,"target"):/freq; a=a[copy]; a=a/sum(a)
    px=st_data(ix,"x"); px=px[copy]
    if(stayervar!="") {
        u=st_data(ix,stayervar); u=u[copy]
        for(i=1;i<=rows(d);i++) if(u[i]) d[i]=max(d)+1
    }
    n=rows(w); ids=uniqrows(sort(w,1)); D=J(n,rows(ids),0)
    for(i=1;i<=rows(ids);i++) D[.,i]=w:==ids[i]
    ids=uniqrows(sort(f,1)); F=J(n,rows(ids)-1,0)
    // Match the documented last-firm display normalization for the intercept.
    for(i=1;i<rows(ids);i++) F[.,i]=f:==ids[i]
    Z=controls=="" ? J(n,0,0) : st_data(ix,tokens(controls))
    if(cols(Z)>0) Z=Z[copy,.]
    X=D,F,Z
    assert(rank(X)==cols(X))
    if(nuisance=="fixedoffset" & cols(Z)>0) {
        coef=qrsolve(X,y); y=y-Z*coef[(cols(D)+cols(F)+1)..cols(X)]
        X=D,F; Z=J(n,0,0)
    }
    p=cols(X); D=D,J(n,cols(F)+cols(Z),0)
    F=J(n,p-cols(F)-cols(Z),0),F,J(n,cols(Z),0)
    H=diag(a)-a*a'; V=invsym(cross(X,X)); b=qrsolve(X,y)
    A=vec(D'*H*D),vec(F'*H*F),vec((D'*H*F+F'*H*D)/2)
    A=A,A[.,1]+A[.,2]+2*A[.,3]
    plugin=correction=J(1,4,0)
    ids=uniqrows(sort(d,1))
    for(j=1;j<=4;j++) {
        Q=colshape(A[.,j],p); plugin[j]=b'*Q*b
        for(i=1;i<=rows(ids);i++) {
            keep=selectindex(d:!=ids[i]); block=selectindex(d:==ids[i])
            assert(rank(X[keep,.])==p)
            yy=y[block]-X[block,.]*qrsolve(X[keep,.],y[keep])
            correction[j]=correction[j]+y[block]'*X[block,.]*V*Q*V*X[block,.]'*yy
        }
    }
    // Independent target-weighted worker-effect projection and block covariance.
    PZ=J(n,1,1),px
    L=D'*(a:*PZ)*invsym(PZ'*(a:*PZ)); score=X*V*L; PV=J(2,2,0)
    for(i=1;i<=rows(ids);i++) {
        keep=selectindex(d:!=ids[i]); block=selectindex(d:==ids[i])
        yy=y[block]-X[block,.]*qrsolve(X[keep,.],y[keep])
        sy=score[block,.]'*y[block]; se=score[block,.]'*yy
        PV=PV+(sy*se'+se*sy')/2
    }
    st_matrix("literal_projection_b",(L'*b)')
    st_matrix("literal_projection_V",PV)
    st_numscalar("projection_psd",min(symeigenvalues(PV))>=0 & min(diagonal(PV))>0)
    st_matrix("literal_plugin",plugin)
    st_matrix("literal_correction",correction)
    return(plugin-correction)
}
end

// Unequal masses, physical frequencies, within-block controls, repeated
// noncontiguous spells, original one-block stayers, and a graph-dropped mover.
set obs 52
replace worker=100 in 41/43
replace firm=1 in 41/43
replace match=100 in 41/43
replace worker=101 in 44
replace firm=2 in 44
replace match=101 in 44
replace worker=102 in 45/48
replace firm=9 in 45/48
replace match=102+(_n>46) in 45/48
replace worker=103 in 49/52
replace firm=cond(_n<51,1,9) in 49/52
replace match=104+(_n>50) in 49/52
generate long rowid=_n
generate long frequency=1+mod(_n,3)
replace frequency=1 in 44
replace target=.3+mod(_n,7)/5
generate double x=cos(_n*.73)
replace y=.2*worker-.15*firm+.4*x+sin(_n)/10
generate byte wanted_movers=_n<=40
generate byte wanted_both=_n<=43
generate byte true_stayer=worker==100
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly datasignature
local signature `"`r(datasignature)'"'
foreach nuisance in joint fixedoffset {
    foreach population in movers both {
        quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
            deletionid(match) targetweight(target) stayers(`population') ///
            nuisance(`nuisance') algorithm(exact) backend(mata) nodisplay
        display "POOLED_CASE `nuisance' `population' N=" e(N_retained) " stayers=" e(N_stayers) " blocks=" e(deletion_units)
        assert e(sample)==wanted_`population'
        assert e(N_stayers)==2
        if "`population'"=="both" {
            assert e(stayer_hybrid_N_stayers)==1
            assert e(stayer_hybrid_N_singleton_drop)==1
            assert e(deletion_units)==26
        }
        else assert e(deletion_units)==20
        matrix exact=e(kss)
        matrix plug=e(plugin)
        matrix correction=e(correction)
        assert `"`c(sortrngstate)'"'==`"`sortstate'"'
        mata: st_matrix("literal",pooled_oracle("x","`nuisance'","wanted_`population'","match","true_stayer"))
        quietly set sortrngstate `sortstate'
        display "ORACLE_GAP " mreldif(exact,literal)
        assert mreldif(exact,literal)<1e-8
        display "PLUGIN_GAP " mreldif(plug,literal_plugin) " CORRECTION_GAP " mreldif(correction,literal_correction)
        assert mreldif(plug,literal_plugin)<1e-8
        assert mreldif(correction,literal_correction)<1e-8
        assert rowid==_n
        assert `"`c(rngstate)'"'==`"`state'"'
        assert `"`c(sortrngstate)'"'==`"`sortstate'"'
        quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
            deletionid(match) targetweight(target) stayers(`population') ///
            nuisance(`nuisance') algorithm(jla) engine(generic) ///
            probes(4096) seed(9252026) backend(mata) nodisplay
        assert e(sample)==wanted_`population'
        matrix jla=e(kss)
        matrix mcse=e(numerical_mcse)
        forvalues j=1/4 {
            assert abs(jla[1,`j']-literal[1,`j'])<=6*mcse[1,`j']+1e-8*max(1,abs(literal[1,`j']))
        }
    }
}
quietly datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'

// Default match IDs still mean worker-firm matches; observation movers still
// mean more than one model firm. Neither counts augmented observation IDs.
egen long default_match=group(worker firm)
foreach deletion in match observation {
    quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
        deletion(`deletion') stayers(movers) algorithm(exact) backend(mata) nodisplay
    assert e(sample)==(rowid<=36)
    matrix ordinary=e(kss)
    if "`deletion'"=="match" {
        quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
            deletionid(default_match) stayers(movers) algorithm(exact) backend(mata) nodisplay
        assert e(sample)==(rowid<=36)
        assert mreldif(ordinary,e(kss))<1e-10
    }
}

// Permutation and string IDs preserve the sample, partition and exact result.
quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
    deletionid(match) targetweight(target) algorithm(exact) backend(mata) nodisplay
matrix reference=e(kss)
generate str12 block_s="block"+string(match)
gsort -rowid
quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
    deletionid(block_s) targetweight(target) algorithm(exact) backend(mata) nodisplay
assert e(sample)==wanted_both
assert rowid==53-_n
assert mreldif(reference,e(kss))<1e-8
sort rowid
local sortstate `"`c(sortrngstate)'"'

capture quietly fevc_rust probe
local native_present = (_rc==0)
local native_units = 0
if `native_present' local native_units = mod(floor(r(core_ready_flags)/32768),2)==1
if `native_units' {
    foreach nuisance in joint fixedoffset {
        foreach population in movers both {
            mata: st_matrix("literal",pooled_oracle("x","`nuisance'","wanted_`population'","match","true_stayer"))
            quietly set sortrngstate `sortstate'
            foreach backend in rust auto {
                quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
                    deletionid(match) targetweight(target) stayers(`population') ///
                    nuisance(`nuisance') algorithm(exact) backend(`backend') nodisplay
                assert "`e(backend_selected)'"=="rust"
                assert e(sample)==wanted_`population'
                assert mreldif(e(kss),literal)<1e-8
                assert mreldif(e(plugin),literal_plugin)<1e-8
                assert mreldif(e(correction),literal_correction)<1e-8
            }
            local solvers diagonal
            if "`c(os)'"!="Windows" local solvers diagonal cmg
            foreach solver of local solvers {
                quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
                    deletionid(match) targetweight(target) stayers(`population') ///
                    nuisance(`nuisance') algorithm(jla) engine(generic) ///
                    probes(4096) seed(9252026) backend(rust) rng(counter_v1) ///
                    preconditioner(`solver') nodisplay
                assert e(sample)==wanted_`population'
                matrix jla=e(kss)
                matrix mcse=e(numerical_mcse)
                forvalues j=1/4 {
                    assert abs(jla[1,`j']-literal[1,`j'])<=max(1e-8*max(1,abs(literal[1,`j'])),6*mcse[1,`j'])
                }
                assert "`e(backend_selected)'"=="rust"
                quietly fevc_rust snapshot
                assert r(state)==0 & r(handle)==0
            }
        }
    }
}

// Old installed plugins must reject/fall back before preparation.
capture quietly fevc_rust probe
if !`native_units' & `native_present' {
    quietly fevc_rust clear
    quietly fevc_rust snapshot
    local prior_release=r(last_released)
    foreach algorithm in exact jla auto {
        quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
            deletionid(match) targetweight(target) algorithm(`algorithm') probes(32) nodisplay
        assert "`e(backend_selected)'"=="mata"
        assert e(backend_fallback)==1
        assert "`e(backend_fallback_reason)'"=="RUST_PARALLEL_DELETION_UNSUPPORTED"
        assert "`e(backend_fallback_phase)'"=="preflight"
        assert e(sample)==wanted_both
        if "`algorithm'"!="jla" assert mreldif(reference,e(kss))<1e-8
        capture noisily fevc y x [fw=frequency], worker(worker) firm(firm) ///
            deletionid(match) algorithm(`algorithm') probes(32) backend(rust) nodisplay
        assert _rc==498
        assert "`e(withholding_status)'"=="RUST_PARALLEL_DELETION_UNSUPPORTED"
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0 & r(last_released)==`prior_release'
    }
    capture noisily fevc y, worker(worker) firm(firm) deletionid(match) ///
        deletion(match) stayers(movers) nuisance(fixedoffset) algorithm(jla) engine(generic) ///
        backend(rust) rng(counter_v1) preconditioner(diagonal) ///
        inference(highrank) inferencemodel(structured_common) nodisplay
    assert _rc==498
    assert "`e(withholding_status)'"=="RUST_PARALLEL_DELETION_UNSUPPORTED"
}

// A deletion block may not cross a coefficient cell, even outside the final
// component. Full/deleted control-rank failures remain typed withholding.
generate long bad_match=match
replace bad_match=match[1] in 2
capture noisily fevc y, worker(worker) firm(firm) deletionid(bad_match) backend(mata) nodisplay
assert _rc==198
assert "`e(withholding_status)'"=="CROSS_COORDINATE_MATCH"
generate double badcontrol=(match==match[1])
capture noisily fevc y badcontrol if wanted_movers, worker(worker) firm(firm) ///
    deletionid(match) stayers(movers) backend(mata) algorithm(exact) nodisplay
assert _rc==498
assert inlist("`e(withholding_status)'","NONESTIMABLE_DELETION","AMBIGUOUS_CONTROL_BASIS")
generate double collinear=worker==99
capture noisily fevc y collinear if wanted_movers, worker(worker) firm(firm) ///
    deletionid(match) stayers(movers) backend(mata) algorithm(exact) nodisplay
assert _rc==498
assert "`e(withholding_status)'"=="SINGULAR_INFORMATION"
assert rowid==_n
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'
display "PASS pooled weighted oracle, routing and failures"

// Q=0 exercises the compressed implementation with exactly the same blocks.
quietly fevc y [fw=frequency] if wanted_movers, worker(worker) firm(firm) ///
    deletionid(match) targetweight(target) stayers(movers) ///
    algorithm(exact) backend(mata) nodisplay
matrix exact=e(kss)
foreach engine in compressed generic {
    quietly fevc y [fw=frequency] if wanted_movers, worker(worker) firm(firm) ///
        deletionid(match) targetweight(target) stayers(movers) ///
        algorithm(jla) engine(`engine') probes(8192) seed(9252026) backend(mata) nodisplay
    assert e(sample)==wanted_movers
    assert e(deletion_units)==20
    matrix jla=e(kss)
    matrix mcse=e(numerical_mcse)
    forvalues j=1/4 {
        assert abs(jla[1,`j']-exact[1,`j'])<=6*mcse[1,`j']+1e-8*max(1,abs(exact[1,`j']))
    }
}

if `native_units' {
    // The public Rust compressed route is selected through engine(auto).
    foreach engine in auto generic {
        quietly fevc y [fw=frequency] if wanted_movers, worker(worker) firm(firm) ///
            deletionid(match) targetweight(target) stayers(movers) ///
            algorithm(jla) engine(`engine') probes(8192) seed(9252026) ///
            backend(rust) rng(counter_v1) preconditioner(diagonal) nodisplay
        assert e(sample)==wanted_movers
        assert e(deletion_units)==20
        matrix jla=e(kss)
        matrix mcse=e(numerical_mcse)
        forvalues j=1/4 {
            assert abs(jla[1,`j']-exact[1,`j'])<=max(1e-8*max(1,abs(exact[1,`j'])),6*mcse[1,`j'])
        }
    }
}

// Exact projection consumes the same declared partition and frozen stayer mask.
// The independently computed covariance determines success versus PSD withholding.
foreach population in movers both {
    mata: st_matrix("literal",pooled_oracle("x","joint","wanted_`population'","match","true_stayer"))
    capture noisily fevc y x [fw=frequency], worker(worker) firm(firm) ///
        deletionid(match) targetweight(target) stayers(`population') algorithm(exact) ///
        project(x) projecteffect(worker) projectweight(target) backend(mata) nodisplay
    local projection_rc=_rc
    if scalar(projection_psd) {
        assert `projection_rc'==0
        assert e(sample)==wanted_`population'
        assert mreldif(e(kss),literal)<1e-8
        assert mreldif(e(projection_b),literal_projection_b)<1e-8
        assert mreldif(e(projection_V),literal_projection_V)<1e-8
    }
    else {
        assert `projection_rc'==498
        assert "`e(withholding_status)'"=="PROJECTION_COVARIANCE_INVALID"
    }
}

// One excluded row sharing a supplied ID must not affect complete-case
// validation or frozen histories. This also checks explicit if restrictions.
replace bad_match=match
replace bad_match=match[1] in 44
quietly fevc y if wanted_movers, worker(worker) firm(firm) deletionid(bad_match) ///
    stayers(movers) algorithm(exact) backend(mata) nodisplay
assert e(sample)==wanted_movers

// A positive projection covariance is required in addition to the indefinite
// fixtures above. The independent literal-block oracle fixes the PSD decision.
preserve
clear
set obs 480
generate long worker=floor((_n-1)/20)
generate long firm=mod(floor((_n-1)/2),10)
generate long match=floor((_n-1)/2)+1
replace firm=0 if worker==0
generate long frequency=1
generate double target=1+mod(_n,5)/10
generate double x=sin(worker/5)+cos(firm/3)
generate double y=-4+.08*worker-.12*firm+sin(match*17/11)+.6*cos(match*7/13)
generate byte selected=1
mata: st_matrix("literal",pooled_oracle("","joint","selected","match",""))
assert scalar(projection_psd)==1
quietly fevc y, worker(worker) firm(firm) deletionid(match) stayers(movers) ///
    targetweight(target) algorithm(exact) backend(mata) ///
    project(x) projecteffect(worker) projectweight(target) nodisplay
assert e(sample)==1
assert e(deletion_units)==240
assert mreldif(e(projection_b),literal_projection_b)<1e-8
assert mreldif(e(projection_V),literal_projection_V)<1e-8
if `native_units' {
    quietly fevc y, worker(worker) firm(firm) deletionid(match) stayers(movers) ///
        targetweight(target) algorithm(jla) engine(generic) backend(rust) ///
        rng(counter_v1) preconditioner(diagonal) probes(8192) seed(9252026) ///
        tolerance(1e-12) project(x) projecteffect(worker) projectweight(target) nodisplay
    assert e(sample)==1
    assert e(deletion_units)==240
    assert mreldif(e(projection_b),literal_projection_b)<1e-8
    // Projection covariance has no numerical MCSE; use the existing sparse
    // projection engineering gate with high probes, not point-estimate MCSE.
    assert mreldif(e(projection_V),literal_projection_V)<.005
    assert e(projection_solver_max_complete)<=e(residual_acceptance_tolerance)
}
restore

if "`profile'"=="full" {
    // Development MC, fixed seed and 256 attempted draws, no fitted-cell
    // selection: all failures are saved and the atomic success gate is 100%.
    // Errors have correlation .64 within blocks and independence across blocks.
    // Literal copies of a stored row share its error; target mass is not frequency.
    keep if wanted_movers
    generate double truth_worker=.2*worker
    generate double truth_firm=-.15*firm
    generate double blockshock=.
    generate double eps=.
    generate double mean_y=truth_worker+truth_firm+.4*x
    mata:
    a=st_data(.,"target"); a=a/sum(a)
    w=st_data(.,"truth_worker"); f=st_data(.,"truth_firm")
    w=w:-sum(a:*w); f=f:-sum(a:*f)
    truth=(sum(a:*w:^2),sum(a:*f:^2),sum(a:*w:*f),sum(a:*(w+f):^2))
    st_matrix("truth",truth)
    end
    local mc_rng `"`c(rngstate)'"'
    local mc_sort `"`c(sortrngstate)'"'
    set seed 9252026
    tempfile mc_results
    tempname mcpost
    postfile `mcpost' int replication rc double worker firm covariance total using `mc_results'
    forvalues replication=1/256 {
        quietly bysort match (rowid): replace blockshock=rnormal() if _n==1
        quietly by match: replace blockshock=blockshock[1]
        quietly replace eps=.8*blockshock+.6*rnormal()
        quietly replace y=mean_y+eps
        capture quietly fevc y x [fw=frequency], worker(worker) firm(firm) ///
            deletionid(match) targetweight(target) stayers(movers) ///
            nuisance(joint) algorithm(exact) backend(mata) nodisplay
        local fit_rc=_rc
        if `fit_rc' {
            post `mcpost' (`replication') (`fit_rc') (.) (.) (.) (.)
        }
        else {
            assert e(sample)==1 & e(deletion_units)==20
            matrix fit=e(kss)
            post `mcpost' (`replication') (0) (fit[1,1]) (fit[1,2]) (fit[1,3]) (fit[1,4])
        }
    }
    postclose `mcpost'
    quietly set rngstate `mc_rng'
    quietly set sortrngstate `mc_sort'
    use `mc_results', clear
    isid replication
    assert _N==256
    capture mkdir ".local"
    capture mkdir ".local/pooled-deletion"
    export delimited using ".local/pooled-deletion/monte_carlo.csv", replace
    count if rc!=0
    display "POOLED_MC attempted=256 failed=" r(N) " seed=9252026"
    list replication rc if rc!=0
    assert rc==0
    local j=0
    foreach target in worker firm covariance total {
        local ++j
        quietly summarize `target'
        scalar error=r(mean)-truth[1,`j']
        scalar sampling_mcse=r(sd)/sqrt(r(N))
        display "POOLED_MC `target' truth=" truth[1,`j'] " bias=" error " MCSE=" sampling_mcse
        assert abs(error)<6*sampling_mcse+1e-8
    }
}
display "PASS test_pooled_deletion.do"
