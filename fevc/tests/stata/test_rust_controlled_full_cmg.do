version 18.0
clear all
set more off
set varabbrev off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 4
set obs 288
generate long key = _n
generate long worker = ceil(key/8)
generate byte slot = mod(key-1,8)
generate int firm = floor(slot/2)+1
replace firm = mod(worker-1,4)+1 if key>256
generate double x = (worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
generate double z = mod(13*key,23)/11+sin(key)
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x-.1*z
generate byte copies = 1+mod(key,3)
generate double mass = .7+mod(13*key,11)/7
generate long unit = key
sort worker firm key
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach deletion in match observation {
    local id
    if "`deletion'"=="match" local id deletionid(unit)
    foreach nuisance in joint fixedoffset {
        foreach population in both movers {
            local pop
            if "`population'"=="movers" local pop stayers(movers)
            quietly fevc y x z [fw=copies], worker(worker) firm(firm) deletion(`deletion') ///
                nuisance(`nuisance') `id' `pop' targetweight(mass) algorithm(jla) ///
                backend(rust) rng(counter_v1) engine(generic) preconditioner(cmg) ///
                batch(1) probes(9) seed(81227) nodisplay
            matrix reference = e(kss)
            generate byte reference_sample = e(sample)
            foreach route in auto cmg {
                fevc y x z [fw=copies], worker(worker) firm(firm) deletion(`deletion') ///
                    nuisance(`nuisance') `id' `pop' targetweight(mass) algorithm(jla) ///
                    backend(rust) rng(counter_v1) engine(auto) preconditioner(`route') ///
                    batch(auto) probes(9) seed(81227) nodisplay
                assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
                assert `"`e(engine_selected)'"'=="generic"
                assert `"`e(stayers)'"'=="`population'"
                assert e(sample)==reference_sample
                assert e(cmg_threads_used)==4
                assert e(memory_budget_supplied)==0
                assert e(full_cmg_receipt)[1,18]==1e-10
                assert e(full_cmg_receipt)[1,19]==1e-10
                assert `"`e(full_cmg_model_schema)'"'=="CMG-FULL-MODEL-V1"
                assert e(full_cmg_model_receipt)[1,1]==2
                assert e(full_cmg_model_receipt)[1,3]==30+("`nuisance'"=="fixedoffset")
                assert e(full_cmg_model_receipt)[1,4]==2
                assert e(full_cmg_model_receipt)[1,5]==cond("`nuisance'"=="joint",19,1)
                assert e(full_cmg_receipt)[1,25]==e(full_cmg_model_receipt)[1,3]+e(full_cmg_model_receipt)[1,6]
                assert e(rust_rhs_receipts)[1,13]==1e-11
                assert e(rust_rhs_receipts)[2,13]==1e-11
                assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
                matrix actual = e(kss)
                forvalues col=1/4 {
                    assert abs(actual[1,`col']-reference[1,`col'])<=1e-8*max(1,abs(reference[1,`col']))
                }
                assert `"`c(rngstate)'"'==`"`state'"'
                assert `"`c(sortrngstate)'"'==`"`sortstate'"'
                quietly fevc_rust snapshot
                assert r(state)==0 & r(handle)==0
            }
            drop reference_sample
        }
    }
}
// Empty control blocks and a semantic key must not request raw implicit
// ingestion on the newly supported fixed-offset path.
foreach deletion in match observation {
    quietly fevc y, worker(worker) firm(firm) deletion(`deletion') stayers(movers) ///
        nuisance(fixedoffset) algorithm(jla) backend(rust) rng(counter_v1) ///
        probeorder(key) probes(9) seed(81227) nodisplay
    assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
    assert e(full_cmg_model_receipt)[1,1]==0
    assert e(full_cmg_model_receipt)[1,3]==28
    assert e(full_cmg_receipt)[1,19]==1e-6
}
// Controlled admission does not borrow the relaxed no-control probe default.
quietly fevc y x z [fw=copies], worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) backend(rust) rng(counter_v1) probes(33) seed(81227) ///
    tolerance(1e-12) targetweight(mass) nodisplay
assert e(full_cmg_receipt)[1,19]==1e-12
assert e(leverage_batch)==32 & e(target_batch)==32
// Explicit strict budgets fail closed and leave the caller and registry reusable.
capture noisily fevc y x z [fw=copies], worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) backend(rust) rng(counter_v1) probes(9) seed(81227) ///
    targetweight(mass) memory_gib(.000001) memorycheck(error) nodisplay
assert _rc!=0
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
quietly fevc y x z [fw=copies], worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) backend(rust) rng(counter_v1) probes(9) seed(81227) ///
    targetweight(mass) memory_gib(1) memorycheck(error) nodisplay
assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
assert e(memory_budget_supplied)==1
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
display "PASS test_rust_controlled_full_cmg.do"
