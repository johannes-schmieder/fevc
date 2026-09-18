version 18.0
clear all
set more off
set varabbrev off
set linesize 255
local package : environment PF_PACKAGE
local input : environment PF_INPUT
local output : environment PF_OUTPUT
local command : environment PF_COMMAND
local threads : environment PF_NATIVE_THREADS
local expected_rows : environment PF_ROWS
local deliberate_failure : environment PF_DELIBERATE_FAILURE
local diagnostic : environment PF_DIAGNOSTIC
local t = real("`threads'")
if missing(`t') | !inrange(`t',1,64) | `t'!=floor(`t') exit 198
set processors `=min(4,`t')'
adopath ++ `"`package'"'
confirm file `"`package'/fevc.ado"'
import delimited using `"`input'"', clear varnames(1) asdouble bindquote(strict)
isid observation_key
sort observation_key
assert _N==real("`expected_rows'")
quietly _datasignature
local signature `"`r(datasignature)'"'
local rng `"`c(rngstate)'"'
local sortrng `"`c(sortrngstate)'"'
local processors = c(processors)
timer clear 80
foreach id in 81 82 83 84 {
    timer clear `id'
}
if "`diagnostic'"=="1" profiler on
timer on 80
if "`deliberate_failure'"=="1" {
    di as error "DELIBERATE_PIPELINE_FAILURE"
    exit 459
}
capture noisily `command'
local estimation_rc = _rc
timer off 80
if "`diagnostic'"=="1" {
    profiler off
    profiler report
}
if `estimation_rc' exit `estimation_rc'
quietly timer list 80
local command_seconds = r(t80)
assert `"`e(backend_selected)'"'=="rust"
assert c(processors)==`processors'
assert `"`c(rngstate)'"'==`"`rng'"'
assert `"`c(sortrngstate)'"'==`"`sortrng'"'
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0

// Full-precision exports include covariance, intervals, availability, phase,
// allocation and execution receipts; nothing is rounded for presentation.
tempname stream
file open `stream' using `"`output'"', write text
file write `stream' "kind" _tab "name" _tab "row" _tab "column" _tab "value" _n
file write `stream' "timer" _tab "command_seconds" _tab "0" _tab "0" _tab %21.17g (`command_seconds') _n
local names estimator_boundary_seconds preparation_boundary_seconds result_fetch_seconds attachment_preparation_seconds
local id = 80
foreach name of local names {
    local ++id
    quietly timer list `id'
    local seconds = r(t`id')
    if missing(`seconds') local seconds = 0
    file write `stream' "timer" _tab "`name'" _tab "0" _tab "0" _tab %21.17g (`seconds') _n
}
local scalars : e(scalars)
foreach name of local scalars {
    file write `stream' "scalar" _tab "`name'" _tab "0" _tab "0" _tab %21.17g (e(`name')) _n
}
local macros : e(macros)
foreach name of local macros {
    local value `"`e(`name')'"'
    file write `stream' "macro" _tab "`name'" _tab "0" _tab "0" _tab `"`value'"' _n
}
local matrices : e(matrices)
foreach name of local matrices {
    tempname m
    matrix `m' = e(`name')
    local rows = rowsof(`m')
    local cols = colsof(`m')
    local rownames : rownames `m'
    local colnames : colnames `m'
    file write `stream' "rownames" _tab "`name'" _tab "0" _tab "0" _tab "`rownames'" _n
    file write `stream' "colnames" _tab "`name'" _tab "0" _tab "0" _tab "`colnames'" _n
    forvalues row=1/`rows' {
        forvalues col=1/`cols' {
            file write `stream' "matrix" _tab "`name'" _tab (`row') _tab (`col') _tab %21.17g (`m'[`row',`col']) _n
        }
    }
}
file close `stream'
display as result "FEVC_PIPELINE_SCREEN_CALL_PASS"
