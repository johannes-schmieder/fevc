version 18.0
clear all
set more off
set varabbrev off
// Repeat-measurement standard errors on pooled movers and stayers
// (stayers(both), the default) under one backend, solver and thread count.
// The second argument keeps the name of a variance model removed in fevc
// 0.6.0, because installers and qualifier stages pass it; it now selects the
// deletion mode: structured_common -> deletion(match) with
// nuisance(fixedoffset), structured_leverage -> deletion(observation) with
// nuisance(joint). Native observation deletion needs unit frequencies, so
// that variant drops the frequency weights on the rust backend.
args pkgroot backend model solver test_threads
if "`solver'"=="" local solver diagonal
assert inlist("`solver'","diagonal","cmg")
if "`test_threads'"!="" set processors `test_threads'
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
if "`backend'"=="" local backend mata
if "`model'"=="" local model structured_leverage
assert inlist("`model'","structured_common","structured_leverage")
adopath ++ `"`pkgroot'"'
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
generate double control=sin(_n/7)+mod(_n,5)/3
generate double shifted=y+100
local deletion match
local nuisance fixedoffset
if "`model'"=="structured_leverage" {
    local deletion observation
    local nuisance joint
}
local weight [fw=fw]
if "`backend'"=="rust" & "`deletion'"=="observation" local weight
local route backend(mata) algorithm(exact)
// An explicit batch: with engine(generic), preconditioner(diagonal|cmg) and an
// automatic batch the plugin rejects the component batch receipt of this model.
if "`backend'"=="rust" local route backend(rust) rng(counter_v1) algorithm(jla) engine(generic) preconditioner(`solver') batch(8) probes(200) mcse(off)
local common worker(worker) firm(firm) deletion(`deletion') nuisance(`nuisance') targetweight(tw) `route' nodisplay
local inference inferenceseed(1791)
local rng0 `"`c(rngstate)'"'
fevc y control `weight', `common' inference(none)
matrix point=e(b)
fevc y control `weight', `common' `inference' inference(highrank)
if "`backend'"=="rust" {
    assert "`e(inference_backend_selected)'"=="rust"
    assert "`e(preconditioner_selected)'"==upper("`solver'")
    assert "`e(inference_solver_selected)'"==upper("`solver'")
    assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
}
assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_SE"
assert "`e(inference_model)'"=="repeat_measurement"
assert "`e(inference_variance_fit)'"=="repeat_measurement"
assert "`e(stayers)'"=="both"
assert mreldif(point,e(b))<1e-9
assert `"`rng0'"'==`"`c(rngstate)'"'
// Match deletion: one unit per mover match and per physical stayer
// observation, frequency copies included; observation deletion: one unit
// per physical observation.
if "`deletion'"=="match" {
    assert e(inference_mover_units)==128
    quietly summarize fw in 257/512, meanonly
    assert e(inference_stayer_units)==r(sum)
    assert e(inference_independent_units)==128+r(sum)
    // The stayers are the repeat-measurement sets.
    assert e(inference_repeat_sets)==32
}
else {
    quietly summarize fw, meanonly
    local physical = cond("`weight'"=="",_N,r(sum))
    assert e(inference_independent_units)==`physical'
}
// The control coefficient enters to first order.
assert e(inference_nuisance_omitted)==0
assert e(inference_mean_omitted)==0
confirm matrix e(V)
matrix basepoint=e(b)
matrix baseinterval=e(component_inference)
matrix baseV=e(V)
fevc shifted control `weight', `common' `inference' inference(highrank)
assert mreldif(basepoint,e(b))<1e-9
assert mreldif(baseinterval,e(component_inference))<1e-8
fevc y control `weight', `common' `inference' inference(q1)
assert "`e(inference_leading_request)'"=="q1"
matrix table=e(inference_intervals)
matrix lead=e(inference_leading)
assert e(inference_q1_components)==(table[1,7]==1)+(table[2,7]==1)+(table[3,7]==1)+(table[4,7]==1)
assert e(inference_q1_components)>0
capture confirm matrix e(V)
assert (_rc!=0)==(e(inference_q1_components)>0)
forvalues row=1/4 {
    // inference(q1) reports the q = 1 interval wherever it exists; the
    // estimate splits exactly into its leading part and the remainder.
    if table[`row',16]==0 {
        assert table[`row',7]==1
        assert lead[`row',16]<1e-6
    }
    assert table[`row',8]<table[`row',1] & table[`row',1]<table[`row',9]
    assert reldif(table[`row',2],baseinterval[`row',2])<1e-12
}
assert `"`rng0'"'==`"`c(rngstate)'"'
// Admission failures preserve data and RNG, and release any native generation.
quietly datasignature
local data0 `"`r(datasignature)'"'
capture noisily fevc y control `weight', `common' `inference' inference(highrank) memory_gib(.000001) memorycheck(error)
assert _rc!=0
assert `"`rng0'"'==`"`c(rngstate)'"'
quietly datasignature
assert `"`data0'"'==`"`r(datasignature)'"'
// The removed variance models and their tuning options stop before any work.
capture noisily fevc y control `weight', `common' `inference' inference(highrank) inferencemodel(`model')
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_REMOVED"
capture noisily fevc y control `weight', `common' `inference' inference(highrank) inferencesimulations(100)
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_OPTION_REMOVED"
assert `"`rng0'"'==`"`c(rngstate)'"'
if "`backend'"=="rust" {
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
    // Design order, thread count, batch width and solver choice preserve the
    // Counter-V1 draws.
    preserve
    generate long original_order=_n
    gsort -original_order
    local alternate : subinstr local common "preconditioner(diagonal)" "preconditioner(cmg)"
    local alternate : subinstr local alternate "batch(8)" "batch(16)"
    fevc y control `weight', `alternate' `inference' inference(highrank) nativethreads(2)
    assert "`e(inference_solver_selected)'"=="CMG"
    assert mreldif(basepoint,e(b))<1e-8
    assert mreldif(baseinterval,e(component_inference))<1e-8
    restore
    // Old transports must fail before preparation or RNG even on valid data.
    capture program drop fevc__rust_public_call
    program define fevc__rust_public_call, rclass
        gettoken command rest : 0
        fevc_rust `command' `rest'
        return add
        if "`command'"=="probe" return scalar component_mixed_api=0
    end
    capture noisily fevc y control `weight', `common' `inference' inference(highrank)
    assert _rc==498
    assert "`e(withholding_status)'"=="REPEAT_MEASUREMENT_TUPLE_REQUIRED"
    assert strpos(`"`e(withholding_detail)'"',"predates them")>0
    assert `"`rng0'"'==`"`c(rngstate)'"'
    program drop fevc__rust_public_call
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
if "`backend'"=="mata" {
    // Literal expansion of the frequency weights reproduces the point
    // estimate and the standard errors: copies are physical observations.
    replace tw=tw/fw
    expand fw
    replace fw=1
    fevc y control [fw=fw], `common' `inference' inference(highrank)
    assert mreldif(basepoint,e(b))<1e-9
    assert mreldif(baseinterval,e(component_inference))<1e-8
    assert mreldif(baseV,e(V))<1e-8
}
display as result "FEVC POOLED COMPONENT INFERENCE PASS: `backend'"
