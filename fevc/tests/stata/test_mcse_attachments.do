version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly fevc_rust probe
assert r(numerical_api)==2
local solvers diagonal
local execution = ("`c(os)'"!="Windows")
if `execution' {
    set processors 4
    local solvers diagonal cmg
}
set obs 240
gen long worker=floor((_n-1)/6)
gen byte time=mod(_n-1,6)
gen long firm=mod(worker+floor(time/2),20)
gen double x=time-2.5
gen double z=sin(worker/5)+cos(firm/3)+time/20
gen double y=-4+.08*worker-.12*firm+.3*x+.25*sin((_n*17)/11)+.15*cos((_n*7)/13)
foreach backend in rust {
    foreach solver of local solvers {
        local common worker(worker) firm(firm) deletion(observation) stayers(movers) ///
            algorithm(jla) backend(`backend') rng(counter_v1) engine(generic) preconditioner(`solver') ///
            probes(200) tolerance(1e-12) centering(none) project(z) projecteffect(firm) nodisplay
        quietly fevc y x, `common' mcse(off)
        matrix point=e(kss)
        matrix projected=e(projection_b)
        matrix projected_V=e(projection_V)
        local selected `e(preconditioner_selected)'
        quietly fevc y x, `common'
        assert "`e(mcse_mode)'"=="all" & "`e(numerical_mc_method)'"=="crossfit_if_v1"
        assert mreldif(e(kss),point)<1e-11
        assert mreldif(e(projection_b),projected)<1e-11
        assert mreldif(e(projection_V),projected_V)<1e-11
        assert "`e(preconditioner_selected)'"=="`selected'"
        assert e(mc_replay_rhs)==200
        if `execution' {
            assert e(rust_execution_receipt)[1,"projection"]==2
            assert e(rust_execution_receipt)[1,"logical"]==604
        }
        assert `"`c(rngstate)'"'==`"`state'"'
        assert `"`c(sortrngstate)'"'==`"`sortstate'"'
    }
}
clear
set obs 800
gen long worker=floor((_n-1)/40)
gen long firm=floor(mod(_n-1,40)/2)
gen long match=floor((_n-1)/2)+1
gen double x=.31*mod(_n-1,2)+mod(_n-1,7)/29
gen double y=worker-.8*firm+1.4*x+(mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
gen double target=(.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
foreach deletion in observation match {
    local options deletion(`deletion')
    if "`deletion'"=="match" local options `options' deletionid(match) nuisance(fixedoffset)
    foreach solver of local solvers {
        foreach reference in highrank q1 {
            // Repeat-measurement standard errors with an automatic batch: the
            // diagonal queue and the direct CMG attachment executor report
            // their work and the component batch receipt.
            local common worker(worker) firm(firm) stayers(movers) `options' ///
                algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(`solver') ///
                probes(200) targetweight(target) centering(none) inference(`reference') nodisplay
            quietly fevc y x, `common' mcse(off)
            assert "`e(inference_model)'"=="repeat_measurement"
            matrix point=e(kss)
            matrix intervals=e(component_inference)
            matrix spectrum=e(component_spectrum)
            matrix units=e(inference_repeat)
            local queue = `execution'
            if `queue' {
                matrix work=e(rust_execution_receipt)
                matrix widths=e(rust_component_batch_receipt)
            }
            local joint=e(inference_joint_available)
            local computed=e(inference_computed_targets)
            if "`reference'"=="q1" {
                matrix table=e(inference_intervals)
                matrix leading=e(inference_leading)
            }
            quietly fevc y x, `common'
            assert "`e(mcse_mode)'"=="all" & "`e(numerical_mc_method)'"=="crossfit_if_v1"
            assert mreldif(e(kss),point)<1e-11
            assert mreldif(e(component_inference),intervals)<1e-11
            assert mreldif(e(component_spectrum),spectrum)<1e-11
            assert mreldif(e(inference_repeat),units)==0
            if `queue' {
                foreach field in mode threads workers cmg_concurrency capacity leverage target ///
                    fit rank point projection component gram logical queued cmg refinement {
                    assert e(rust_execution_receipt)[1,"`field'"]==work[1,"`field'"]
                }
                // How many queue workers took a task depends on scheduling;
                // the direct CMG executor reports none.
                if "`solver'"=="diagonal" assert inrange(e(rust_execution_receipt)[1,"active"],1,work[1,"threads"])
                else assert e(rust_execution_receipt)[1,"active"]==0 & work[1,"active"]==0
                assert work[1,"gram"]==0
                assert work[1,"component"]==e(component_inference_receipt)[1,"solver_columns"]
                assert e(rust_execution_receipt)[1,"residual"]<=e(residual_acceptance_tolerance)
                assert e(rust_execution_receipt)[1,"peak"]>=work[1,"peak"]
                foreach field in policy reason declared_component declared_gram component gram cap threads capacity {
                    assert e(rust_component_batch_receipt)[1,"`field'"]==widths[1,"`field'"]
                }
                assert widths[1,"declared_gram"]==0 & widths[1,"gram"]==0
            }
            assert e(inference_joint_available)==`joint' & e(inference_computed_targets)==`computed'
            if "`reference'"=="q1" {
                assert mreldif(e(inference_intervals),table)<1e-11
                assert mreldif(e(inference_leading),leading)<1e-11
            }
            assert e(mc_replay_rhs)==200
            assert `"`c(rngstate)'"'==`"`state'"'
            assert `"`c(sortrngstate)'"'==`"`sortstate'"'
        }
    }
}
// The structured model and its tuning options were removed in fevc 0.6.0.
capture noisily fevc y x, worker(worker) firm(firm) stayers(movers) deletion(observation) ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    probes(200) inference(highrank) inferencemodel(structured_common) nodisplay
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_REMOVED"
foreach option in inferencesimulations(129) inferencegramprobes(513) {
    capture noisily fevc y x, worker(worker) firm(firm) stayers(movers) deletion(observation) ///
        algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
        probes(200) inference(highrank) `option' nodisplay
    assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_OPTION_REMOVED"
}
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'
di as result "PASS test_mcse_attachments.do"
