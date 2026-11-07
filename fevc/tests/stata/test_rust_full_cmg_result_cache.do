version 18.0
clear all
set more off
set varabbrev off
args pkgroot test_threads
if "`test_threads'"=="" local test_threads = 1
adopath ++ `"`pkgroot'"'
quietly run `"`pkgroot'/fevc.ado"'
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 18.0
    gettoken selector rest : 0
    if "`selector'"=="result" global FEVC_CMG_RESULT_EXPORTS = $FEVC_CMG_RESULT_EXPORTS + 1
    fevc_rust `selector' `rest'
    return add
end
local seed 8675309
forvalues cell=3/5 {
    foreach threads in `test_threads' {
        clear
        set processors `threads'
        local controls
        local extra
        local deletion match
        local probes 32
        local tolerance 1e-10
        if `cell'==3 {
            set obs 288
            generate long key=_n
            generate long worker=ceil(key/8)
            generate byte slot=mod(key-1,8)
            generate int firm=floor(slot/2)+1
            replace firm=mod(worker-1,4)+1 if key>256
            generate double x=(worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
            generate double z=mod(13*key,23)/11+sin(key)
            generate double y=.3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x-.1*z
            generate byte copies=1+mod(key,3)
            generate double mass=.7+mod(13*key,11)/7
            local controls x z [fw=copies]
            local deletion observation
            local extra targetweight(mass) nuisance(joint) stayers(movers)
        }
        if `cell'==4 {
            set obs 240
            generate long worker=floor((_n-1)/6)
            generate byte period=mod(_n-1,6)
            generate long firm=mod(worker+floor(period/2),20)
            generate double x=period-2.5
            generate double z=sin(worker/5)+cos(firm/3)+period/20
            generate double y=-4+.08*worker-.12*firm+.3*x+.25*sin((_n*17)/11)+.15*cos((_n*7)/13)
            local controls x
            local deletion observation
            local extra stayers(movers) project(z) projecteffect(firm) centering(none)
            local probes 200
            local tolerance 1e-12
        }
        if `cell'==5 {
            set seed 98132
            set obs 256
            generate long worker=ceil(_n/16)
            generate byte firm=floor(mod(_n-1,16)/2)+1
            generate double y=worker*.2-firm*.3+rnormal()
            set obs 512
            replace worker=17+floor((_n-257)/8) in 257/512
            replace firm=mod(worker,8)+1 in 257/512
            replace y=worker*.2-firm*.3+rnormal() in 257/512
            generate long fw=1+mod(_n,3)
            generate double tw=fw*(1+mod(_n,7)/10)
            generate double x=sin(_n/7)+mod(_n,5)/3
            local controls x [fw=fw]
            local extra stayers(both) nuisance(fixedoffset) targetweight(tw) inference(highrank) inferencemodel(structured_common) inferencesimulations(100) inferenceseed(1791) centering(none)
            local probes 200
        }

        global FEVC_CMG_RESULT_EXPORTS 0
        quietly fevc y `controls', worker(worker) firm(firm) deletion(`deletion') ///
            backend(rust) rng(counter_v1) algorithm(jla) engine(generic) ///
            preconditioner(cmg) batch(auto) probes(`probes') seed(`seed') ///
            tolerance(`tolerance') `extra' nodisplay
        assert "`e(cmg_backend)'"=="CMG_FULL_V2"
        assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
        display "CMG_RESULT_EXPORTS case=`cell' count=$FEVC_CMG_RESULT_EXPORTS"
        assert $FEVC_CMG_RESULT_EXPORTS == 1
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
}
macro drop FEVC_CMG_RESULT_EXPORTS
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 18.0
    fevc_rust `0'
    return add
end
display "PASS test_rust_full_cmg_result_cache.do"
exit 0
