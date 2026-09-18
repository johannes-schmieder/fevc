version 18.0
clear all
set more off
set varabbrev off
set linesize 255

local package : environment OP_PACKAGE
local input : environment OP_INPUT
local output : environment OP_OUTPUT
local profile : environment OP_PROFILE
local variant : environment OP_VARIANT
local threads_arg : environment OP_THREADS
local seed_arg : environment OP_SEED
local warmup_arg : environment OP_WARMUP
local repetition_arg : environment OP_REPETITION
local input_sha : environment OP_INPUT_SHA256
local source_sha : environment OP_SOURCE_SHA256
local threads = real("`threads_arg'")
local seed = real("`seed_arg'")
local warmup = real("`warmup_arg'")
local repetition = real("`repetition_arg'")
if !inlist("`variant'","baseline","candidate") | !inlist(`threads',1,4,7) | ///
    !inlist(`warmup',0,1) | missing(`seed') | `seed'<=0 |               ///
    !inlist("`profile'","mover_match_both","pooled_stayer_match",    ///
        "observation_stayers","controlled_weighted_match",           ///
        "controlled_weighted_observation","observation_projection",  ///
        "structured_observation_inference","fixed_offset_match_inference") | ///
    !ustrregexm("`input_sha'","^[0-9a-f]{64}$") |                    ///
    !ustrregexm("`source_sha'","^[0-9a-f]{64}$") {
    di as error "invalid bounded development call"
    exit 198
}
confirm file `"`package'/fevc.ado"'
confirm file `"`package'/fevc_rust_linux_x64.plugin"'
confirm file `"`input'"'
capture confirm file `"`output'"'
if !_rc exit 602
adopath ++ `"`package'"'
capture set processors `threads'
if _rc | c(processors)!=`threads' exit 198
import delimited using `"`input'"', clear varnames(1) asdouble bindquote(strict)
confirm numeric variable observation_key worker firm period match y frequency ///
    target_weight control_1 control_2 projection
isid observation_key
quietly count
local rows = r(N)
if !inlist(`rows',8000,100000) exit 459
sort observation_key
quietly _datasignature
local signature `"`r(datasignature)'"'
set rng default
set seed 20260913
local rng_before `"`c(rngstate)'"'

local common worker(worker) firm(firm) backend(rust) rng(counter_v1) ///
    batch(auto) probes(200) seed(`seed') maxiter(10000) nodisplay
local command
if "`profile'"=="mover_match_both" {
    local command fevc y, `common' deletion(match) deletionid(match) stayers(both) ///
        algorithm(jla) engine(generic) preconditioner(cmg) exact_limit(2)
}
else if "`profile'"=="pooled_stayer_match" {
    local command fevc y, `common' deletion(match) deletionid(match) stayers(both) ///
        algorithm(jla) engine(auto) preconditioner(cmg)
}
else if "`profile'"=="observation_stayers" {
    local command fevc y, `common' deletion(observation) stayers(both) ///
        algorithm(jla) engine(auto) preconditioner(cmg)
}
else if "`profile'"=="controlled_weighted_match" {
    local command fevc y control_1 control_2 [fw=frequency], `common' ///
        deletion(match) deletionid(match) stayers(both) targetweight(target_weight) ///
        algorithm(jla) engine(generic) preconditioner(cmg)
}
else if "`profile'"=="controlled_weighted_observation" {
    local command fevc y control_1 control_2 [fw=frequency], `common' ///
        deletion(observation) stayers(both) targetweight(target_weight) ///
        algorithm(jla) engine(generic) preconditioner(cmg)
}
else if "`profile'"=="observation_projection" {
    local command fevc y control_1, `common' deletion(observation) stayers(movers) ///
        algorithm(jla) engine(generic) project(projection) projecteffect(firm) ///
        projectweight(target) targetweight(target_weight) preconditioner(diagonal)
}
else if "`profile'"=="structured_observation_inference" {
    local command fevc y control_1, `common' deletion(observation) stayers(movers) ///
        algorithm(jla) engine(generic) targetweight(target_weight) ///
        inference(highrank) inferencemodel(structured_common) ///
        inferencesimulations(129) inferencegramprobes(513) preconditioner(diagonal)
}
else if "`profile'"=="fixed_offset_match_inference" {
    local command fevc y control_1 [fw=frequency], `common' deletion(match) ///
        deletionid(match) nuisance(fixedoffset) stayers(movers) ///
        algorithm(jla) engine(generic) targetweight(target_weight) ///
        inference(highrank) inferencemodel(structured_common) ///
        inferencesimulations(129) inferencegramprobes(513) preconditioner(diagonal)
}

timer clear 80
timer on 80
capture noisily `command'
local command_rc = _rc
timer off 80
if `command_rc' exit `command_rc'
timer list 80
local command_seconds = r(t80)
assert `"`e(backend_selected)'"'=="rust"
assert `"`e(algorithm)'"'=="jla"
assert e(N_stored)==`rows'
assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
assert e(target_identity_residual)<=1e-10
matrix results = e(results)
assert rowsof(results)==4 & colsof(results)==4
local engine `"`e(engine_selected)'"'
local route `"`e(preconditioner_selected)'"'
local execution_mode legacy
if `"`e(rust_execution_mode)'"'!="" local execution_mode `"`e(rust_execution_mode)'"'
local native_total = .
capture matrix native_profile = e(rust_phase_profile)
if !_rc local native_total = native_profile[1,8]
local execution_threads = .
capture local execution_threads = e(rust_execution_threads)
local execution_queued = .
capture local execution_queued = e(rust_execution_queued_rhs)
local execution_cmg = .
capture local execution_cmg = e(rust_execution_cmg_rhs)
local lev_batch = e(leverage_batch)
local tgt_batch = e(target_batch)
local residual = e(complete_residual_max)
local acceptance = e(residual_acceptance_tolerance)
local identity = e(target_identity_residual)
local iterations = e(solver_iterations)
local peak = e(resource_peak_bytes)

quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
assert `"`c(rngstate)'"'==`"`rng_before'"'
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'

tempname handle
file open `handle' using `"`output'"', write text
file write `handle' "schema" _tab "profile" _tab "variant" _tab "threads" _tab ///
    "seed" _tab "warmup" _tab "repetition" _tab "rows" _tab "command_seconds" _tab ///
    "engine" _tab "route" _tab "execution_mode" _tab "execution_threads" _tab ///
    "queued_rhs" _tab "cmg_rhs" _tab "leverage_batch" _tab "target_batch" _tab ///
    "max_residual" _tab "acceptance" _tab "target_identity" _tab "iterations" _tab ///
    "peak_bytes" _tab "native_total" _tab "input_sha256" _tab "source_sha256"
forvalues row=1/4 {
    forvalues column=1/4 {
        file write `handle' _tab "result_`row'_`column'"
    }
}
file write `handle' _n "FEVC-OPTIMIZATION-DEVELOPMENT-CALL-V1" _tab ///
    "`profile'" _tab "`variant'" _tab (`threads') _tab (`seed') _tab (`warmup') _tab ///
    (`repetition') _tab (`rows') _tab (`command_seconds') _tab "`engine'" _tab ///
    "`route'" _tab "`execution_mode'" _tab (`execution_threads') _tab ///
    (`execution_queued') _tab (`execution_cmg') _tab (`lev_batch') _tab (`tgt_batch') _tab ///
    (`residual') _tab (`acceptance') _tab (`identity') _tab (`iterations') _tab ///
    (`peak') _tab (`native_total') _tab "`input_sha'" _tab "`source_sha'"
forvalues row=1/4 {
    forvalues column=1/4 {
        file write `handle' _tab (results[`row',`column'])
    }
}
file write `handle' _n
file close `handle'
di as result "FEVC OPTIMIZATION DEVELOPMENT CALL PASS: `profile' `variant' threads=`threads' warmup=`warmup' repetition=`repetition'"
