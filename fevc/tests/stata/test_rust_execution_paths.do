version 18.0
clear all
set more off
set varabbrev off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
quietly fevc_rust probe
assert r(execution_api)==3
capture confirm scalar __vckss_rust_execution_api
assert _rc!=0
// Independent Python struct.pack("<18IQd") / FNV-1a vectors for the V3
// request identity. Preserve exact binary64 wall bytes and a >32-bit limit.
matrix signatures = (0,664754525,462816238 \ .1,2809327863,2676726163 \ ///
    1.25,695919709,489281203 \ 45,3550917978,2822617336 \ ///
    1e-300,1144819214,3646670875 \ 1e100,3894867578,1172526385)
forvalues row = 1/6 {
    local wall = signatures[`row',1]
    quietly _vckss_request_signature 3 2 2 1 2 1 1 0 2 1 1 0 3 0 1 ///
        4294967297 1 1 0 `wall'
    assert r(signature_hi)==signatures[`row',2]
    assert r(signature_lo)==signatures[`row',3]
}
set processors 4
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
sort worker firm key
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach deletion in observation match {
    foreach nuisance in joint fixedoffset {
        foreach population in both movers {
            local common worker(worker) firm(firm) deletion(`deletion') nuisance(`nuisance') ///
                stayers(`population') targetweight(mass) algorithm(jla) backend(rust) ///
                rng(counter_v1) engine(generic) probes(33) seed(81227) nodisplay
            quietly fevc y x [fw=copies], `common' preconditioner(cmg) batch(1)
            matrix reference = e(kss)
            generate byte reference_sample = e(sample)
            foreach threads in 1 4 7 {
                capture quietly set processors `threads'
                if _rc {
                    display "EXECUTION_THREADS_SKIPPED threads=`threads' license_or_slot_limit"
                    continue
                }
                foreach width in auto 7 8 {
                    display "EXECUTION_ATTEMPT deletion=`deletion' nuisance=`nuisance' population=`population' threads=`threads' batch=`width'"
                    noisily fevc y x [fw=copies], `common' preconditioner(diagonal) batch(`width')
                    assert "`e(rust_execution_mode)'"=="diagonal_queue"
                    assert "`e(rust_execution_schema)'"=="VCKSS-GENERIC-EXECUTION-V1"
                    matrix work = e(rust_execution_receipt)
                    assert work[1,"threads"]==`threads'
                    assert work[1,"mode"]==1 & work[1,"rank"]==1
                    assert work[1,"point"]==99 & work[1,"projection"]==0
                    assert work[1,"component"]==0 & work[1,"gram"]==0
                    assert work[1,"fit"]==1+("`nuisance'"=="fixedoffset")
                    assert work[1,"logical"]==work[1,"queued"]+work[1,"fit"]
                    assert work[1,"active"]>=1 & work[1,"active"]<=`threads'
                    assert work[1,"cmg"]==0 & work[1,"cmg_concurrency"]==0
                    assert work[1,"residual"]<=e(residual_acceptance_tolerance)
                    if "`width'"=="auto" {
                        assert e(leverage_batch)==min(33,max(32,8*`threads'))
                        assert e(target_batch)==min(33,max(32,4*`threads'))
                    }
                    else assert e(leverage_batch)==`width' & e(target_batch)==`width'
                    assert e(sample)==reference_sample
                    assert mreldif(e(kss),reference)<1e-8
                    assert e(memory_budget_supplied)==0
                    assert `"`c(rngstate)'"'==`"`state'"'
                    assert `"`c(sortrngstate)'"'==`"`sortstate'"'
                    quietly fevc_rust snapshot
                    assert r(state)==0 & r(handle)==0
                }
            }
            drop reference_sample
        }
    }
}
set processors 4
// Strict budget rejection and advisory/off success preserve explicit widths.
foreach policy in error warn off {
    capture noisily fevc y x [fw=copies], worker(worker) firm(firm) deletion(observation) ///
        algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
        batch(8) probes(33) memory_gib(.000001) memorycheck(`policy') nodisplay
    if "`policy'"=="error" assert _rc!=0
    else {
        assert _rc==0
        assert e(leverage_batch)==8 & e(target_batch)==8
    }
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'

// Corrupt every work-receipt field after native validation; the public layer
// must still withhold and restore the prepared lifecycle and caller state.
capture program drop _fevc_rust_public_call
program define _fevc_rust_public_call, rclass
    version 18.0
    gettoken command rest : 0
    tempname work
    if "`command'"=="solve" & "$FEVC_EXECUTION_FAULT"=="break" exit 1
    fevc_rust `command' `rest'
    if "`command'"=="executionreceipt" & "$FEVC_EXECUTION_FAULT"!="" {
        matrix `work' = r(receipt)
        matrix `work'[1,real("$FEVC_EXECUTION_FAULT")] = -1
    }
    return add
    if "`command'"=="executionreceipt" & "$FEVC_EXECUTION_FAULT"!="" return matrix receipt = `work'
end
foreach fault in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 break {
    global FEVC_EXECUTION_FAULT `fault'
    capture noisily fevc y x, worker(worker) firm(firm) deletion(observation) ///
        algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
        batch(8) probes(9) nodisplay
    if "`fault'"=="break" assert _rc==1
    else assert _rc==498
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
    assert `"`c(rngstate)'"'==`"`state'"'
    assert `"`c(sortrngstate)'"'==`"`sortstate'"'
    quietly _datasignature
    assert `"`r(datasignature)'"'==`"`signature'"'
}
macro drop FEVC_EXECUTION_FAULT
capture program drop _fevc_rust_public_call
program define _fevc_rust_public_call, rclass
    version 18.0
    fevc_rust `0'
    return add
end
quietly fevc y x, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(8) probes(9) nodisplay
assert "`e(rust_execution_mode)'"=="diagonal_queue"

// Explicit CMG point batches remain the legacy path; attachments with omitted
// point batches select the new direct executor, with strict tolerances.
clear
set obs 240
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker+floor(time/2),20)
generate double x = time-2.5
generate double z = sin(worker/5)+cos(firm/3)+time/20
generate double y = -4+.08*worker-.12*firm+.3*x+.25*sin((_n*17)/11)+.15*cos((_n*7)/13)
foreach solver in diagonal cmg {
    quietly fevc y x, worker(worker) firm(firm) deletion(observation) stayers(movers) ///
        algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
        probes(200) tolerance(1e-12) project(z) projecteffect(firm) nodisplay
    assert e(rust_execution_receipt)[1,"projection"]==2
    assert e(rust_execution_receipt)[1,"logical"]==604
    if "`solver'"=="diagonal" {
        matrix projected = e(projection_b)
        matrix projected_V = e(projection_V)
    }
    else {
        assert "`e(rust_execution_mode)'"=="direct_attachments"
        assert "`e(full_cmg_model_schema)'"==""
        assert e(full_cmg_receipt)[1,"probe_tolerance"]==1e-12
        assert mreldif(e(projection_b),projected)<1e-8
        assert mreldif(e(projection_V),projected_V)<1e-8
    }
}

clear
set obs 800
generate long worker = floor((_n-1)/40)
generate long firm = floor(mod(_n-1,40)/2)
generate long match = floor((_n-1)/2)+1
generate double x = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double y = worker-.8*firm+1.4*x+(mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
// Retain the existing individual-inference regression's heterogeneous target
// masses. The uniform-target alteration is separately replayed and rejected
// by legacy and candidate solvers under the unchanged complete-residual gate.
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
foreach deletion in observation match {
    local options deletion(`deletion')
    if "`deletion'"=="match" local options `options' deletionid(match) nuisance(fixedoffset)
    foreach solver in diagonal cmg {
        display "INFERENCE_ATTEMPT deletion=`deletion' solver=`solver' batch=auto heterogeneous_target=1"
        noisily fevc y x, worker(worker) firm(firm) stayers(movers) `options' ///
            algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
            probes(200) targetweight(target) inference(highrank) inferencemodel(structured_common) ///
            inferencesimulations(129) inferencegramprobes(513) nodisplay
        matrix work = e(rust_execution_receipt)
        matrix inference_batch = e(rust_component_batch_receipt)
        assert "`e(rust_component_batch_schema)'"=="VCKSS-COMPONENT-BATCH-V1"
        assert inference_batch[1,"policy"]==1
        assert inference_batch[1,"declared_component"]==1
        assert inference_batch[1,"declared_gram"]==1
        assert inference_batch[1,"component"]==32 & inference_batch[1,"gram"]==32
        assert inference_batch[1,"cap"]==32 & inference_batch[1,"threads"]==4
        assert inference_batch[1,"capacity"]==work[1,"capacity"]
        assert work[1,"gram"]==513
        assert work[1,"component"]+work[1,"gram"]==e(component_inference_receipt)[1,"solver_columns"]
        assert work[1,"logical"]==work[1,"fit"]+1+600+work[1,"component"]+513
        assert work[1,"residual"]<=e(residual_acceptance_tolerance)
        if "`solver'"=="diagonal" {
            matrix point = e(kss)
            matrix component = e(component_inference)
        }
        else {
            assert "`e(rust_execution_mode)'"=="direct_attachments"
            assert work[1,"logical"]+work[1,"refinement"]==work[1,"cmg"]
            assert e(full_cmg_receipt)[1,"probe_tolerance"]==1e-10
            assert mreldif(point,e(kss))<1e-8
            assert mreldif(component,e(component_inference))<1e-8
        }
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
}
display as result "PASS test_rust_execution_paths.do"
