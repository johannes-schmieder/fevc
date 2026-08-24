version 18.0
clear all
set more off
set varabbrev off

args output_dir source_commit
if strtrim("`output_dir'") == "" | strlen("`source_commit'") < 7 {
    di as error "output_dir and source_commit are required"
    exit 198
}
capture confirm file "vckss/vckss.ado"
if _rc {
    di as error "run stata_oracle.do from the staged source root"
    exit 601
}
adopath ++ "`c(pwd)'/vckss"

set obs 24
generate long worker = floor((_n-1)/4)
generate byte period = mod(_n-1,4)
generate double c1 = period-1.5
generate double c2 = period==2
generate byte firm = .
generate long match = .
generate double noise = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise

vckss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nuisance(joint) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
tempname result
matrix `result' = e(results)

forvalues target_index = 1/4 {
    scalar stata_plugin_`target_index' = `result'[1,`target_index']
    scalar stata_correction_`target_index' = `result'[2,`target_index']
    scalar stata_corrected_`target_index' = `result'[3,`target_index']
}

preserve
clear
set obs 4
generate str40 source_commit = "`source_commit'"
generate str24 oracle = "stata_vckss_exact"
generate str32 target = ""
local target_names worker_variance firm_variance worker_firm_covariance total_variance
forvalues target_index = 1/4 {
    local target_name : word `target_index' of `target_names'
    quietly replace target = "`target_name'" in `target_index'
}
generate double plugin = .
generate double correction = .
generate double corrected = .
forvalues target_index = 1/4 {
    quietly replace plugin = scalar(stata_plugin_`target_index') in `target_index'
    quietly replace correction = scalar(stata_correction_`target_index') in `target_index'
    quietly replace corrected = scalar(stata_corrected_`target_index') in `target_index'
}
export delimited using "`output_dir'/stata_oracle.csv", replace
restore

import delimited using "`output_dir'/matlab_oracle.csv", clear varnames(1) asdouble
assert _N == 4
assert source_commit == "`source_commit'"
forvalues target_index = 1/4 {
    assert abs(plugin-scalar(stata_plugin_`target_index')) < 2e-10 in `target_index'
    assert abs(correction-scalar(stata_correction_`target_index')) < 2e-9 in `target_index'
    assert abs(corrected-scalar(stata_corrected_`target_index')) < 2e-9 in `target_index'
}

tempname marker
file open `marker' using "`output_dir'/oracle.stata.pass", write text replace
file write `marker' "VCKSS STATA MATLAB ORACLE PASS `source_commit'" _n
file close `marker'
di as result "VCKSS STATA/MATLAB ORACLE PASS"
exit 0
