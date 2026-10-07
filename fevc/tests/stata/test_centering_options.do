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
capture noisily fevc y, `common' algorithm(jla) centering(corrected) probes(3) mcse(off)
assert _rc!=0
assert `"`c(rngstate)'"'==`"`before'"'
assert "$VCKSS_CENTERING"==""
quietly fevc y, `common' algorithm(exact) centering(corrected) probes(3) mcse(off)
assert "`e(centering)'"=="corrected"
assert el(e(mcse),1,1)==0 | missing(el(e(mcse),1,1))
foreach algorithm in exact jla {
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
