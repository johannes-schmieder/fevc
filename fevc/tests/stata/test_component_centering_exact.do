version 18.0
clear all
set more off
set varabbrev off

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

set obs 24
generate long worker = floor((_n-1)/4)+1
generate byte time = mod(_n-1,4)
generate double c1 = time-1.5
generate double c2 = time==2
generate byte firm = .
generate double noise = .
local firms 1 1 2 2 1 3 3 2 2 3 4 4 3 4 1 1 4 2 2 3 4 4 3 1
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5+worker+2*firm+.4*c1-.15*c2+noise
generate double target_mass = 1+time/10
generate double projection = worker+time/10
generate double shifted = y+1000
generate double scaled = 3*y
generate long original_row = _n

// Repeat-measurement standard errors on the exact Mata route under outcome
// centering. Their values are checked against an independent dense
// calculation in test_repeat_measurement_se.do; here the point estimate of
// every centering mode is checked against an independent calculation built
// from explicit dummy variables and weighted covariance definitions, and the
// standard errors against the invariances the centering modes imply.
mata:
void component_centering_exact_oracle(string scalar controls,
    string scalar weighted)
{
    real scalar n, nw, nf, i, j, c
    real colvector y, w, f, mass, z, h, d
    real matrix X, U, F, Z, A, M, Htarget, Q, B, C, oracle, QW, QF, QC
    y=st_data(.,"y","oracle_sample")
    w=st_data(.,"worker","oracle_sample")
    f=st_data(.,"firm","oracle_sample")
    mass=st_data(.,"target_mass","oracle_sample")
    if(weighted=="none") mass=J(rows(y),1,1)
    Z=st_data(.,("c1","c2"),"oracle_sample")
    n=rows(y);nw=max(w);nf=max(f)
    X=U=F=J(n,nw+nf-1,0)
    for(i=1;i<=n;i++) {
        X[i,w[i]]=U[i,w[i]]=1
        if(f[i]<nf) X[i,nw+f[i]]=F[i,nw+f[i]]=1
    }
    // Jointly estimated controls stay in the leave-out operator.
    if(controls!="none") {
        X=(X,Z);U=(U,J(n,cols(Z),0));F=(F,J(n,cols(Z),0))
    }
    mass=mass/sum(mass)
    Htarget=diag(mass)-mass*mass'
    QW=U'*Htarget*U;QF=F'*Htarget*F
    QC=(U'*Htarget*F+F'*Htarget*U)/2
    A=invsym(X'*X)
    M=I(n)-X*A*X';h=1:-diagonal(M)
    c=mean(y);z=y:-c
    oracle=J(1,4,.)
    for(j=1;j<=4;j++) {
        if(j==1) Q=QW
        else if(j==2) Q=QF
        else if(j==3) Q=QC
        else Q=QW+QF+2*QC
        B=X*A*Q*A*X';d=diagonal(B)
        C=B-(diag(d:/(1:-h))*M+M*diag(d:/(1:-h)))/2
        assert(max(abs(diagonal(C)))<1e-12)
        assert(max(abs(B*J(n,1,1)))<1e-12)
        oracle[j]=(z'*C*z)[1]
    }
    st_matrix("oracle_point",oracle)
    st_numscalar("oracle_mean",c)
}
end

set seed 719321
local rng_before `"`c(rngstate)'"'
local mean_text "mean-centred estimate; pseudo-noise centred identically"
local corrected_text "standard error of the mean-centred estimate; corrected-centering increment treated as fixed"
local cells=0
// Observation deletion with and without jointly estimated controls, and
// match deletion without controls. The model does not carry observation
// deletion with nuisance(fixedoffset), and on this 24-row fixture its fitted
// variance for movers has no positive scale under match deletion with controls.
foreach design in observation_none observation_joint match_none {
    local deletion = cond(strpos("`design'","match"),"match","observation")
    local controls = cond(strpos("`design'","joint"),"joint","none")
    local covars
    if "`controls'"!="none" local covars c1 c2
    foreach weighted in none target {
        local target_option
        if "`weighted'"=="target" local target_option targetweight(target_mass)
        local common worker(worker) firm(firm) deletion(`deletion') ///
            algorithm(exact) backend(mata) `target_option' inferenceseed(42) nodisplay
        foreach reference in highrank q1 {
            di as text "CELL `design' `weighted' `reference'"
            quietly fevc y `covars', `common' inference(`reference')
            assert "`e(centering)'"=="mean"
            assert "`e(inference_model)'"=="repeat_measurement"
            // The pseudo-noise is centred as the estimate is: the observed
            // mean is estimated, not held fixed.
            assert "`e(inference_centering)'"=="`mean_text'"
            assert e(inference_mean_omitted)==0
            matrix mean_b=e(b)
            matrix mean_results=e(component_inference)
            matrix mean_se=mean_results[1..4,2]
            capture confirm matrix e(V)
            local posted = !_rc
            if `posted' matrix mean_V=e(V)
            if "`reference'"=="highrank" assert `posted'
            if "`reference'"=="q1" {
                assert e(inference_q1_components)>0
                matrix mean_table=e(inference_intervals)
                matrix mean_reported=mean_table[1..4,1..10]
                assert `posted'==(e(inference_q1_components)==0)
            }
            generate byte oracle_sample=e(sample)
            assert oracle_sample==1
            if "`deletion'"=="observation" {
                mata: component_centering_exact_oracle("`controls'","`weighted'")
                assert mreldif(mean_b,oracle_point)<2e-11
            }
            else {
                quietly summarize y, meanonly
                scalar oracle_mean = r(mean)
            }
            // Shifting by the observed mean leaves the mean-centred
            // estimate and its standard errors unchanged.
            generate double manual_centered=y-oracle_mean
            quietly fevc manual_centered `covars', `common' inference(`reference') centering(mean)
            assert mreldif(mean_b,e(b))<2e-11
            assert mreldif(mean_results,e(component_inference))<2e-10
            if "`reference'"=="q1" {
                matrix candidate=e(inference_intervals)
                matrix candidate=candidate[1..4,1..10]
                assert mreldif(mean_reported,candidate)<2e-10
            }
            // centering(none) on the centred outcome: the same estimate, but
            // the pseudo-noise is not centred, so the standard errors hold
            // the observed mean fixed and differ slightly.
            quietly fevc manual_centered `covars', `common' inference(`reference') centering(none)
            assert "`e(inference_centering)'"=="uncentered"
            assert e(inference_mean_omitted)==0
            assert mreldif(mean_b,e(b))<2e-11
            matrix candidate=e(component_inference)
            forvalues target=1/4 {
                assert reldif(candidate[`target',2],mean_se[`target',1])<.005
            }
            quietly fevc y `covars', `common' inference(`reference') centering(mean)
            assert mreldif(mean_b,e(b))<1e-13
            assert mreldif(mean_results,e(component_inference))<1e-13
            quietly fevc shifted `covars', `common' inference(`reference')
            assert mreldif(mean_b,e(b))<2e-9
            assert mreldif(mean_results,e(component_inference))<2e-9
            if `posted' assert mreldif(mean_V,e(V))<2e-9
            if "`reference'"=="q1" {
                matrix candidate=e(inference_intervals)
                matrix candidate=candidate[1..4,1..10]
                assert mreldif(mean_reported,candidate)<2e-9
            }
            // Scale equivariance: estimates by 9, variances by 81, interval
            // bounds by 9, variance shares unchanged.
            quietly fevc scaled `covars', `common' inference(`reference')
            matrix scale_b=e(b)/9
            assert mreldif(mean_b,scale_b)<2e-10
            if `posted' {
                matrix scale_V=e(V)/81
                assert mreldif(mean_V,scale_V)<2e-9
            }
            if "`reference'"=="q1" {
                matrix scale_table=e(inference_intervals)
                forvalues target=1/4 {
                    forvalues column=1/6 {
                        assert reldif(mean_table[`target',`column'],scale_table[`target',`column']/9)<2e-9
                    }
                    assert reldif(mean_table[`target',10],scale_table[`target',10])<2e-9
                    assert mean_table[`target',7]==scale_table[`target',7]
                }
            }
            // centering(corrected): the standard errors of the mean-centred
            // estimate with the corrected increment held fixed; a q = 1
            // interval moves with the posted estimate.
            quietly fevc y `covars', `common' inference(`reference') centering(corrected)
            assert "`e(centering)'"=="corrected"
            assert "`e(inference_centering)'"=="`corrected_text'"
            matrix candidate=e(component_inference)
            matrix candidate=candidate[1..4,2]
            assert mreldif(mean_se,candidate)<1e-13
            if "`reference'"=="q1" {
                matrix candidate=e(inference_intervals)
                matrix shift=e(b)-mean_b
                forvalues target=1/4 {
                    foreach column in 5 6 {
                        assert abs(candidate[`target',`column']-mean_table[`target',`column']- ///
                            shift[1,`target'])<1e-11*max(1,abs(mean_table[`target',`column']))
                    }
                }
            }
            assert `"`c(rngstate)'"'==`"`rng_before'"'
            assert original_row==_n
            drop oracle_sample manual_centered
            local ++cells
        }
    }
}
assert `cells'==12

local common worker(worker) firm(firm) deletion(observation) algorithm(exact) ///
    backend(mata) inferenceseed(42) nodisplay
quietly fevc y c1 c2, `common' inference(highrank)
matrix component_b=e(b)
matrix component_V=e(V)
matrix component_mcse=e(mcse)
quietly lincom worker_variance+firm_variance+2*worker_firm_covariance
assert abs(r(estimate)-component_b[1,4])<2e-12
assert abs(r(se)-sqrt(component_V[4,4]))<2e-12
quietly fevc y c1 c2, `common' inference(none)
assert mreldif(component_b,e(b))<2e-12
assert mreldif(component_mcse,e(mcse))<2e-12
assert "`e(inference_centering)'"==""
capture confirm scalar e(inference_mean_omitted)
assert _rc!=0
// Block projections are a separate estimand: the default leaves the
// components to point estimates beside them, and an explicit request for
// component standard errors cannot share the call.
quietly fevc y c1 c2, `common' inference(none) project(projection) projecteffect(firm)
matrix project_b=e(projection_b)
matrix project_V=e(projection_V)
matrix project_naive=e(projection_V_naive)
quietly fevc y c1 c2, `common' inference(auto) project(projection) projecteffect(firm)
assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""
assert mreldif(component_b,e(b))<2e-12
assert mreldif(project_b,e(projection_b))<2e-12
assert mreldif(project_V,e(projection_V))<2e-12
assert mreldif(project_naive,e(projection_V_naive))<2e-12
foreach reference in highrank q1 {
    capture noisily fevc y c1 c2, `common' inference(`reference') ///
        project(projection) projecteffect(firm)
    assert _rc==498
    assert "`e(withholding_status)'"=="COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED"
}
// The simulation and bin counts of the removed exact model stop at once.
foreach option in "inferencesimulations(200)" "inferencebins(4)" "inferencegramprobes(512)" {
    capture noisily fevc y c1 c2, `common' inference(highrank) `option'
    assert _rc==198
    assert "`e(withholding_status)'"=="INFERENCE_OPTION_REMOVED"
}
capture noisily fevc y c1 c2, `common' inference(q1) inferencemodel(target_lowess)
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_REMOVED"
assert `"`c(rngstate)'"'==`"`rng_before'"'
assert "$VCKSS_CENTERING"==""
di as result "PASS test_component_centering_exact.do cells=`cells'"
exit 0
