version 18
clear all
set more off
set processors 4
args pkgroot backend
if `"`pkgroot'"'=="" local pkgroot "`c(pwd)'/fevc"
if "`backend'"=="" local backend mata
adopath ++ `"`pkgroot'"'
set seed 270923
set obs 11520
gen long worker=ceil(_n/96)
gen time=mod(_n-1,96)
gen firm=1+mod(worker+floor(time/(15+mod(worker,7))),17)
replace firm=1+mod(worker,17) if worker>100
gen year=2002+floor(time/12)
gen age=25+mod(worker,20)+year-2002
gen edu=mod(worker,3)
local controls
forvalues j=0/2 {
    gen double a2_`j'=((age-40)/10)^2*(edu==`j')
    gen double a3_`j'=((age-40)/10)^3*(edu==`j')
    local controls `controls' a2_`j' a3_`j'
}
forvalues j=2003/2009 {
    gen byte yr`j'=year==`j'
    local controls `controls' yr`j'
}
gen double y=worker/500+firm/100+.03*((age-40)/10)^2+rnormal()/5
keep if worker<=100 | inlist(time,0,95)
gen double mass=1+mod(worker,5)/10
gen double rotated=a2_0+.5*yr2009
local transformed : subinstr local controls "a2_0" "rotated",all
sort worker time
local caller_rng `"`c(rngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
local rng=cond("`backend'"=="rust","counter_v1","stata")
foreach pop in movers both {
    local options worker(worker) firm(firm) deletion(match) stayers(`pop') nuisance(joint) ///
        targetweight(mass) backend(`backend') rng(`rng') probes(256) seed(9320) nodisplay
    quietly fevc y `controls', `options' algorithm(exact)
    matrix exact=e(results)
    matrix plugin=e(plugin)
    assert e(N)==cond("`pop'"=="movers",9600,9640)
    assert e(sample)==(worker<=100 | "`pop'"=="both")
    * An independent dummy-variable OLS regression checks the plug-in targets.
    preserve
        keep if e(sample)
        quietly regress y `controls' ib1.worker ib1.firm
        gen double ahat=_b[_cons]
        gen double phat=0
        quietly levelsof worker,local(workers)
        foreach w of local workers {
            if `w'!=1 quietly replace ahat=ahat+_b[`w'.worker] if worker==`w'
        }
        forvalues f=2/17 {
            quietly replace phat=_b[`f'.firm] if firm==`f'
        }
        mata: Z=st_data(.,("ahat","phat")); w=st_data(.,"mass"); w=w/sum(w); Z=Z:-colsum(w:*Z); V=Z'*(w:*Z); st_matrix("oracle",(V[1,1],V[2,2],V[1,2],sum(V)))
        assert mreldif(plugin,oracle)<1e-8
    restore
    foreach method in exact jla {
        quietly fevc y `controls', `options' algorithm(`method')
        matrix reference=e(results)
        assert e(sample)==(worker<=100 | "`pop'"=="both")
        if "`method'"=="jla" {
            mata: a=st_matrix("exact"); b=st_matrix("reference"); assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),6*b[4,.]'))'))
            assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
        }
        quietly fevc y `transformed', `options' algorithm(`method')
        mata: a=st_matrix("reference"); b=st_matrix("e(results)"); assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),.1*rowmax((a[4,.]',b[4,.]'))))'))
        assert `"`c(rngstate)'"'==`"`caller_rng'"'
        quietly _datasignature
        assert `"`r(datasignature)'"'==`"`signature'"'
    }
}
noi di "PASS test_control_panel.do"
