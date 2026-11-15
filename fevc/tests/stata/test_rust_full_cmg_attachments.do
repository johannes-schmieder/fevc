version 15.0
clear all
set more off
set varabbrev off
args package_dir test_threads
if "`test_threads'"=="" local test_threads = 4
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors `test_threads'
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
        probes(200) tolerance(1e-12) centering(none) project(z) projecteffect(firm) nodisplay
    if "`solver'"=="cmg" {
        assert el(e(rust_execution_receipt),1,colnumb(e(rust_execution_receipt),"projection"))==2
        assert el(e(rust_execution_receipt),1,colnumb(e(rust_execution_receipt),"logical"))==604
    }
    if "`solver'"=="diagonal" {
        matrix projected = e(projection_b)
        matrix projected_V = e(projection_V)
    }
    else {
        assert "`e(rust_execution_mode)'"=="direct_attachments"
        assert "`e(full_cmg_model_schema)'"==""
        assert el(e(full_cmg_receipt),1,colnumb(e(full_cmg_receipt),"probe_tolerance"))==1e-12
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
// Repeat-measurement standard errors on both solvers with an automatic batch:
// with CMG the direct attachment executor carries them. They fit no Gram
// matrix, so the component batch receipt declares and selects no Gram width.
local inference_cap = min(128,max(32,8*`test_threads'))
foreach deletion in observation match {
    local options deletion(`deletion')
    if "`deletion'"=="match" local options `options' deletionid(match) nuisance(fixedoffset)
    foreach solver in diagonal cmg {
        display "INFERENCE_ATTEMPT deletion=`deletion' solver=`solver' batch=auto heterogeneous_target=1"
        noisily fevc y x, worker(worker) firm(firm) stayers(movers) `options' ///
            algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
            probes(200) targetweight(target) centering(none) inference(highrank) nodisplay
        assert "`e(inference_model)'"=="repeat_measurement"
        assert "`e(preconditioner_selected)'"==upper("`solver'")
        assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
        // The diagonal queue reports its work on macOS and Linux only.
        capture confirm matrix e(rust_execution_receipt)
        if !_rc {
            assert el(e(rust_execution_receipt),1,colnumb(e(rust_execution_receipt),"threads"))==`test_threads'
            assert el(e(rust_execution_receipt),1,colnumb(e(rust_execution_receipt),"component"))== ///
                el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"solver_columns"))
        }
        if "`solver'"=="cmg" {
            matrix work = e(rust_execution_receipt)
            matrix inference_batch = e(rust_component_batch_receipt)
            assert "`e(rust_component_batch_schema)'"=="VCKSS-COMPONENT-BATCH-V1"
            assert el(inference_batch,1,colnumb(inference_batch,"policy"))==1
            assert el(inference_batch,1,colnumb(inference_batch,"declared_component"))==1
            assert el(inference_batch,1,colnumb(inference_batch,"declared_gram"))==0 & el(inference_batch,1,colnumb(inference_batch,"gram"))==0
            assert el(inference_batch,1,colnumb(inference_batch,"component"))==`inference_cap'
            assert el(inference_batch,1,colnumb(inference_batch,"cap"))==`inference_cap'
            assert el(inference_batch,1,colnumb(inference_batch,"threads"))==`test_threads'
            assert el(inference_batch,1,colnumb(inference_batch,"capacity"))==el(work,1,colnumb(work,"capacity"))
            assert el(work,1,colnumb(work,"gram"))==0
            assert el(work,1,colnumb(work,"logical"))==el(work,1,colnumb(work,"fit"))+1+600+el(work,1,colnumb(work,"component"))
            assert el(work,1,colnumb(work,"residual"))<=e(residual_acceptance_tolerance)
        }
        if "`solver'"=="diagonal" {
            matrix point = e(kss)
            matrix component = e(component_inference)
        }
        else {
            assert "`e(rust_execution_mode)'"=="direct_attachments"
            assert el(work,1,colnumb(work,"logical"))+el(work,1,colnumb(work,"refinement"))==el(work,1,colnumb(work,"cmg"))
            assert el(e(full_cmg_receipt),1,colnumb(e(full_cmg_receipt),"probe_tolerance"))==1e-10
            assert mreldif(point,e(kss))<1e-8
            assert mreldif(component,e(component_inference))<1e-8
        }
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
}
capture noisily fevc y x, worker(worker) firm(firm) stayers(movers) deletion(observation) ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(cmg) ///
    probes(200) inference(highrank) inferencemodel(structured_common) nodisplay
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_REMOVED"
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
display as result "PASS test_rust_full_cmg_attachments.do"
