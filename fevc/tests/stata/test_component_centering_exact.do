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

// The matrices below are built from explicit dummy variables and weighted
// covariance definitions, independently of production target/influence code.
mata:
real colvector component_centering_noise_oracle(real matrix C,
    real colvector variance, real scalar simulations, real scalar seed)
{
    real scalar begin, finish
    real matrix errors
    real colvector output
    rseed(seed)
    output=J(simulations,1,.)
    for(begin=1;begin<=simulations;begin=begin+32) {
        finish=min((simulations,begin+31))
        errors=sqrt(variance):*rnormal(rows(C),finish-begin+1,0,1)
        output[|begin\finish|]=colsum(errors:*(C*errors))'
    }
    return(output)
}

void component_centering_exact_oracle(string scalar controls,
    string scalar nuisance, string scalar weighted)
{
    real scalar n, nw, nf, i, j, k, lambda, ei, sigma, c
    real colvector y, w, f, mass, z, beta, residual, h, elo, d, row_order
    real colvector mode, eval, drem, g, grem, raw, qsim, expected, fitted
    real matrix X, U, F, Z, A, M, Htarget, Q, B, C, BR, CR
    real matrix root, evec, oracle, q1oracle, W, QW, QF, QC, VV, noiseq
    real matrix key, public_noise, public_V
    y=st_data(.,"y","oracle_sample")
    w=st_data(.,"worker","oracle_sample")
    f=st_data(.,"firm","oracle_sample")
    mass=st_data(.,"target_mass","oracle_sample")
    if(weighted=="none") mass=J(rows(y),1,1)
    Z=st_data(.,("c1","c2"),"oracle_sample")
    // Reproduce only the documented exact-route row keys so independent
    // Gaussian draws attach to the same observations as the public command.
    if(controls=="none") key=(w,f,y,J(rows(y),1,1),mass)
    else key=(w,f,mass,y,Z)
    row_order=order(key,1..cols(key))
    y=y[row_order];w=w[row_order];f=f[row_order]
    mass=mass[row_order];Z=Z[row_order,.]
    n=rows(y);nw=max(w);nf=max(f)
    X=U=F=J(n,nw+nf-1,0)
    for(i=1;i<=n;i++) {
        X[i,w[i]]=U[i,w[i]]=1
        if(f[i]<nf) X[i,nw+f[i]]=F[i,nw+f[i]]=1
    }
    if(controls!="none") {
        beta=qrsolve((X,Z),y)
        if(nuisance=="fixedoffset") y=y-Z*beta[(cols(X)+1)..rows(beta)]
        else {
            X=(X,Z);U=(U,J(n,cols(Z),0));F=(F,J(n,cols(Z),0))
        }
    }
    mass=mass/sum(mass)
    Htarget=diag(mass)-mass*mass'
    QW=U'*Htarget*U;QF=F'*Htarget*F
    QC=(U'*Htarget*F+F'*Htarget*U)/2
    A=invsym(X'*X);beta=A*X'*y
    M=I(n)-X*A*X';residual=M*y;h=1:-diagonal(M)
    elo=residual:/(1:-h);c=mean(y);z=y:-c;raw=z:*elo
    sigma=mean(raw)
    assert(sigma>0)
    oracle=J(1,4,.);q1oracle=J(4,7,.)
    W=J(n,4,.);noiseq=J(73,4,.)
    public_noise=J(200,4,.)
    fitted=.8:+(1..n)'/n
    root=cholesky(A)
    for(j=1;j<=4;j++) {
        if(j==1) Q=QW
        else if(j==2) Q=QF
        else if(j==3) Q=QC
        else Q=QW+QF+2*QC
        B=X*A*Q*A*X';d=diagonal(B)
        C=B-(diag(d:/(1:-h))*M+M*diag(d:/(1:-h)))/2
        assert(max(abs(diagonal(C)))<1e-12)
        assert(max(abs(B*J(n,1,1)))<1e-12)
        g=C*z;W[.,j]=g
        oracle[j]=(z'*C*z)[1]
        assert(abs(oracle[j]-(beta'*Q*beta-sum(d:*raw))[1])<1e-11)
        assert(mreldif(g,vckss_inf__W(z,X,A,Q,beta,h,elo,d))<1e-11)
        // These probes are zero-mean errors through C, never demeaned draws.
        qsim=vckss_inf__simquad(X,A,Q,h,d,fitted,73,18431,J(0,1,.),0)
        expected=component_centering_noise_oracle(C,fitted,73,18431)
        assert(mreldif(qsim,expected)<1e-11)
        noiseq[.,j]=qsim
        public_noise[.,j]=component_centering_noise_oracle(C,
            J(n,1,sigma),200,42)
        symeigensystem(root'*Q*root,evec,eval)
        ei=selectindex(abs(eval):==max(abs(eval)))[1]
        lambda=eval[ei];mode=X*root*evec[.,ei]
        mode=mode/sqrt(sum(mode:^2))
        assert(abs(sum(mode))<1e-10)
        BR=B-lambda*mode*mode';drem=diagonal(BR)
        CR=BR-(diag(drem:/(1:-h))*M+M*diag(drem:/(1:-h)))/2
        grem=CR*z
        expected=vckss_inf__W(z,X,A,Q,beta,h,elo,drem)-
            lambda*mode*(mode'*z)
        assert(mreldif(grem,expected)<1e-11)
        assert(abs(oracle[j]-lambda*((mode'*z)[1]^2-
            sum(mode:^2:*raw))-(z'*CR*z)[1])<1e-11)
        qsim=vckss_inf__simquad(X,A,Q,h,drem,fitted,73,18431,mode,lambda)
        expected=component_centering_noise_oracle(CR,fitted,73,18431)
        assert(mreldif(qsim,expected)<1e-11)
        q1oracle[j,.]=(lambda,sum(mode:^2:*raw),(z'*CR*z)[1],
            (mode'*z)[1]^2,(2*sigma*mode'*grem)[1]^2,
            sum(abs(abs(eval):-abs(lambda)):<1e-9*abs(lambda))==1,
            4*sigma*sum(grem:^2)-variance(component_centering_noise_oracle(
                CR,J(n,1,sigma),200,42)))
    }
    assert(mreldif(W[.,4],W[.,1]+W[.,2]+2*W[.,3])<1e-12)
    assert(mreldif(noiseq[.,4],noiseq[.,1]+noiseq[.,2]+2*noiseq[.,3])<1e-11)
    // Joint covariance and polarization use identical errors for every target.
    VV=4*W'*(fitted:*W)-variance(noiseq)
    for(j=1;j<=3;j++) for(k=j+1;k<=3;k++) {
        assert(abs(VV[j,k]-(4*sum((W[.,j]+W[.,k]):^2:*fitted)-
            variance(noiseq[.,j]+noiseq[.,k])-VV[j,j]-VV[k,k])/2)<1e-10)
    }
    st_matrix("oracle_point",oracle)
    st_matrix("oracle_q1",q1oracle)
    public_V=4*sigma*W'*W-variance(public_noise)
    st_matrix("oracle_public_V",public_V)
    st_numscalar("oracle_mean",c)
    st_numscalar("oracle_sigma",sigma)
}
end

set seed 719321
local rng_before `"`c(rngstate)'"'
local cells=0
foreach controls in none joint fixedoffset {
    local covars
    local nuisance joint
    if "`controls'"!="none" local covars c1 c2
    if "`controls'"=="fixedoffset" local nuisance fixedoffset
    foreach weighted in none target {
        local target_option
        if "`weighted'"=="target" local target_option targetweight(target_mass)
        local common worker(worker) firm(firm) deletion(observation) ///
            algorithm(exact) backend(mata) nuisance(`nuisance') `target_option' ///
            inferencesimulations(200) inferenceseed(42) inferencebins(4) nodisplay
        foreach reference in highrank q1 {
            di as text "CELL `controls' `weighted' `reference'"
            quietly fevc y `covars', `common' inference(`reference')
            assert "`e(centering)'"=="mean"
            assert "`e(inference_centering)'"=="fixed observed mean"
            assert e(inference_mean_omitted)==1
            matrix mean_b=e(b)
            matrix mean_V=e(V)
            matrix mean_results=e(component_inference)
            if "`reference'"=="q1" {
                assert e(q1_computed_targets)==4
                matrix mean_q1=e(q1_inference)
                matrix mean_status=e(q1_status)
                matrix mean_failure=e(q1_failure_diagnostics)
            }
            generate byte oracle_sample=e(sample)
            assert oracle_sample==1
            mata: component_centering_exact_oracle("`controls'","`nuisance'","`weighted'")
            // Oracle RNG use is explicit test work, separate from caller restoration.
            set rngstate `rng_before'
            assert mreldif(mean_b,oracle_point)<2e-11
            assert mreldif(mean_V,oracle_public_V)<2e-10
            if "`reference'"=="q1" {
                mata: assert(sum(st_matrix("oracle_q1")[.,6])>=3)
                if "`weighted'"=="target" assert oracle_q1[4,6]==1
                forvalues target=1/4 {
                    assert reldif(mean_q1[`target',7],oracle_q1[`target',1])<2e-10
                    // A repeated leading eigenvalue allows different bases
                    // of the same eigenspace; compare mode-specific public
                    // scalars only when the leading direction is unique.
                    if oracle_q1[`target',6] {
                        assert reldif(mean_failure[`target',5],oracle_q1[`target',2])<2e-10
                        assert reldif(mean_q1[`target',14],oracle_q1[`target',3])<2e-10
                        assert reldif(mean_q1[`target',13]^2,oracle_q1[`target',4])<2e-10
                        assert reldif(mean_q1[`target',11]^2,oracle_q1[`target',5])<2e-10
                        assert reldif(mean_q1[`target',12],oracle_q1[`target',7])<2e-10
                    }
                    assert reldif(mean_q1[`target',10],oracle_sigma)<2e-10
                    assert mean_failure[`target',6]<1e-9
                }
            }
            generate double manual_centered=y-oracle_mean
            quietly fevc manual_centered `covars', `common' inference(`reference') centering(none)
            assert "`e(inference_centering)'"=="uncentered"
            assert e(inference_mean_omitted)==0
            assert mreldif(mean_b,e(b))<2e-11
            assert mreldif(mean_V,e(V))<2e-10
            assert mreldif(mean_results,e(component_inference))<2e-10
            if "`reference'"=="q1" {
                assert mreldif(mean_q1,e(q1_inference))<2e-9
                assert mreldif(mean_status,e(q1_status))<2e-10
            }
            quietly fevc y `covars', `common' inference(`reference') centering(mean)
            assert mreldif(mean_b,e(b))<1e-13
            assert mreldif(mean_V,e(V))<1e-13
            quietly fevc shifted `covars', `common' inference(`reference')
            assert mreldif(mean_b,e(b))<2e-8
            assert mreldif(mean_V,e(V))<2e-8
            if "`reference'"=="q1" assert mreldif(mean_q1,e(q1_inference))<2e-7
            quietly fevc scaled `covars', `common' inference(`reference')
            matrix scale_b=e(b)/9
            matrix scale_V=e(V)/81
            assert mreldif(mean_b,scale_b)<2e-10
            assert mreldif(mean_V,scale_V)<2e-9
            if "`reference'"=="q1" {
                matrix scale_q1=e(q1_inference)
                forvalues target=1/4 {
                    foreach column in 5 6 {
                        assert reldif(mean_q1[`target',`column'],scale_q1[`target',`column']/9)<2e-9
                    }
                    assert reldif(mean_q1[`target',16],scale_q1[`target',16])<2e-9
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
    backend(mata) inferencebins(4) inferencesimulations(200) inferenceseed(42) nodisplay
quietly fevc y c1 c2, `common' inference(highrank)
matrix component_b=e(b)
matrix component_V=e(V)
matrix component_mcse=e(mcse)
quietly lincom worker_variance+firm_variance+2*worker_firm_covariance
assert abs(r(estimate)-component_b[1,4])<2e-12
assert abs(r(se)-sqrt(component_V[4,4]))<2e-12
quietly fevc y c1 c2, `common'
assert mreldif(component_b,e(b))<2e-12
assert mreldif(component_mcse,e(mcse))<2e-12
assert "`e(inference_centering)'"==""
capture confirm scalar e(inference_mean_omitted)
assert _rc!=0
quietly fevc y c1 c2, `common' project(projection) projecteffect(firm)
matrix project_b=e(projection_b)
matrix project_V=e(projection_V)
matrix project_naive=e(projection_V_naive)
foreach reference in highrank q1 {
    quietly fevc y c1 c2, `common' inference(`reference') ///
        project(projection) projecteffect(firm)
    assert mreldif(component_b,e(b))<2e-12
    assert mreldif(component_V,e(V))<2e-12
    assert mreldif(project_b,e(projection_b))<2e-12
    assert mreldif(project_V,e(projection_V))<2e-12
    assert mreldif(project_naive,e(projection_V_naive))<2e-12
}
assert `"`c(rngstate)'"'==`"`rng_before'"'
assert "$VCKSS_CENTERING"==""
di as result "PASS test_component_centering_exact.do cells=`cells'"
exit 0
