version 18
clear all
set more off
args pkgroot backend
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
if "`backend'"=="" local backend mata
adopath ++ `"`pkgroot'"'
set obs 120
gen long w=ceil(_n/6)
gen long f=1+mod(floor((_n-1)/3),3)
replace f=1 if w>=19
gen double x=sin(_n*.27)+cos(_n*.81)
gen double y=5+sin(w*.5)+f*.2+.4*x+sin(_n*.93)
gen int copies=1+mod(_n,3)
gen double mass=copies*(1+.1*mod(_n,4))
gen double shifted=y
local cells=0
foreach algorithm in exact jla {
 foreach deletion in match observation {
  foreach stay in movers both {
   foreach controls in no joint fixedoffset {
    local covars
    local nuisance joint
    if "`controls'"!="no" local covars x
    if "`controls'"=="fixedoffset" local nuisance fixedoffset
    local common worker(w) firm(f) algorithm(`algorithm') backend(`backend') deletion(`deletion') stayers(`stay') nuisance(`nuisance') probes(32) seed(12345) targetweight(mass) preconditioner(auto) tolerance(1e-12) rank_tolerance(1e-12) nodisplay
    di "CELL `algorithm' `deletion' `stay' `controls'"
    quietly fevc y `covars'  [fw=copies], `common' centering(mean) mcse(all)
    matrix point=e(kss)
    matrix se=e(mcse)
    matrix cov=e(mcse_cov)
    gen byte keep=e(sample)
    quietly sum y [fw=copies] if keep, meanonly
    local center=r(mean)
    if "`controls'"=="fixedoffset" {
        quietly regress y x i.w i.f [fw=copies] if keep
        local gamma=_b[x]
        quietly sum x [fw=copies] if keep, meanonly
        local center=`center'-`gamma'*r(mean)
    }
    quietly replace shifted=y-`center'
    quietly fevc shifted `covars' [fw=copies], `common' centering(none) mcse(all)
    di "DIFF point " mreldif(point,e(kss)) " se " mreldif(se,e(mcse)) " cov " mreldif(cov,e(mcse_cov))
    assert mreldif(point,e(kss))<1e-8
    assert mreldif(se,e(mcse))<1e-8
    assert mreldif(cov,e(mcse_cov))<1e-8
    drop keep
    local ++cells
   }
  }
 }
}
di "SIMPLE_MEAN_PASS cells=`cells' backend=`backend'"
