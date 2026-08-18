version 18.0
clear all
set more off
set varabbrev off

args package_root label prepared_dta prepared_sha matlab_detail ///
    matlab_detail_sha output_dir source_commit matlab_source_commit ///
    wage_input_sha

if !ustrregexm("`label'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`prepared_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`matlab_detail_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`wage_input_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !ustrregexm("`matlab_source_commit'", "^[0-9a-f]{40}$") {
    di as error "invalid MATLAB-retained sample arguments"
    exit 198
}
confirm file `"`package_root'/varcomp_kss.mata"'
confirm file `"`package_root'/benchmarks/separations_sample.mata"'
confirm file `"`prepared_dta'"'
confirm file `"`matlab_detail'"'

mata: mata clear
quietly do `"`package_root'/varcomp_kss.mata"'
quietly do `"`package_root'/benchmarks/separations_sample.mata"'

timer clear 82
timer on 82
import delimited using `"`matlab_detail'"', clear delimiters(tab) ///
    varnames(nonames) numericcols(_all)
ds
local detail_variables `r(varlist)'
local detail_count : word count `detail_variables'
if `detail_count' != 4 {
    di as error "MATLAB detail artifact must contain exactly four columns"
    exit 498
}
confirm numeric variable v1 v2 v3 v4
assert !missing(v2,v3)
assert v2 == floor(v2) & v3 == floor(v3)
rename v2 worker
rename v3 firm
keep worker firm
isid worker firm
quietly count
local matlab_matches = r(N)
tempfile matlab_keys
save `matlab_keys'

quietly use `"`prepared_dta'"', clear
confirm numeric variable worker firm period y_minus_xb observation_key
isid observation_key
quietly count
local input_rows = r(N)
egen byte __input_worker_tag = tag(worker)
egen byte __input_firm_tag = tag(firm)
quietly count if __input_worker_tag
local input_workers = r(N)
quietly count if __input_firm_tag
local input_firms = r(N)
drop __input_worker_tag __input_firm_tag

quietly merge m:1 worker firm using `matlab_keys', generate(__matlab_merge)
quietly count if __matlab_merge == 2
if r(N) != 0 {
    di as error "MATLAB retained match key is absent from the prepared input"
    exit 498
}
quietly keep if __matlab_merge == 3
drop __matlab_merge
isid observation_key
quietly count
local matlab_rows = r(N)
if `matlab_rows' == 0 | `matlab_matches' == 0 {
    di as error "MATLAB retained sample is empty"
    exit 498
}

local graph_removed_rows = 0
local bridge_removed_matches = 0
local bridge_removed_rows = 0
local iterations = 0
local repeat = 1
local iteration_limit = `input_workers' + 2
tempvar graph_worker graph_firm graph_deletion graph_frequency
tempvar graph_sample graph_keep bridge_keep
tempname graph_diagnostics bridge_diagnostics

while `repeat' {
    local ++iterations
    if `iterations' > `iteration_limit' {
        di as error "MATLAB retained sample audit exceeded its finite bound"
        exit 498
    }
    quietly egen long `graph_worker' = group(worker)
    quietly egen long `graph_firm' = group(firm)
    quietly egen long `graph_deletion' = group(worker firm)
    quietly generate byte `graph_frequency' = 1
    quietly generate byte `graph_sample' = 1
    quietly generate byte `graph_keep' = 0
    local graph_status
    local graph_message
    mata: vckss__stata_prune_graph(                              ///
        "`graph_worker'", "`graph_firm'", "`graph_frequency'", ///
        "`graph_deletion'", "`graph_sample'", "match",         ///
        "`graph_keep'", "`graph_diagnostics'",                  ///
        "graph_status", "graph_message")
    if "`graph_status'" != "CONVERGED" {
        di as error `"`graph_message'"'
        exit 498
    }
    quietly count if !`graph_keep'
    local graph_removed_rows = `graph_removed_rows' + r(N)
    quietly keep if `graph_keep'
    drop `graph_worker' `graph_firm' `graph_deletion' ///
        `graph_frequency' `graph_sample' `graph_keep'

    quietly egen long `graph_worker' = group(worker)
    quietly egen long `graph_firm' = group(firm)
    quietly generate byte `graph_frequency' = 1
    quietly generate byte `graph_sample' = 1
    quietly generate byte `bridge_keep' = 0
    local bridge_status
    local bridge_message
    mata: vckss_sep__stata_prune_bridges(                       ///
        "`graph_worker'", "`graph_firm'", "`graph_frequency'", ///
        "`graph_sample'", "`bridge_keep'",                      ///
        "`bridge_diagnostics'", "bridge_status", "bridge_message")
    if "`bridge_status'" != "CONVERGED" {
        di as error `"`bridge_message'"'
        exit 498
    }
    local bridge_count = `bridge_diagnostics'[1,1]
    local bridge_removed_matches = `bridge_removed_matches' + `bridge_count'
    if `bridge_count' > 0 {
        quietly count if !`bridge_keep'
        local bridge_removed_rows = `bridge_removed_rows' + r(N)
        quietly keep if `bridge_keep'
    }
    drop `graph_worker' `graph_firm' `graph_frequency' ///
        `graph_sample' `bridge_keep'
    local repeat = (`bridge_count' > 0)
}

sort worker observation_key
isid observation_key
quietly count
local stored_rows = r(N)
egen byte __worker_tag = tag(worker)
egen byte __firm_tag = tag(firm)
egen byte __match_tag = tag(worker firm)
quietly count if __worker_tag
local workers = r(N)
quietly count if __firm_tag
local firms = r(N)
quietly count if __match_tag
local matches = r(N)
drop __worker_tag __firm_tag __match_tag
if `stored_rows' == 0 | `workers' < 2 | `firms' < 2 | `matches' < 2 {
    di as error "MATLAB-retained bridge core is empty or degenerate"
    exit 498
}
save `"`output_dir'/prepared.dta"', replace
preserve
keep worker firm period y_minus_xb
export delimited using `"`output_dir'/prepared.csv"', replace
restore
timer off 82
quietly timer list 82
local preparation_seconds = r(t82)

clear
set obs 1
generate str64 label = "`label'"
generate str8 sample_mode = "matlab"
generate str32 sample_selection = "matlab_retained_bridge_core"
generate str40 source_commit = "`source_commit'"
generate str40 matlab_source_commit = "`matlab_source_commit'"
generate str64 parent_prepared_sha256 = "`prepared_sha'"
generate str64 matlab_detail_sha256 = "`matlab_detail_sha'"
generate str64 wage_input_sha256 = "`wage_input_sha'"
generate double input_rows = `input_rows'
generate double input_workers = `input_workers'
generate double input_firms = `input_firms'
generate double matlab_matches = `matlab_matches'
generate double matlab_rows = `matlab_rows'
generate double graph_removed_rows = `graph_removed_rows'
generate double bridge_removed_matches = `bridge_removed_matches'
generate double bridge_removed_rows = `bridge_removed_rows'
generate double audit_iterations = `iterations'
generate double stored_rows = `stored_rows'
generate double workers = `workers'
generate double firms = `firms'
generate double matches = `matches'
generate double preparation_seconds = `preparation_seconds'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
export delimited using `"`output_dir'/prepare.csv"', replace

tempname marker
file open `marker' using `"`output_dir'/prepare.stata.pass"', ///
    write text replace
file write `marker' ///
    "VARCOMP_KSS MATLAB SAMPLE PREPARE PASS `label' `source_commit'" _n
file close `marker'
di as result "VARCOMP_KSS MATLAB SAMPLE PREPARE PASS: `label'"
exit 0
