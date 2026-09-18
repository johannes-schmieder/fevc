version 18.0
clear all
set more off
set varabbrev off
args package_dir trace
confirm file `"`package_dir'/fevc.ado"'
adopath ++ `"`package_dir'"'
quietly fevc_rust probe
local original_flags = r(core_ready_flags)
local plugin _fevc_rust_linux
if strpos("`c(machine_type)'", "Mac")==1 local plugin _fevc_rust_macos
_fevc_rust_plugin_call `plugin', exactexecutioncapability
assert __vckss_exact_execution_api==1
set processors 4
set obs 99
generate long key = _n
generate long worker = floor((_n-1)/8)+1
generate int firm = mod(floor((_n-1)/2),4)+1
replace worker = 14 in 99
generate long stayer_worker = worker-12 if worker>12
replace firm = 1 in 97/98
replace firm = 2 in 99
generate long deletion_id = floor((_n-1)/2)+1
generate double y = .7*worker-.45*firm+.3*mod(key,2)+mod(7*key,5)/11
generate double control = (worker-.4*firm)*(mod(key,2)+1)+mod(3*key,7)/17
generate byte frequency = mod(key,3)+1
replace frequency = 2 in 99
generate double target = .5+mod(5*key,7)/3
sort key
local rng `"`c(rngstate)'"'
local sort_rng `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'

foreach deletion in observation match {
    foreach nuisance in joint fixedoffset {
        foreach population in movers both {
            local hybrid = ("`deletion'"=="match" & "`population'"=="both")
            local native_stayers = cond(`hybrid',"all","movers")
            local deletion_source = cond("`deletion'"=="match","matchid","observation")
            quietly fevc_rust requestcapability, algorithm(exact) deletion(`deletion') ///
                nuisance(`nuisance') route(auto) rngcontract(none) controls(1) ///
                frequencyused(1) engine(auto) batchmode(auto) leveragebatchmode(auto) ///
                targetbatchmode(auto) stayers(`native_stayers') targetweightmode(explicit) ///
                deletionsource(`deletion_source') probeordersupplied(0) wallsecondssupplied(0) ///
                fallback(1) wallseconds(0) physicallimit(50000000)
            assert r(supported)==1 & r(request_schema)==3 & r(profile_code)==4
            local hi = strtrim(strofreal(r(request_signature_hi),"%21.0f"))
            local lo = strtrim(strofreal(r(request_signature_lo),"%21.0f"))
            foreach threads in 0 1 4 7 {
                tempvar keep
                local marked if worker<=12
                if "`deletion'"=="observation" & "`population'"=="both" local marked
                quietly fevc_rust prepare worker firm deletion_id y frequency target control `marked', ///
                    cleanup generate(`keep') memorygib(.5) deletion(`deletion')
                local handle = r(handle)
                if `hybrid' {
                    quietly fevc_rust augmentstayers firm stayer_worker y frequency target control if worker>12, ///
                        handle(`handle')
                }
                local selector solve
                local thread_argument
                if `threads' {
                    local selector solveexactexecution
                    local thread_argument `threads'
                }
                display as text "EXACT_SMOKE deletion=`deletion' nuisance=`nuisance' population=`population' threads=`threads' handle=`handle' signature=`hi':`lo'"
                if "`trace'"=="1" set trace on
                // Raw plugin arguments must not contain bare negative
                // exponents: Stata's plugin parser turns 1e-12 into 1e.
                _fevc_rust_plugin_call `plugin', `selector' `handle' 81227 7 0 0 auto .000000000001 10000 ///
                    exact `deletion' `nuisance' 500 5000 .0000000001 .0000000001 auto auto `native_stayers' ///
                    explicit `deletion_source' 0 0 50000000 3 4 1 `hi' `lo' auto auto 1 0 `thread_argument'
                set trace off
                // Read the raw result and separate opt-in receipt. Ordinary
                // Stata exact reconciliation must continue to reject schema 2.
                _fevc_rust_plugin_call `plugin', result `handle'
                local prefix corrected
                if `hybrid' {
                    _fevc_rust_plugin_call `plugin', stayerresult `handle'
                    local prefix hyb_corrected
                }
                matrix current = (__vckss_`prefix'_worker,__vckss_`prefix'_firm, ///
                    __vckss_`prefix'_cov,__vckss_`prefix'_total)
                if !`threads' {
                    matrix reference = current
                    assert __vckss_plan_route_schema==1
                    assert __vckss_plan_threads_used==1 & __vckss_plan_threads_req==1
                }
                else {
                    _fevc_rust_plugin_call `plugin', exactexecutionreceipt `handle'
                    assert __vckss_eex_threads==`threads'
                    assert __vckss_eex_workers>0 & __vckss_eex_workers<=`threads'
                    assert __vckss_eex_passes==1+`hybrid'
                    assert __vckss_eex_residual<=1e-9
                    assert __vckss_plan_route_schema==2
                    assert __vckss_eex_peak==__vckss_mem_command
                    assert mreldif(current,reference)<1e-8
                }
                quietly fevc_rust release `handle'
                quietly fevc_rust snapshot
                assert r(state)==0 & r(handle)==0
                drop `keep'
                assert `"`c(rngstate)'"'==`"`rng'"'
                assert `"`c(sortrngstate)'"'==`"`sort_rng'"'
            }
        }
    }
}
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
quietly fevc_rust probe
assert r(core_ready_flags)==`original_flags'
display as result "PASS exact_transport_smoke.do"
