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
gen double x=sin(_n*.31)
gen double y=5+sin(w*.7)+f*.2+.3*x+cos(_n*.9)
gen double shifted=y+20
local common worker(w) firm(f) backend(`backend') stayers(movers) nodisplay
set seed 7654
local before `"`c(rngstate)'"'
capture noisily fevc y, `common' centering(unknown)
assert _rc==198
capture noisily fevc y, `common' centering(mean) inference(observation)
assert _rc==498
capture noisily fevc y, `common' centering(corrected) project(x)
assert _rc==498
capture noisily fevc y, `common' inference(highrank)
assert _rc==498
assert "`e(withholding_status)'"=="CENTERING_INFERENCE_UNSUPPORTED"
capture noisily fevc y, `common' project(x) inference(highrank)
assert _rc==498
assert "`e(withholding_status)'"=="CENTERING_INFERENCE_UNSUPPORTED"
capture noisily fevc y, `common' algorithm(jla) centering(corrected) probes(3) mcse(off)
assert _rc!=0
assert `"`c(rngstate)'"'==`"`before'"'
assert "$VCKSS_CENTERING"==""
quietly fevc y, `common' algorithm(exact) centering(corrected) probes(3) mcse(off)
assert "`e(centering)'"=="corrected"
assert el(e(mcse),1,1)==0 | missing(el(e(mcse),1,1))
foreach algorithm in exact jla {
 quietly fevc y x, `common' algorithm(`algorithm') probes(32) seed(1234) mcse(all)
 assert "`e(centering)'"=="mean"
 assert "`e(mcse_centering)'"=="fixed observed mean"
 matrix default_point=e(kss)
 matrix default_se=e(mcse)
 matrix default_cov=e(mcse_cov)
 gen byte default_sample=e(sample)
 quietly fevc y x, `common' algorithm(`algorithm') centering(mean) probes(32) seed(1234) mcse(all)
 assert mreldif(default_point,e(kss))<1e-12
 assert mreldif(default_se,e(mcse))<1e-12
 assert mreldif(default_cov,e(mcse_cov))<1e-12
 assert default_sample==e(sample)
 drop default_sample
 quietly fevc y x, `common' algorithm(`algorithm') centering(none) probes(32) seed(1234) mcse(all)
 assert "`e(centering)'"=="none"
 assert "`e(mcse_centering)'"=="uncentered"
 assert mreldif(default_point,e(kss))>1e-7
 foreach mode in mean corrected {
  quietly fevc y x, `common' algorithm(`algorithm') centering(`mode') probes(32) seed(1234) mcse(off)
  matrix expected=e(kss)
  assert "`e(centering)'"=="`mode'"
  quietly fevc shifted x, `common' algorithm(`algorithm') centering(`mode') probes(32) seed(1234) mcse(off)
  assert mreldif(expected,e(kss))<1e-8
 }
}
assert `"`c(rngstate)'"'==`"`before'"'
assert "$VCKSS_CENTERING"==""
di "SIMPLE_CENTERING_OPTIONS_PASS backend=`backend'"
