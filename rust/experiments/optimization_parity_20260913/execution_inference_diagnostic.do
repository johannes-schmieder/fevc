version 18.0
clear all
set more off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 4
set obs 800
generate long worker = floor((_n-1)/40)
generate long firm = floor(mod(_n-1,40)/2)
generate long match = floor((_n-1)/2)+1
generate double x = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double y = worker-.8*firm+1.4*x+(mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
foreach deletion in observation match {
    local options deletion(`deletion')
    if "`deletion'"=="match" local options `options' deletionid(match) nuisance(fixedoffset)
    foreach solver in diagonal cmg {
        foreach width in 8 auto {
            display "INFERENCE_ATTEMPT deletion=`deletion' solver=`solver' batch=`width'"
            capture noisily fevc y x, worker(worker) firm(firm) stayers(movers) `options' ///
                algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
                batch(`width') probes(200) inference(highrank) inferencemodel(structured_common) ///
                inferencesimulations(129) inferencegramprobes(513) nodisplay
            local rc = _rc
            display "INFERENCE_RESULT rc=`rc' mode=`e(rust_execution_mode)' status=`e(withholding_status)'"
            quietly fevc_rust snapshot
            assert r(state)==0 & r(handle)==0
        }
    }
}
display "INFERENCE_DIAGNOSTIC_COMPLETE"
