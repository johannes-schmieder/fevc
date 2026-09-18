version 18.0
clear all
set more off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 4
set obs 2112
generate long key = _n
generate long worker = ceil(key/8)
generate byte slot = mod(key-1,8)
generate int firm = mod(worker-1+3*floor(slot/2),128)+1
replace firm = mod(worker-1,128)+1 if key>2048
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101
generate byte copies = 1+mod(key,3)
generate double mass = .7+mod(13*key,11)/7
generate long unit = key
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach engine in auto generic {
    quietly fevc y [fw=copies], worker(worker) firm(firm) deletion(match) ///
        stayers(movers) probeorder(key) deletionid(unit) targetweight(mass) ///
        backend(rust) rng(counter_v1) algorithm(jla) engine(`engine') ///
        preconditioner(cmg) batch(1) probes(33) seed(81227) tolerance(1e-10) nodisplay
    matrix reference = e(kss)
    generate byte reference_sample = e(sample)
    quietly fevc y [fw=copies], worker(worker) firm(firm) deletion(match) ///
        stayers(movers) probeorder(key) deletionid(unit) targetweight(mass) ///
        backend(rust) rng(counter_v1) algorithm(jla) engine(`engine') ///
        preconditioner(cmg) batch(auto) probes(33) seed(81227) tolerance(1e-10) nodisplay
    assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
    assert e(rust_probeorder_supplied)==1
    assert e(N_retained)==2048
    assert e(deletion_units)==2048
    assert e(coefficient_cells)==1024
    assert e(sample)==reference_sample
    matrix actual = e(kss)
    forvalues column=1/4 {
        assert abs(actual[1,`column']-reference[1,`column'])<=1e-8*max(1,abs(reference[1,`column']))
    }
    assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
    drop reference_sample
}
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
display "WEIGHTED_KEY_DECLARED_DELETION_PASS"
