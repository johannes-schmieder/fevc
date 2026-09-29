version 18
clear all
set more off
set processors 4
args package fixture output upper
if "`upper'"=="" local upper 8
log using "`output'", text replace
adopath ++ "`package'"
import delimited using "`fixture'/input.csv", clear varnames(1) asdouble
run "`fixture'/oracle.do"
local controls
forvalues j=1/13 {
 local controls `controls' x`j'
}
gen double z1=x1+.5*x13
gen double z2=8*x2
gen double z3=x3-.25*x4
local transformed z1 z2 z3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13
// x^2(1+x), x^2(1-x) span the same quadratic/cubic pair in each education group.
local polynomial
forvalues j=1(2)5 {
 local k=`j'+1
 gen double polynomial`j'=x`j'+x`k'
 gen double polynomial`k'=x`j'-x`k'
 local polynomial `polynomial' polynomial`j' polynomial`k'
}
local polynomial `polynomial' x7 x8 x9 x10 x11 x12 x13
gen long rowkey=_n
gen double duplicate=x1
local reverse x13 x12 x11 x10 x9 x8 x7 x6 x5 x4 x3 x2 x1
quietly _datasignature
local signature `"`r(datasignature)'"'
local caller `"`c(rngstate)'"'
foreach population in movers both {
 foreach weight in native earnings {
  foreach nuisance in joint fixedoffset {
   local label=cond("`nuisance'"=="joint","joint","offset")
   local opts worker(worker) firm(firm) deletionid(match) deletion(match) stayers(`population') nuisance(`nuisance') targetweight(`weight') nodisplay
   foreach backend in rust mata {
    quietly fevc y `controls', `opts' backend(`backend') algorithm(exact)
    mata: a=st_matrix("e(kss)");b=st_matrix("o_`population'_`weight'_`label'");assert(all(abs(a-b):<=1e-8:*rowmax((J(4,1,1),abs(a'),abs(b')))' ))
   }
   foreach seed in 92826 92827 {
    quietly fevc y `controls', `opts' backend(rust) algorithm(jla) rng(counter_v1) probes(512) seed(`seed') nativethreads(4)
    matrix base=e(results)
    mata: a=st_matrix("base");b=st_matrix("o_`population'_`weight'_`label'");assert(all(abs(a[3,.]-b):<=rowmax((J(4,1,1e-8),6*a[4,.]'))'))
    foreach change in transformed reverse polynomial {
     quietly fevc y ``change'', `opts' backend(rust) algorithm(jla) rng(counter_v1) probes(512) seed(`seed') nativethreads(`upper')
     matrix changed=e(results)
     mata: a=st_matrix("base");b=st_matrix("changed");assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),.1*rowmax((a[4,.]',b[4,.]'))))'))
    }
   }
   // Duplicate columns are rejected under the current strict rank policy.
   foreach backend in rust mata {
    foreach algorithm in exact jla {
     capture quietly fevc y `controls' duplicate, `opts' backend(`backend') algorithm(`algorithm')
     assert _rc==498
     assert "`e(status)'"=="WITHHELD"
     assert "`e(withholding_status)'"==cond("`algorithm'"=="exact","SINGULAR_INFORMATION","SINGULAR_NUISANCE_BLOCK")
    }
   }
   preserve
   gsort -rowkey
   quietly fevc y `controls', `opts' backend(rust) algorithm(jla) rng(counter_v1) probes(512) seed(92827) nativethreads(`upper')
   matrix changed=e(results)
   mata: a=st_matrix("base");b=st_matrix("changed");assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),.1*rowmax((a[4,.]',b[4,.]'))))'))
   restore
   assert `"`c(rngstate)'"'==`"`caller'"'
   quietly _datasignature
   assert `"`r(datasignature)'"'==`"`signature'"'
   noi di "PASS_SPAN population=`population' weight=`weight' nuisance=`nuisance'"
  }
 }
}
noi di "PASS test_control_span.do"
log close
