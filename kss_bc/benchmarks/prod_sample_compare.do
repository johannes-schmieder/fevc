version 18.0
clear all
set more off
set varabbrev off

args experiment_id dataset retained_csv matlab_detail matlab_detail_sha ///
    prepared_sha wage_sha data_manifest_sha bundle_sha source_commit ///
    processors_arg output_dir

local requested_processors = real("`processors_arg'")

if !ustrregexm("`experiment_id'", "^[A-Za-z0-9._-]+$") | ///
    !inlist("`dataset'", "cz24", "cz25") | ///
    !ustrregexm("`matlab_detail_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`prepared_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`wage_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`data_manifest_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`bundle_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !inlist(`requested_processors', 4, 8) {
    di as error "invalid KSS-PROD sample-comparison arguments"
    exit 198
}
confirm file `"`retained_csv'"'
confirm file `"`matlab_detail'"'
capture set processors `requested_processors'
if c(processors) != `requested_processors' | c(MP) != 1 {
    di as error "Stata/MP license lacks requested sample-comparison capacity"
    exit 459
}

timer clear 97
timer on 97
quietly import delimited using `"`retained_csv'"', varnames(1) clear
confirm numeric variable worker firm
keep worker firm
drop if missing(worker, firm)
duplicates drop
sort worker firm
isid worker firm
quietly count
local kss_match_count = r(N)
generate byte __in_kss = 1
tempfile kss_keys
save `kss_keys'

quietly import delimited using `"`matlab_detail'"', delimiters(tab) ///
    varnames(nonames) numericcols(_all) clear
ds
local detail_variables `r(varlist)'
local detail_count : word count `detail_variables'
if `detail_count' != 4 {
    di as error "maintained MATLAB detail must contain exactly four columns"
    exit 498
}
confirm numeric variable v1 v2 v3 v4
assert !missing(v2, v3)
assert v2 == floor(v2) & v3 == floor(v3)
rename v2 worker
rename v3 firm
keep worker firm
duplicates drop
sort worker firm
isid worker firm
quietly count
local matlab_matches = r(N)
generate byte __in_matlab = 1
quietly merge 1:1 worker firm using `kss_keys'
quietly count if _merge == 1
local matlab_only = r(N)
quietly count if _merge == 2
local kss_only = r(N)
timer off 97
quietly timer list 97
local comparison_seconds = r(t97)

clear
set obs 1
generate str64 experiment_id = "`experiment_id'"
generate str8 dataset = "`dataset'"
generate str40 source_commit = "`source_commit'"
generate str64 bundle_sha256 = "`bundle_sha'"
generate str64 data_manifest_sha256 = "`data_manifest_sha'"
generate str64 prepared_sha256 = "`prepared_sha'"
generate str64 wage_input_sha256 = "`wage_sha'"
generate str64 matlab_detail_sha256 = "`matlab_detail_sha'"
generate double kss_matches = `kss_match_count'
generate double matlab_matches = `matlab_matches'
generate double kss_only = `kss_only'
generate double matlab_only = `matlab_only'
generate double comparison_seconds = `comparison_seconds'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
generate byte stata_mp = c(MP)
generate double requested_processors = `requested_processors'
generate double actual_processors = c(processors)
generate str32 comparison_status = cond(`kss_only' == 0 & ///
    `matlab_only' == 0, "RETAINED_SAMPLE_EQUAL", "RETAINED_SAMPLE_DIFFERENT")
export delimited using `"`output_dir'/sample_comparison.csv"', replace

if `kss_only' != 0 | `matlab_only' != 0 {
    di as error "KSS and maintained MATLAB retained match sets differ"
    exit 459
}
tempname marker
file open `marker' using `"`output_dir'/sample_comparison.stata.pass"', ///
    write text replace
file write `marker' "KSS_PROD SAMPLE COMPARISON PASS `experiment_id'" _n
file close `marker'
di as result "KSS_PROD SCC SAMPLE COMPARISON PASS: `experiment_id'"
exit 0
