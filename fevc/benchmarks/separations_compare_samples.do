version 18.0
clear all
set more off
set varabbrev off

args label run_dir source_commit
if !ustrregexm("`label'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") {
    exit 198
}
local root `"`run_dir'/separations/`label'"'
confirm file `"`root'/b1/retained_matches.dta"'

quietly use `"`root'/b1/retained_matches.dta"', clear
isid worker firm
generate byte in_b1 = 1
tempfile b1
save `b1'
quietly count
local b1_matches = r(N)

local cmg_available = 0
local cmg_matches = .
local b1_only_cmg = .
local cmg_only_b1 = .
capture confirm file `"`root'/cmg/retained_matches.dta"'
if !_rc {
    local cmg_available = 1
    quietly use `"`root'/cmg/retained_matches.dta"', clear
    isid worker firm
    quietly count
    local cmg_matches = r(N)
    generate byte in_cmg = 1
    quietly merge 1:1 worker firm using `b1'
    quietly count if _merge == 1
    local cmg_only_b1 = r(N)
    quietly count if _merge == 2
    local b1_only_cmg = r(N)
}

local matlab_available = 0
local matlab_matches = .
local matlab_only_b1 = .
local b1_only_matlab = .
capture confirm file `"`root'/matlab/detailed.csv"'
if !_rc {
    local matlab_available = 1
    quietly import delimited using `"`root'/matlab/detailed.csv"', ///
        delimiters(tab) varnames(nonames) numericcols(_all) clear
    confirm numeric variable v1 v2 v3 v4
    rename v2 worker
    rename v3 firm
    keep worker firm
    drop if missing(worker,firm)
    duplicates drop
    sort worker firm
    isid worker firm
    quietly count
    local matlab_matches = r(N)
    generate byte in_matlab = 1
    quietly merge 1:1 worker firm using `b1'
    quietly count if _merge == 1
    local matlab_only_b1 = r(N)
    quietly count if _merge == 2
    local b1_only_matlab = r(N)
}

clear
set obs 1
generate str64 label = "`label'"
generate str40 source_commit = "`source_commit'"
generate double b1_matches = `b1_matches'
generate byte cmg_available = `cmg_available'
generate double cmg_matches = `cmg_matches'
generate double b1_only_cmg = `b1_only_cmg'
generate double cmg_only_b1 = `cmg_only_b1'
generate byte matlab_available = `matlab_available'
generate double matlab_matches = `matlab_matches'
generate double b1_only_matlab = `b1_only_matlab'
generate double matlab_only_b1 = `matlab_only_b1'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
export delimited using `"`root'/comparison/sample_overlap.csv"', replace

tempname marker
file open `marker' using `"`root'/comparison/sample_overlap.stata.pass"', ///
    write text replace
file write `marker' ///
    "FEVC SEPARATIONS SAMPLE COMPARISON PASS `label' `source_commit'" _n
file close `marker'
di as result "FEVC SEPARATIONS SAMPLE COMPARISON PASS: `label'"
exit 0
