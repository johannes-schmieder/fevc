version 18.0
clear all
set more off
set varabbrev off
args pkgroot block arm
adopath ++ `"`pkgroot'"'
quietly run `"`pkgroot'/fevc.ado"'
if "`block'"=="" {
    tempname input
    file open `input' using "windows-performance.block", read text
    file read `input' block
    file close `input'
}
if "`arm'"=="" {
    if "`c(os)'"=="Windows" {
        tempname input
        file open `input' using "windows-performance.arm", read text
        file read `input' arm
        file close `input'
    }
    else local arm candidate
}
assert inlist("`arm'","candidate","incumbent")
assert inrange(real("`block'"),0,5)
local seeds "8675309 8675311 8675317 8675323 8675329 8675333"
local seed : word `=real("`block'")+1' of `seeds'
tempname output result mcse
file open `output' using "windows-performance.csv", write text
forvalues cell=0/5 {
    foreach threads in 1 2 {
        clear
        set processors `threads'
        local controls
        local extra
        local deletion match
        local probes 32
        local tolerance 1e-10
        if `cell'<=2 {
            local workers = cond(`cell'==0,512,cond(`cell'==1,4000,16384))
            local firms = cond(`cell'==0,256,cond(`cell'==1,1000,2048))
            local periods = cond(`cell'==0,4,8)
            set obs `=`workers'*`periods''
            generate long key=_n
            generate long worker=ceil(_n/`periods')
            generate byte slot=mod(_n-1,`periods')
            generate long firm=mod(worker-1+cond(`cell'==0,1,3)*floor(slot/2),`firms')+1
            generate double y=.31*worker-.23*firm+.07*slot+mod(17*key+3,29)/101
            local extra stayers(movers) probeorder(key)
            if `cell'==2 local deletion observation
        }
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
        // Windows 0.5.1 does not provide the full-CMG generic route. Compare
        // the same estimator request through its supported diagonal solver.
        local route engine(generic) preconditioner(cmg)
        if "`arm'"=="incumbent" local route engine(generic) preconditioner(diagonal)
        if `cell'==0 local route engine(auto) preconditioner(auto)
        local options worker(worker) firm(firm) deletion(`deletion') backend(rust) rng(counter_v1) algorithm(jla) `route' batch(auto) probes(`probes') seed(`seed') tolerance(`tolerance') `extra' nodisplay
        quietly fevc y `controls', `options'
        timer clear 97
        timer on 97
        quietly fevc y `controls', `options'
        timer off 97
        if "`arm'"=="candidate" {
            assert "`e(cmg_backend)'"=="CMG_FULL_V2"
            assert "`e(cmg_source_commit)'"=="e5613e81fb3b03c78de98658b372646fa802aabb"
            assert e(cmg_threads_requested)==`threads' & e(cmg_threads_used)==`threads'
        }
        else {
            assert "`e(version)'"=="0.5.1"
            if "`c(os)'"=="Windows" assert "`e(cmg_backend)'"==""
        }
        matrix `result'=e(kss)
        matrix `mcse'=e(numerical_mcse)
        assert rowsof(`result')==1 & colsof(`result')==4
        assert rowsof(`mcse')==1 & colsof(`mcse')==4
        assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
        local residual=e(complete_residual_max)
        local limit=e(residual_acceptance_tolerance)
        local retained=e(N_retained)
        quietly timer list 97
        local elapsed=1000*r(t97)
        file write `output' (`cell') "," (`threads') "," %21.17g (`elapsed')
        forvalues j=1/4 {
            assert !missing(`result'[1,`j'])
            file write `output' "," %21.17g (`result'[1,`j'])
        }
        forvalues j=1/4 {
            assert !missing(`mcse'[1,`j']) & `mcse'[1,`j']>=0
            file write `output' "," %21.17g (`mcse'[1,`j'])
        }
        file write `output' "," %21.17g (`residual') "," %21.17g (`limit') "," (`retained') _n
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
        display as result "WINDOWS_PERFORMANCE_CELL=`cell' THREADS=`threads' PASS"
    }
}
file close `output'
display as result "PASS test_windows_cmg_performance.do"
