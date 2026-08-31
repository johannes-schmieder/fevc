/*
    Referee-facing validation suite for fevc.

    The suite simulates an all-mover design with one observation per match,
    validates deterministic exact behavior and JLA uncertainty internally,
    tries the optional native exact backend, and calls the maintained MATLAB
    package through fevc_matlab.  The MATLAB row is a transparent behavioral
    comparison: the maintained package's current correction formula is not an
    equality oracle for fevc, so the numerical difference is reported but is
    not used as a pass/fail gate.
*/

version 18.0
clear all
set more off
set linesize 255
set varabbrev off

/* ---------------------------- User settings ---------------------------- */
local manual_directory ""       // Empty: search common repository locations.
local matlab_root      ""       // Local config or KSS_MATLAB_ROOT environment variable.
local matlab_binary    "matlab" // Local config may set the full executable path.
local threads          1
local probes           200
local seed             20260831
local strict           1        // Return r(459) if a required validation fails.
local require_matlab   1        // Set to zero to allow a skipped MATLAB comparison.
/* ----------------------------------------------------------------------- */

if `"`manual_directory'"' == "" {
    foreach candidate in "fevc/tests/manual" "tests/manual" "." {
        capture confirm file `"`candidate'/fevc_matlab.ado"'
        if !_rc & `"`manual_directory'"' == "" {
            local manual_directory `"`c(pwd)'/`candidate'"'
        }
    }
}
capture confirm file `"`manual_directory'/fevc_matlab.ado"'
if _rc {
    display as error ///
        "Set manual_directory to the directory containing the manual test files"
    exit 601
}
local local_settings `"`manual_directory'/.fevc_manual_local.do"'
capture confirm file `"`local_settings'"'
if !_rc include `"`local_settings'"'
if `"`matlab_root'"' == "" {
    local matlab_root : environment KSS_MATLAB_ROOT
}
adopath ++ `"`manual_directory'"'

if `threads' < 1 | `threads' != floor(`threads') | ///
        `threads' > c(processors) {
    display as error "threads must be an integer between 1 and c(processors)"
    exit 198
}
if `probes' < 2 | `probes' != floor(`probes') | ///
        !inlist(`strict', 0, 1) | !inlist(`require_matlab', 0, 1) {
    display as error ///
        "probes must be at least two; strict and require_matlab must be zero or one"
    exit 198
}
capture findfile fevc.ado
if _rc {
    display as error "fevc is not installed or is not on the ado-path"
    exit 111
}
local fevc_path `"`r(fn)'"'

local have_matlab 1
if strtrim(`"`matlab_root'"') == "" local have_matlab 0
if `have_matlab' {
    capture confirm file `"`matlab_root'/codes/leave_out_KSS.m"'
    if _rc local have_matlab 0
}
if !`have_matlab' & `require_matlab' {
    display as error ///
        "Set matlab_root or KSS_MATLAB_ROOT to a maintained LeaveOutTwoWay checkout"
    exit 601
}

local original_processors = c(processors)
set processors `threads'
display as text "fevc referee validation suite"
display as text `"  fevc:        `fevc_path'"'
display as text "  Stata:       `c(stata_version)' `c(flavor)'"
display as text "  OS/machine:  `c(os)' / `c(machine_type)'"
display as text "  processors:  `threads'"
display as text "  probes:      `probes'"
if `have_matlab' display as text `"  MATLAB root: `matlab_root'"'
else display as text "  MATLAB:      skipped (root unavailable)"

frame create __fvm_results                                      ///
    str32 test_id str18 comparator str10 status byte failed      ///
    int rc double rows retained_units fevc_seconds matlab_seconds ///
    matlab_wrapper_seconds max_abs max_scaled str100 note

/* Strong, deterministic all-mover graph.  Each row is its own match, so the
   stored-row, physical-observation, match, and target masses coincide. */
local workers = 160
local firms = 20
local degree = 3
local rows = `workers' * `degree'
set obs `rows'
generate long observation_key = _n
generate long match = _n
generate long worker = floor((_n-1)/`degree') + 1
generate byte period = mod(_n-1, `degree') + 1
generate long layer = floor((worker-1)/`firms')
generate long base_firm = mod(worker-1, `firms')
generate long offset = 0
replace offset = 1 + mod(layer, 4) if period == 2
replace offset = 7 + mod(97*layer, 5) if period == 3
generate long firm = mod(base_firm + offset, `firms') + 1
generate double y = mod(worker, 37)/8 + mod(firm, 19)/16 + ///
    period/32 + mod(match, 11)/64
drop layer base_firm offset
sort worker period firm
isid observation_key
isid worker firm
quietly _datasignature
local data_signature `"`r(datasignature)'"'

/* ------------------ Deterministic exact repeatability ------------------ */
timer clear 80
timer on 80
capture quietly fevc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) stayers(movers) algorithm(exact)           ///
    backend(mata) rng(stata) nodisplay
local exact_rc = _rc
timer off 80
quietly timer list 80
local exact_seconds = r(t80)
timer clear 80
local exact_failed = (`exact_rc' != 0)
local exact_note
local retained .
tempname exact_result repeat_result
if !`exact_rc' {
    matrix `exact_result' = e(kss)
    local retained = e(N_retained)
    local exact_failed = (`retained' != `rows')
    forvalues column = 1/4 {
        local scale = max(1, abs(`exact_result'[1,`column']))
        if abs(`exact_result'[1,`column']) >= . local exact_failed = 1
    }
    capture quietly fevc y, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) stayers(movers) algorithm(exact)           ///
        backend(mata) rng(stata) nodisplay
    local repeat_rc = _rc
    if !`repeat_rc' matrix `repeat_result' = e(kss)
    if `repeat_rc' {
        local exact_failed 1
        local exact_note "exact repeat returned r(`repeat_rc')"
    }
    else if mreldif(`exact_result', `repeat_result') != 0 {
        local exact_failed 1
        local exact_note "exact repeat changed"
    }
}
else local exact_note "fevc exact returned r(`exact_rc')"
local exact_status = cond(`exact_failed', "fail", "pass")
frame post __fvm_results ("mata_exact_repeat") ("fevc")        ///
    ("`exact_status'") (`exact_failed') (`exact_rc') (`rows')   ///
    (`retained') (`exact_seconds') (.) (.) (0) (0) (`"`exact_note'"')
if `exact_failed' display as error "FAIL  mata_exact_repeat: `exact_note'"
else display as result "PASS  mata_exact_repeat"

/* ----------------------- JLA numerical envelope ----------------------- */
local jla_failed 1
local jla_rc .
local jla_max_abs .
local jla_max_scaled .
local jla_seconds .
local jla_note "exact baseline unavailable"
if !`exact_failed' {
    timer clear 80
    timer on 80
    capture quietly fevc y, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) stayers(movers) algorithm(jla)             ///
        probes(`probes') seed(`seed') backend(mata) rng(stata) nodisplay
    local jla_rc = _rc
    timer off 80
    quietly timer list 80
    local jla_seconds = r(t80)
    timer clear 80
    if !`jla_rc' {
        tempname jla_result jla_mcse
        matrix `jla_result' = e(kss)
        matrix `jla_mcse' = e(numerical_mcse)
        local jla_failed 0
        local jla_max_abs 0
        local jla_max_scaled 0
        local jla_note
        forvalues column = 1/4 {
            local difference = abs(`jla_result'[1,`column'] - ///
                `exact_result'[1,`column'])
            local scale = max(1, abs(`jla_result'[1,`column']), ///
                abs(`exact_result'[1,`column']))
            local allowance = max(1e-8*`scale', 6*`jla_mcse'[1,`column'])
            local scaled = `difference' / `allowance'
            local jla_max_abs = max(`jla_max_abs', `difference')
            local jla_max_scaled = max(`jla_max_scaled', `scaled')
            if `difference' > `allowance' local jla_failed 1
        }
        if `jla_failed' local jla_note "JLA difference exceeded its numerical envelope"
    }
    else local jla_note "fevc JLA returned r(`jla_rc')"
}
local jla_status = cond(`jla_failed', "fail", "pass")
frame post __fvm_results ("mata_jla_vs_exact") ("fevc")       ///
    ("`jla_status'") (`jla_failed') (`jla_rc') (`rows')       ///
    (`retained') (`jla_seconds') (.) (.) (`jla_max_abs')       ///
    (`jla_max_scaled') (`"`jla_note'"')
if `jla_failed' display as error "FAIL  mata_jla_vs_exact: `jla_note'"
else display as result "PASS  mata_jla_vs_exact"

/* ---------------------- Optional native exact parity ------------------ */
local rust_failed 0
local rust_status "skip"
local rust_rc .
local rust_max_abs .
local rust_max_scaled .
local rust_seconds .
local rust_note "native runtime unavailable"
if !`exact_failed' {
    timer clear 80
    timer on 80
    capture quietly fevc y, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) stayers(movers) algorithm(exact)           ///
        backend(rust) nodisplay
    local rust_rc = _rc
    timer off 80
    quietly timer list 80
    local rust_seconds = r(t80)
    timer clear 80
    if !`rust_rc' {
        tempname rust_result
        matrix `rust_result' = e(kss)
        local rust_status "pass"
        local rust_note
        local rust_max_abs 0
        local rust_max_scaled 0
        forvalues column = 1/4 {
            local difference = abs(`rust_result'[1,`column'] - ///
                `exact_result'[1,`column'])
            local scale = max(1, abs(`rust_result'[1,`column']), ///
                abs(`exact_result'[1,`column']))
            local scaled = `difference' / (1e-8*`scale')
            local rust_max_abs = max(`rust_max_abs', `difference')
            local rust_max_scaled = max(`rust_max_scaled', `scaled')
            if `scaled' > 1 local rust_failed 1
        }
        if `rust_failed' {
            local rust_status "fail"
            local rust_note "native exact differs from Mata exact"
        }
    }
}
frame post __fvm_results ("rust_exact_vs_mata") ("fevc")      ///
    ("`rust_status'") (`rust_failed') (`rust_rc') (`rows')     ///
    (`retained') (`rust_seconds') (.) (.) (`rust_max_abs')      ///
    (`rust_max_scaled') (`"`rust_note'"')
if "`rust_status'" == "pass" display as result "PASS  rust_exact_vs_mata"
else if "`rust_status'" == "fail" display as error ///
    "FAIL  rust_exact_vs_mata: `rust_note'"
else display as text "SKIP  rust_exact_vs_mata (native runtime unavailable)"

/* ---------------- Maintained MATLAB behavioral comparison ------------- */
local matlab_failed 0
local matlab_status "skip"
local matlab_rc .
local matlab_seconds .
local wrapper_seconds .
local matlab_retained .
local matlab_max_abs .
local matlab_max_scaled .
local matlab_note "MATLAB root unavailable"
if `have_matlab' & !`exact_failed' {
    capture noisily fevc_matlab y, worker(worker) firm(firm) order(period) ///
        matlabroot(`"`matlab_root'"') matlab(`"`matlab_binary'"')        ///
        deletion(match) algorithm(exact) probes(`probes') seed(`seed')   ///
        threads(`threads')
    local matlab_rc = _rc
    if `matlab_rc' {
        local matlab_failed 1
        local matlab_status "fail"
        local matlab_note "MATLAB bridge returned r(`matlab_rc')"
    }
    else {
        tempname matlab_result
        matrix `matlab_result' = r(kss)
        local matlab_seconds = r(command_seconds)
        local wrapper_seconds = r(wrapper_seconds)
        local matlab_retained = r(retained_units)
        local matlab_status "pass"
        local matlab_note "descriptive only: maintained correction formula is not an equality oracle"
        if `matlab_retained' != `rows' {
            local matlab_failed 1
            local matlab_status "fail"
            local matlab_note "MATLAB retained-unit count changed"
        }
        local matlab_max_abs 0
        local matlab_max_scaled 0
        forvalues column = 1/4 {
            local difference = abs(`matlab_result'[1,`column'] - ///
                `exact_result'[1,`column'])
            local scale = max(1, abs(`matlab_result'[1,`column']), ///
                abs(`exact_result'[1,`column']))
            local matlab_max_abs = max(`matlab_max_abs', `difference')
            local matlab_max_scaled = max(`matlab_max_scaled', ///
                `difference'/`scale')
        }
    }
}
frame post __fvm_results ("matlab_exact_descriptive")          ///
    ("maintained MATLAB") ("`matlab_status'") (`matlab_failed') ///
    (`matlab_rc') (`rows') (`matlab_retained') (`exact_seconds') ///
    (`matlab_seconds') (`wrapper_seconds') (`matlab_max_abs')    ///
    (`matlab_max_scaled') (`"`matlab_note'"')
if "`matlab_status'" == "pass" display as result ///
    "PASS  matlab_exact_descriptive (comparison reported; no equality gate)"
else if "`matlab_status'" == "fail" display as error ///
    "FAIL  matlab_exact_descriptive: `matlab_note'"
else display as text "SKIP  matlab_exact_descriptive (MATLAB unavailable)"

quietly _datasignature
assert `"`r(datasignature)'"' == `"`data_signature'"'

/* ------------------------------ Summary ------------------------------- */
frame change __fvm_results
sort status test_id
format fevc_seconds matlab_seconds matlab_wrapper_seconds %10.3f
format max_abs max_scaled %10.3e
quietly count
local total_tests = r(N)
quietly count if status == "pass"
local passed_tests = r(N)
quietly count if status == "fail"
local failed_tests = r(N)
quietly count if status == "skip"
local skipped_tests = r(N)

display as text _newline "Referee validation summary"
display as text "  passed:  " as result `passed_tests'
display as text "  failed:  " as result `failed_tests'
display as text "  skipped: " as result `skipped_tests'
list test_id comparator status rc retained_units fevc_seconds       ///
    matlab_seconds matlab_wrapper_seconds max_abs max_scaled note,  ///
    noobs abbreviate(30)

set processors `original_processors'
local final_rc 0
if `failed_tests' local final_rc 459
if `require_matlab' & `have_matlab' == 0 local final_rc 459
if `final_rc' {
    display as error ///
        "FEVC_REFEREE_SUITE|FAIL|tests=`total_tests'|passed=`passed_tests'|failed=`failed_tests'|skipped=`skipped_tests'"
    if `strict' exit `final_rc'
}
else if `skipped_tests' {
    display as text ///
        "FEVC_REFEREE_SUITE|INCOMPLETE|tests=`total_tests'|passed=`passed_tests'|failed=0|skipped=`skipped_tests'"
}
else display as result ///
    "FEVC_REFEREE_SUITE|PASS|tests=`total_tests'|passed=`passed_tests'|failed=0|skipped=0"
