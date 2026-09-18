version 18.0
clear all
set more off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 1
set obs 288
generate long key = _n
generate long worker = ceil(key/8)
generate byte slot = mod(key-1,8)
generate int firm = floor(slot/2)+1
replace firm = mod(worker-1,4)+1 if key>256
generate double x = (worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x
generate byte copies = 1+mod(key,3)
generate double mass = .7+mod(13*key,11)/7
fevc y x [fw=copies], worker(worker) firm(firm) deletion(observation) nuisance(joint) ///
    stayers(both) targetweight(mass) algorithm(jla) backend(rust) rng(counter_v1) ///
    engine(generic) probes(33) seed(81227) preconditioner(diagonal) batch(auto) nodisplay
ereturn list
matrix list e(rust_execution_receipt)
display as result "PASS execution_diagnostic.do"
