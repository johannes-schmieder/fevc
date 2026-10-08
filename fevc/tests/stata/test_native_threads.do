version 18
clear all
set more off
set processors 4
args package upper output
if "`upper'"=="" local upper 8
if "`output'"!="" log using "`output'",text replace
adopath ++ "`package'"
set obs 288
gen long key=_n
gen long worker=ceil(key/8)
gen byte slot=mod(key-1,8)
gen int firm=floor(slot/2)+1
replace firm=mod(worker-1,4)+1 if key>256
gen double x=(worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
gen double y=.3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x
gen double mass=.7+mod(13*key,11)/7
gen double z=mod(worker,7)
local state `"`c(rngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
local common worker(worker) firm(firm) targetweight(mass) backend(rust) rng(counter_v1) probes(64) seed(81227) nodisplay
foreach nuisance in joint fixedoffset {
 foreach population in movers both {
  foreach path in exact auto diagonal cmg autojla {
   local route algorithm(`path')
   if "`path'"=="diagonal" local route algorithm(jla) engine(generic) preconditioner(diagonal) batch(4)
   if "`path'"=="cmg" local route algorithm(jla) engine(generic) preconditioner(cmg)
   if "`path'"=="autojla" local route algorithm(jla)
   foreach threads in 4 `upper' {
    quietly fevc y x, `common' `route' nuisance(`nuisance') stayers(`population') nativethreads(`threads')
    assert c(processors)==4
    assert e(active_processors)==4
    assert e(native_threads_requested)==`threads'
    assert e(native_threads_selected)==`threads'
    assert inrange(e(native_threads_capacity),1,`threads')
    assert "$VCKSS_NATIVE_SELECTED"==""
    matrix now=e(results)
    if `threads'==4 matrix base=now
    else mata: a=st_matrix("base");b=st_matrix("now");assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),.1*rowmax((a[4,.]',b[4,.]'))))'))
   }
   noi di "THREAD_PATH_PASS `nuisance' `population' `path'"
  }
 }
}
// No-control dispatch also includes the compressed native engine.
foreach path in exact auto_diagonal auto_cmg generic_cmg {
 local route algorithm(exact)
 if "`path'"=="auto_diagonal" local route algorithm(jla) engine(auto) preconditioner(diagonal)
 if "`path'"=="auto_cmg" local route algorithm(jla) engine(auto) preconditioner(cmg)
 if "`path'"=="generic_cmg" local route algorithm(jla) engine(generic) preconditioner(cmg)
 foreach threads in 4 `upper' {
  quietly fevc y, `common' `route' deletion(match) stayers(movers) nativethreads(`threads')
  if inlist("`path'","auto_diagonal","auto_cmg") assert "`e(engine_selected)'"=="compressed"
  assert e(native_threads_selected)==`threads'
  assert inrange(e(native_threads_capacity),1,`threads')
  assert c(processors)==4
  if `threads'==4 matrix base=e(results)
  else {
   matrix now=e(results)
   mata: a=st_matrix("base");b=st_matrix("now");assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),.1*rowmax((a[4,.]',b[4,.]'))))'))
  }
 }
 noi di "THREAD_NO_CONTROL_PASS `path'"
}
foreach invalid in 0 65 2.5 missing {
 capture quietly fevc y x, `common' nativethreads(`invalid')
 assert _rc==198
 assert "`e(status)'"=="WITHHELD"
 assert "`e(withholding_status)'"=="INVALID_NATIVE_THREADS"
 assert "$VCKSS_NATIVE_SELECTED"==""
}
assert `"`c(rngstate)'"'==`"`state'"'
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
// Independently retained positive attachment fixtures from the execution suite.
clear
set obs 240
gen long worker=floor((_n-1)/6)
gen byte time=mod(_n-1,6)
gen long firm=mod(worker+floor(time/2),20)
gen double x=time-2.5
gen double z=sin(worker/5)+cos(firm/3)+time/20
gen double y=-4+.08*worker-.12*firm+.3*x+.25*sin((_n*17)/11)+.15*cos((_n*7)/13)
foreach solver in diagonal cmg {
 foreach threads in 4 `upper' {
  quietly fevc y x, worker(worker) firm(firm) deletion(observation) stayers(movers) ///
   algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
   probes(200) tolerance(1e-12) centering(none) project(z) projecteffect(firm) nodisplay nativethreads(`threads')
  assert e(rust_execution_receipt)[1,"threads"]==`threads'
  assert c(processors)==4
  if `threads'==4 matrix base=e(projection_b)
  else assert mreldif(base,e(projection_b))<1e-8
 }
 noi di "THREAD_ATTACHMENT_PASS projection `solver'"
}
clear
set obs 800
gen long worker=floor((_n-1)/40)
gen long firm=floor(mod(_n-1,40)/2)
gen long match=floor((_n-1)/2)+1
gen double x=.31*mod(_n-1,2)+mod(_n-1,7)/29
gen double y=worker-.8*firm+1.4*x+(mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
gen double target=(.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
foreach solver in diagonal cmg {
 foreach threads in 4 `upper' {
  quietly fevc y x, worker(worker) firm(firm) stayers(movers) deletion(observation) ///
   algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
   probes(200) targetweight(target) centering(none) inference(highrank) inferencemodel(structured_common) ///
   inferencesimulations(129) inferencegramprobes(513) nodisplay nativethreads(`threads')
  assert e(rust_execution_receipt)[1,"threads"]==`threads'
  assert e(rust_component_batch_receipt)[1,"threads"]==`threads'
  assert c(processors)==4
  if `threads'==4 matrix base=e(kss)
  else assert mreldif(base,e(kss))<1e-8
 }
 noi di "THREAD_ATTACHMENT_PASS component `solver'"
}
noi di "PASS test_native_threads.do"
if "`output'"!="" log close
