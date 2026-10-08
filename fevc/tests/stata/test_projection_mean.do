version 18.0
clear all
set more off
set varabbrev off

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

// Unequal physical frequencies and match lengths, deterministic
// heteroskedastic errors, and eligible stayers distinguish the sample mean
// from stored-row, target-weighted, and transformed-match means.
set obs 168
generate long worker = cond(_n<=144,floor((_n-1)/12)+1,13+floor((_n-145)/6))
generate byte firm = cond(_n<=144,floor(mod(_n-1,12)/3)+1,1+floor((_n-145)/6))
generate double c1 = sin(_n*.53)
generate double z = sin(worker/3)+firm*.3+c1*.05
generate byte copies = 1+mod(_n+floor(_n/7),3)
generate double target_mass = .5+mod(_n*7,11)/5
generate double noise = (1+.35*firm+.25*mod(_n,3))*(sin(_n*1.7)+cos(_n*.61))
generate double y = .1+.06*worker-.15*firm+.4*c1+noise
generate double shifted = y+20
generate long stored_row = _n

mata:
void projection_mean_oracle(string scalar deletion, string scalar nuisance,
    string scalar effect, string scalar weight)
{
    real colvector y, w, f, v, c, z, t, beta, r, yc, idx, keep, db, de
    real colvector physical_remaining, py, pe, pr
    real matrix X, T, Z, Xi, L, S, V, Vnone, Vnaive
    real scalar n, nw, i, mu
    y=st_data(.,"y","oracle_sample")
    w=st_data(.,"worker","oracle_sample")
    f=st_data(.,"firm","oracle_sample")
    v=st_data(.,"copies","oracle_sample")
    c=st_data(.,"c1","oracle_sample")
    z=st_data(.,"z","oracle_sample")
    t=st_data(.,"target_mass","oracle_sample")
    n=rows(y); nw=max(w)
    X=T=J(n,nw+3,0)
    for(i=1;i<=n;i++) {
        X[i,w[i]]=1
        if(f[i]<4) X[i,nw+f[i]]=1
        if(effect=="worker") T[i,w[i]]=1
        else if(f[i]<4) T[i,nw+f[i]]=1
    }
    if(nuisance!="none") {
        X=(X,c);T=(T,J(n,1,0))
    }
    Xi=invsym(X'*(v:*X));beta=Xi*(X'*(v:*y))
    if(nuisance=="fixedoffset") {
        y=y-c*beta[rows(beta)]
        X=X[.,1..cols(X)-1];T=T[.,1..cols(T)-1]
        Xi=invsym(X'*(v:*X));beta=Xi*(X'*(v:*y))
    }
    r=y-X*beta;mu=sum(v:*y)/sum(v);yc=y:-mu
    Z=(J(n,1,1),z)
    if(weight=="frequency") t=v
    L=T'*(t:*Z)*invsym(Z'*(t:*Z));S=X*Xi*L
    V=Vnone=Vnaive=J(2,2,0)
    for(i=1;i<=n;i++) {
        if(deletion=="match" & w[i]<=12) {
            idx=selectindex((w:==w[i]):&(f:==f[i]))
            if(i!=idx[1]) continue
            keep=selectindex(!((w:==w[i]):&(f:==f[i])))
            // Refit after deleting the complete block: this oracle does not
            // use production leverages, residual makers, or centering code.
            db=invsym(X[keep,.]'*(v[keep]:*X[keep,.]))*
                (X[keep,.]'*(v[keep]:*y[keep]))
            de=y[idx]-X[idx,.]*db
            py=S[idx,.]'*(v[idx]:*yc[idx])
            pe=S[idx,.]'*(v[idx]:*de)
            pr=S[idx,.]'*(v[idx]:*r[idx])
            V=V+.5:*(py*pe'+pe*py')
            py=S[idx,.]'*(v[idx]:*y[idx])
            Vnone=Vnone+.5:*(py*pe'+pe*py')
            Vnaive=Vnaive+pr*pr'
        }
        else {
            // Remove one physical copy, preserving the other identical
            // copies when the stored frequency exceeds one.
            physical_remaining=v;physical_remaining[i]=v[i]-1
            db=invsym(X'*(physical_remaining:*X))*(X'*(physical_remaining:*y))
            de=y[i]-X[i,.]*db
            V=V+v[i]*yc[i]*de[1]*(S[i,.]'*S[i,.])
            Vnone=Vnone+v[i]*y[i]*de[1]*(S[i,.]'*S[i,.])
            Vnaive=Vnaive+v[i]*r[i]^2*(S[i,.]'*S[i,.])
        }
    }
    st_matrix("oracle_b",(L'*beta)')
    st_matrix("oracle_V",V)
    st_matrix("oracle_V_none",Vnone)
    st_matrix("oracle_V_naive",Vnaive)
    st_numscalar("oracle_mean",mu)
    st_numscalar("oracle_row_mean",mean(y))
    st_numscalar("oracle_target_mean",sum(st_data(.,"target_mass","oracle_sample"):*y)/
        sum(st_data(.,"target_mass","oracle_sample")))
    st_numscalar("oracle_raw_mean",sum(v:*st_data(.,"y","oracle_sample"))/sum(v))
}
end

local cells = 0
scalar mode_difference = 0
set seed 721946
local rng_before `"`c(rngstate)'"'
foreach deletion in observation match {
    foreach population in both movers {
        foreach nuisance in none joint fixedoffset {
            local controls
            local nuisance_option
            if "`nuisance'"!="none" {
                local controls c1
                local nuisance_option nuisance(`nuisance')
            }
            foreach effect in worker firm {
                foreach weight in frequency target {
                    local common worker(worker) firm(firm) algorithm(exact) backend(mata) ///
                        deletion(`deletion') stayers(`population') `nuisance_option' ///
                        targetweight(target_mass) project(z) projecteffect(`effect') ///
                        projectweight(`weight') nodisplay
                    display as text "CELL `deletion' `population' `nuisance' `effect' `weight'"
                    quietly fevc y `controls' [fw=copies], `common'
                    assert "`e(centering)'"=="mean"
                    assert "`e(status)'"=="KSS_PROJECTION_INFERENCE"
                    matrix actual_b=e(projection_b)
                    matrix actual_V=e(projection_V)
                    matrix actual_naive=e(projection_V_naive)
                    generate byte oracle_sample=e(sample)
                    count if oracle_sample
                    assert r(N)==cond("`population'"=="both",168,144)
                    mata: projection_mean_oracle("`deletion'","`nuisance'","`effect'","`weight'")
                    assert mreldif(actual_b,oracle_b)<2e-10
                    assert mreldif(actual_V,oracle_V)<2e-10
                    assert mreldif(actual_naive,oracle_V_naive)<2e-10
                    assert abs(oracle_mean-oracle_row_mean)>1e-4
                    assert abs(oracle_mean-oracle_target_mean)>1e-4
                    if "`nuisance'"=="fixedoffset" assert abs(oracle_mean-oracle_raw_mean)>1e-4
                    drop oracle_sample

                    quietly fevc y `controls' [fw=copies], `common' centering(mean)
                    assert mreldif(actual_b,e(projection_b))<1e-13
                    assert mreldif(actual_V,e(projection_V))<1e-13
                    quietly fevc y `controls' [fw=copies], `common' centering(none)
                    assert "`e(centering)'"=="none"
                    assert mreldif(actual_b,e(projection_b))<2e-10
                    assert mreldif(actual_naive,e(projection_V_naive))<2e-10
                    assert mreldif(oracle_V_none,e(projection_V))<2e-10
                    scalar mode_difference=max(mode_difference,mreldif(actual_V,e(projection_V)))

                    quietly fevc shifted `controls' [fw=copies], `common'
                    assert mreldif(actual_V,e(projection_V))<2e-9
                    assert mreldif(actual_naive,e(projection_V_naive))<2e-9
                    matrix shifted_b=e(projection_b)
                    assert abs(actual_b[1,2]-shifted_b[1,2])<2e-9
                    local ++cells
                }
            }
        }
    }
}
assert `cells'==48
assert mode_difference>1e-4

// Literal expansion independently verifies physical-frequency semantics for
// both deletion schemes, including the mixed mover/stayer partition.
foreach deletion in observation match {
    local common worker(worker) firm(firm) algorithm(exact) backend(mata) ///
        deletion(`deletion') stayers(both) nuisance(fixedoffset) ///
        project(z) projecteffect(worker) projectweight(target) nodisplay
    quietly fevc y c1 [fw=copies], `common' targetweight(target_mass)
    matrix weighted_b=e(projection_b)
    matrix weighted_V=e(projection_V)
    matrix weighted_naive=e(projection_V_naive)
    preserve
        expand copies
        generate double target_copy=target_mass/copies
        quietly fevc y c1, `common' targetweight(target_copy)
        assert mreldif(weighted_b,e(projection_b))<2e-10
        assert mreldif(weighted_V,e(projection_V))<2e-10
        assert mreldif(weighted_naive,e(projection_V_naive))<2e-10
    restore
}
assert `"`c(rngstate)'"'==`"`rng_before'"'
assert "$VCKSS_CENTERING"==""
di as result "PASS test_projection_mean.do cells=`cells'"
exit 0
