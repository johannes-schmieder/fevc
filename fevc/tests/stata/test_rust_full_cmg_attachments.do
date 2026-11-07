version 18.0
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
        assert e(rust_execution_receipt)[1,"projection"]==2
        assert e(rust_execution_receipt)[1,"logical"]==604
    }
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
            probes(200) targetweight(target) centering(none) inference(highrank) inferencemodel(structured_common) ///
            inferencesimulations(129) inferencegramprobes(513) nodisplay
        if "`solver'"=="cmg" {
        matrix work = e(rust_execution_receipt)
        matrix inference_batch = e(rust_component_batch_receipt)
        assert "`e(rust_component_batch_schema)'"=="VCKSS-COMPONENT-BATCH-V1"
        assert inference_batch[1,"policy"]==1
        assert inference_batch[1,"declared_component"]==1
        assert inference_batch[1,"declared_gram"]==1
        assert inference_batch[1,"component"]==32 & inference_batch[1,"gram"]==32
        assert inference_batch[1,"cap"]==32 & inference_batch[1,"threads"]==`test_threads'
        assert inference_batch[1,"capacity"]==work[1,"capacity"]
        assert work[1,"gram"]==513
        assert work[1,"component"]+work[1,"gram"]==e(component_inference_receipt)[1,"solver_columns"]
        assert work[1,"logical"]==work[1,"fit"]+1+600+work[1,"component"]+513
        assert work[1,"residual"]<=e(residual_acceptance_tolerance)
        }
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
display as result "PASS test_rust_full_cmg_attachments.do"
