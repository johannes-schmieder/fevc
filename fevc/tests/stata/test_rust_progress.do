version 18.0
clear all
set more off
set varabbrev off
set linesize 255
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
scalar __vckss_rust_progress_api = 999
quietly fevc_rust probe
assert r(progress_api)==1
capture confirm scalar __vckss_rust_progress_api
assert _rc!=0

program define _progress_read_log, rclass
    args filename
    tempname stream
    file open `stream' using `"`filename'"', read text
    local messages = 0
    local selection = 0
    local allocations = 0
    local exclusions = 0
    local warnings = 0
    file read `stream' line
    while r(eof)==0 {
        if strpos(`"`line'"',"Warning: forecast direct allocations")==1 local ++warnings
        if substr(`"`line'"',1,2)=="  " {
            if strpos(`"`line'"',"Prepared graph sample:") | ///
                strpos(`"`line'"',"Method:") | strpos(`"`line'"',"probes:") {
                local ++messages
            }
            if strpos(`"`line'"',"Selection:") local ++selection
            if strpos(`"`line'"',"allocations") local ++allocations
            if strpos(`"`line'"',"Stayer exclusions: 1 physical-singleton workers; 1 workers outside retained firms") local ++exclusions
        }
        file read `stream' line
    }
    file close `stream'
    return scalar messages = `messages'
    return scalar selection = `selection'
    return scalar allocations = `allocations'
    return scalar exclusions = `exclusions'
    return scalar warnings = `warnings'
end

set processors 4
set obs 288
generate long key = _n
generate long worker = ceil(key/8)
generate byte slot = mod(key-1,8)
generate int firm = floor(slot/2)+1
quietly replace firm = mod(worker-1,4)+1 if key>256
generate double x = (worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x
generate byte copies = 1+mod(key,3)
generate double mass = .7+mod(13*key,11)/7
sort worker firm key
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
tempfile transcript

foreach deletion in observation match {
    forvalues route = 1/4 {
        local algorithm jla
        local solver
        local controls
        if `route'==1 local algorithm exact
        if `route'==3 {
            local solver engine(generic) preconditioner(diagonal)
            local controls x
        }
        if `route'==4 {
            local solver engine(generic) preconditioner(cmg)
            local controls x
        }
        local common worker(worker) firm(firm) deletion(`deletion') ///
            backend(rust) algorithm(`algorithm') `solver' probes(33) seed(81227) targetweight(mass)
        quietly fevc y `controls' [fw=copies], `common' nolog
        matrix reference = e(kss)
        generate byte retained = e(sample)
        local selected `"`e(algorithm)'|`e(engine_selected)'|`e(preconditioner_selected)'"'
        foreach output in default verbose nolog nologverbose nodisplay quietly {
            local options `output'
            local prefix noisily
            if "`output'"=="default" local options
            if "`output'"=="nologverbose" local options nolog verbose
            if "`output'"=="quietly" {
                local prefix quietly
                local options verbose
            }
            quietly log using `"`transcript'"', text replace name(progress_test)
            `prefix' fevc y `controls' [fw=copies], `common' `options'
            quietly log close progress_test
            assert mreldif(e(kss),reference)<1e-12
            assert e(sample)==retained
            assert `"`e(algorithm)'|`e(engine_selected)'|`e(preconditioner_selected)'"'==`"`selected'"'
            assert `"`c(rngstate)'"'==`"`state'"'
            assert `"`c(sortrngstate)'"'==`"`sortstate'"'
            assert "$VCKSS_REPORT_LEVEL$VCKSS_REPORT_API$VCKSS_REPORT_NOTICE$VCKSS_REPORT_ALLOWED"==""
            quietly _progress_read_log `"`transcript'"'
            if inlist("`output'","default","verbose") {
                assert r(messages)>0 & r(allocations)>0
                if "`output'"=="verbose" assert r(selection)>0
                else assert r(selection)==0
            }
            else assert r(messages)==0 & r(selection)==0 & r(allocations)==0
        }
        drop retained
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
}

// Stayer exclusions precede augmentation and are distinct from graph pruning.
preserve
quietly set obs 291
quietly replace worker = 37 if _n==289
quietly replace firm = 1 if _n==289
quietly replace worker = 38 if _n>=290
quietly replace firm = 99 if _n>=290
quietly replace y = sin(_n) if _n>=289
quietly log using `"`transcript'"', text replace name(progress_test)
fevc y, worker(worker) firm(firm) deletion(match) stayers(both) backend(rust) algorithm(exact)
quietly log close progress_test
assert e(stayer_hybrid_N_singleton_drop)==1
assert e(stayer_hybrid_N_unattached)==1
quietly _progress_read_log `"`transcript'"'
assert r(exclusions)==1
restore

// Native reporting preserves inference and its strict work gates; use the
// established heterogeneous-target fixture.
preserve
clear
set obs 800
generate long worker = floor((_n-1)/40)
generate long firm = floor(mod(_n-1,40)/2)
generate double x = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double shock = (mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
generate double y = worker-.8*firm+1.4*x+shock
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
foreach deletion in observation match {
    local nuisance
    if "`deletion'"=="match" local nuisance nuisance(fixedoffset)
    local common worker(worker) firm(firm) deletion(`deletion') `nuisance' rng(counter_v1) ///
        stayers(movers) backend(rust) algorithm(jla) engine(generic) preconditioner(diagonal) ///
        probes(200) targetweight(target) inference(highrank) inferencemodel(structured_common) ///
        inferencesimulations(129) inferencegramprobes(513)
    quietly fevc y x, `common' nolog
    matrix reference = e(kss)
    matrix inference = e(component_inference)
    local posted = e(inference_joint_posted)
    if `posted' matrix covariance = e(V)
    fevc y x, `common'
    assert mreldif(e(kss),reference)<1e-12
    assert mreldif(e(component_inference),inference)<1e-12
    assert e(inference_joint_posted)==`posted'
    if `posted' assert mreldif(e(V),covariance)<1e-12
}
restore

// Both log formats and Mata tolerate the new command options.
quietly log using `"`transcript'"', smcl replace name(progress_test)
fevc y, worker(worker) firm(firm) backend(rust) algorithm(exact) verbose
quietly log close progress_test
quietly fevc y, worker(worker) firm(firm) backend(mata) algorithm(exact) nolog verbose
quietly log using `"`transcript'"', text replace name(progress_test)
fevc y, worker(worker) firm(firm) backend(rust) algorithm(exact) ///
    memory_gib(.00001) memorycheck(warn) nolog
quietly log close progress_test
quietly _progress_read_log `"`transcript'"'
assert r(warnings)>0 & r(messages)==0 & r(allocations)==0
capture noisily fevc y, worker(worker) firm(firm) backend(rust) algorithm(exact) ///
    memory_gib(.00001) memorycheck(error) nolog
assert _rc!=0
assert "`e(status)'"=="WITHHELD"
capture noisily fevc y, worker(worker) firm(firm) backend(rust) probes(1)
assert _rc!=0
assert "$VCKSS_REPORT_LEVEL$VCKSS_REPORT_API$VCKSS_REPORT_NOTICE$VCKSS_REPORT_ALLOWED"==""
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
display "PASS test_rust_progress.do"
