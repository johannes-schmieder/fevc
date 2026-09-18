version 18.0
clear all
set more off
set varabbrev off
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
// Two deletion units per two-row coefficient cell: do not silently replace
// this identifier with the worker-firm key when selecting the direct solver.
generate long unit = key
sort worker firm key
local caller_state `"`c(rngstate)'"'
local caller_sort `"`c(sortrngstate)'"'
local caller_order : sortedby
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach deletion in match observation {
    foreach engine in auto generic {
        foreach weighted in 0 1 {
            local fw
            local extra
            if `weighted' {
                local fw [fw=copies]
                local extra targetweight(mass)
                if "`deletion'"=="match" local extra `extra' deletionid(unit)
            }
            // An explicit single-column batch retains the legacy solver and
            // identical statistical family/Counter draws for this oracle.
            quietly fevc y `fw', worker(worker) firm(firm) deletion(`deletion') ///
                backend(rust) rng(counter_v1) algorithm(jla) engine(`engine') ///
                preconditioner(cmg) batch(1) probes(9) seed(81227) ///
                tolerance(1e-10) `extra' nodisplay
            matrix reference = e(kss)
            local family `"`e(engine_selected)'"'
            local retained = e(N_retained)
            generate byte reference_sample = e(sample)
            foreach route in auto cmg {
                fevc y `fw', worker(worker) firm(firm) deletion(`deletion') ///
                    backend(rust) rng(counter_v1) algorithm(jla) engine(`engine') ///
                    preconditioner(`route') batch(auto) probes(9) seed(81227) ///
                    tolerance(1e-10) `extra' nodisplay
                assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
                assert `"`e(preconditioner_requested)'"'=="`route'"
                assert `"`e(preconditioner_selected)'"'=="CMG"
                assert `"`e(engine_selected)'"'=="`family'"
                assert `"`e(stayers)'"'=="both"
                assert e(N_retained)==`retained'
                assert e(sample)==reference_sample
                assert e(cmg_threads_used)==4
                assert e(rust_probeorder_supplied)==0
                assert e(memory_budget_supplied)==0
                assert e(full_cmg_receipt)[1,25]==1+3*e(probes)
                assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
                matrix actual = e(kss)
                forvalues column=1/4 {
                    assert abs(actual[1,`column']-reference[1,`column'])<=1e-8*max(1,abs(reference[1,`column']))
                }
                assert `"`c(rngstate)'"'==`"`caller_state'"'
                assert `"`c(sortrngstate)'"'==`"`caller_sort'"'
                quietly fevc_rust snapshot
                assert r(state)==0 & r(handle)==0
            }
            // Explicit diagonal remains diagonal, never disguised as CMG.
            quietly fevc y `fw', worker(worker) firm(firm) deletion(`deletion') ///
                backend(rust) rng(counter_v1) algorithm(jla) engine(generic) ///
                preconditioner(diagonal) batch(auto) probes(9) seed(81227) ///
                tolerance(1e-10) `extra' nodisplay
            assert `"`e(cmg_backend)'"'==""
            assert `"`e(preconditioner_requested)'"'=="diagonal"
            drop reference_sample
        }
    }
}
// Omitted point tolerance retains the production fit/probe split on the new
// explicit generic weighted path, with no implicit memory budget.
quietly fevc y [fw=copies], worker(worker) firm(firm) deletion(observation) ///
    backend(rust) rng(counter_v1) algorithm(jla) engine(generic) ///
    preconditioner(cmg) batch(auto) probes(33) seed(81227) ///
    targetweight(mass) nodisplay
assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
assert e(full_cmg_receipt)[1,18]==1e-10
assert e(full_cmg_receipt)[1,19]==1e-6
assert e(leverage_batch)==32 & e(target_batch)==32
assert e(memory_budget_supplied)==0
assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
// Supplied flags describe provenance, not permission to use an already
// supported effective point tuple. Omitted defaults must not lose threading.
quietly fevc y [fw=copies], worker(worker) firm(firm) deletion(observation) ///
    backend(rust) engine(generic) probes(33) seed(81227) targetweight(mass) nodisplay
assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
assert `"`e(rng_requested)'"'=="auto"
assert `"`e(rng_selected)'"'=="counter_v1"
assert e(algorithm_option_supplied)==0
assert e(preconditioner_option_supplied)==0
assert e(batch_option_supplied)==0
assert e(cmg_threads_used)==4
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
local restored_order : sortedby
assert `"`restored_order'"'==`"`caller_order'"'
assert `"`c(rngstate)'"'==`"`caller_state'"'
assert `"`c(sortrngstate)'"'==`"`caller_sort'"'
display "PASS test_rust_point_routes_full_cmg.do"
