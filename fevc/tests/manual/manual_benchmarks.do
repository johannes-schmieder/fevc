/*
    Referee-facing fevc and maintained-MATLAB performance diagnostics.

    Data generation and MATLAB process/pool/MEX setup are outside the reported
    estimator command times.  MATLAB is launched in a fresh process for every
    repetition; its complete wrapper time is retained separately.  These are
    accessible local diagnostics, not release or qualification evidence.
*/

version 18.0
clear all
set more off
set linesize 255
set varabbrev off

/* ---------------------------- User settings ---------------------------- */
local manual_directory ""          // Empty: search common repository locations.
local matlab_root      ""          // Or set KSS_MATLAB_ROOT in the environment.
local matlab_binary    "matlab"    // Full executable path if needed.
local output_directory "fevc_manual_benchmark_output"
local dataset_sizes    "2400 9600"
local thread_counts    "1 4"
local repetitions      1
local probes           200
local seed             20260831
/* ----------------------------------------------------------------------- */

local environment_override : environment FEVC_MANUAL_SIZES
if strtrim(`"`environment_override'"') != "" {
    local dataset_sizes `"`environment_override'"'
}
local environment_override : environment FEVC_MANUAL_THREADS
if strtrim(`"`environment_override'"') != "" {
    local thread_counts `"`environment_override'"'
}
local environment_override : environment FEVC_MANUAL_REPETITIONS
if strtrim(`"`environment_override'"') != "" {
    local repetitions = real(`"`environment_override'"')
}
local environment_override : environment FEVC_MANUAL_OUTPUT
if strtrim(`"`environment_override'"') != "" {
    local output_directory `"`environment_override'"'
}

if `"`matlab_root'"' == "" local matlab_root : environment KSS_MATLAB_ROOT
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
adopath ++ `"`manual_directory'"'
capture findfile fevc.ado
if _rc {
    display as error "fevc is not installed or is not on the ado-path"
    exit 111
}
capture confirm file `"`matlab_root'/codes/leave_out_KSS.m"'
if _rc {
    display as error ///
        "Set matlab_root or KSS_MATLAB_ROOT to a maintained LeaveOutTwoWay checkout"
    exit 601
}
if `repetitions' < 1 | `repetitions' != floor(`repetitions') | ///
        `probes' < 2 | `probes' != floor(`probes') {
    display as error "repetitions and probes must be positive integers"
    exit 198
}
foreach rows of local dataset_sizes {
    if real("`rows'") < 2400 | mod(real("`rows'"), 120) != 0 {
        display as error "each dataset size must be at least 2400 and divisible by 120"
        exit 198
    }
}
foreach threads of local thread_counts {
    if real("`threads'") < 1 | real("`threads'") != floor(real("`threads'")) | ///
            real("`threads'") > c(processors) {
        display as error ///
            "each thread count must be an integer between 1 and c(processors)"
        exit 198
    }
}
capture mkdir `"`output_directory'"'
capture confirm file `"`output_directory'/manual_benchmark_runs.dta"'
if !_rc display as text "Existing benchmark outputs will be replaced"

local original_processors = c(processors)
frame create __fmb_runs                                      ///
    long rows workers firms int threads repetition sequence  ///
    str8 first str8 fevc_status str8 matlab_status            ///
    int fevc_rc matlab_rc double fevc_seconds matlab_seconds  ///
    matlab_wrapper_seconds matlab_pool_startup_seconds        ///
    matlab_mex_setup_seconds corrected_max_abs                ///
    corrected_max_scaled str12 backend str100 note

display as text "fevc manual benchmark"
display as text "  Stata:       `c(stata_version)' `c(flavor)'"
display as text `"  MATLAB root: `matlab_root'"'
display as text `"  sizes:       `dataset_sizes'"'
display as text `"  threads:     `thread_counts'"'
display as text "  repetitions: `repetitions'"
display as text "  probes:      `probes'"

local sequence 0
foreach rows of local dataset_sizes {
    local degree 3
    local workers = `rows' / `degree'
    local firms = `workers' / 40
    clear
    set obs `rows'
    generate long observation_key = _n
    generate long match = _n
    generate long worker = floor((_n-1)/`degree') + 1
    generate byte period = mod(_n-1, `degree') + 1
    generate long layer = floor((worker-1)/`firms')
    generate long base_firm = mod(worker-1, `firms')
    generate long offset = 0
    replace offset = 1 + mod(layer, floor(`firms'/4)-1) if period == 2
    replace offset = ceil(`firms'/3) + mod(97*layer, floor(`firms'/4)) ///
        if period == 3
    generate long firm = mod(base_firm + offset, `firms') + 1
    generate double y = mod(worker, 257)/16 + mod(firm, 127)/32 + ///
        period/64 + mod(match, 13)/128
    drop layer base_firm offset
    sort worker period firm
    isid observation_key
    isid worker firm

    foreach threads of local thread_counts {
        set processors `threads'

        /* Warm command loading and sample preparation outside timings. */
        capture quietly fevc y if observation_key <= 480,              ///
            worker(worker) firm(firm) deletion(match) deletionid(match) ///
            stayers(movers) algorithm(jla) probes(20) seed(`seed')      ///
            backend(mata) rng(stata) nodisplay

        forvalues repetition = 1/`repetitions' {
            local sequence = `sequence' + 1
            local matlab_first = mod(`sequence', 2)
            local first = cond(`matlab_first', "matlab", "fevc")
            local fevc_rc .
            local matlab_rc .
            local fevc_seconds .
            local matlab_seconds .
            local matlab_wrapper_seconds .
            local matlab_pool_startup .
            local matlab_mex_setup .
            local backend
            tempname fevc_result matlab_result

            forvalues position = 1/2 {
                local run_matlab = cond(`matlab_first', `position' == 1, ///
                    `position' == 2)
                if `run_matlab' {
                    capture noisily fevc_matlab y, worker(worker) firm(firm) ///
                        order(period) matlabroot(`"`matlab_root'"')          ///
                        matlab(`"`matlab_binary'"') deletion(match)         ///
                        algorithm(jla) probes(`probes') seed(`seed')        ///
                        threads(`threads')
                    local matlab_rc = _rc
                    if !`matlab_rc' {
                        matrix `matlab_result' = r(kss)
                        local matlab_seconds = r(command_seconds)
                        local matlab_wrapper_seconds = r(wrapper_seconds)
                        local matlab_pool_startup = r(pool_startup_seconds)
                        local matlab_mex_setup = r(mex_setup_seconds)
                    }
                }
                else {
                    timer clear 80
                    timer on 80
                    capture quietly fevc y, worker(worker) firm(firm)      ///
                        deletion(match) deletionid(match) stayers(movers)  ///
                        probeorder(observation_key) algorithm(jla)         ///
                        probes(`probes') seed(`seed') backend(auto)        ///
                        rng(auto) nodisplay
                    local fevc_rc = _rc
                    timer off 80
                    quietly timer list 80
                    local fevc_seconds = r(t80)
                    timer clear 80
                    if !`fevc_rc' {
                        matrix `fevc_result' = e(kss)
                        local backend `"`e(backend_selected)'"'
                    }
                }
            }

            local fevc_status = cond(`fevc_rc' == 0, "pass", "fail")
            local matlab_status = cond(`matlab_rc' == 0, "pass", "fail")
            local max_abs .
            local max_scaled .
            local note "descriptive difference; formulas and random draws are not equalized"
            if !`fevc_rc' & !`matlab_rc' {
                local max_abs 0
                local max_scaled 0
                forvalues column = 1/4 {
                    local difference = abs(`fevc_result'[1,`column'] - ///
                        `matlab_result'[1,`column'])
                    local scale = max(1, abs(`fevc_result'[1,`column']), ///
                        abs(`matlab_result'[1,`column']))
                    local max_abs = max(`max_abs', `difference')
                    local max_scaled = max(`max_scaled', `difference'/`scale')
                }
            }
            else local note "one or both estimator calls failed"

            frame post __fmb_runs (`rows') (`workers') (`firms')       ///
                (`threads') (`repetition') (`sequence') ("`first'")   ///
                ("`fevc_status'") ("`matlab_status'") (`fevc_rc')   ///
                (`matlab_rc') (`fevc_seconds') (`matlab_seconds')     ///
                (`matlab_wrapper_seconds') (`matlab_pool_startup')    ///
                (`matlab_mex_setup') (`max_abs') (`max_scaled')       ///
                (`"`backend'"') (`"`note'"')
            display as text "  N=`rows' threads=`threads' repetition=`repetition': " ///
                as result "fevc `fevc_status', MATLAB `matlab_status'"
        }
    }
}

frame change __fmb_runs
sort rows threads repetition
format fevc_seconds matlab_seconds matlab_wrapper_seconds %10.3f
format corrected_max_abs corrected_max_scaled %10.3e
save `"`output_directory'/manual_benchmark_runs.dta"', replace
export delimited using ///
    `"`output_directory'/manual_benchmark_runs.csv"', replace

preserve
collapse (median) fevc_seconds matlab_seconds matlab_wrapper_seconds ///
    corrected_max_abs corrected_max_scaled, by(rows workers firms threads)
generate double fevc_over_matlab = fevc_seconds / matlab_seconds
save `"`output_directory'/manual_benchmark_summary.dta"', replace
export delimited using ///
    `"`output_directory'/manual_benchmark_summary.csv"', replace

foreach threads of local thread_counts {
    twoway (connected fevc_seconds rows if threads == `threads',       ///
                sort msymbol(circle))                                 ///
           (connected matlab_seconds rows if threads == `threads',    ///
                sort msymbol(triangle)),                              ///
        xscale(log) yscale(log)                                       ///
        xtitle("Stored observations (log scale)")                     ///
        ytitle("Estimator command seconds (log scale)")              ///
        title("fevc and MATLAB: `threads' worker(s)")                 ///
        legend(order(1 "fevc" 2 "maintained MATLAB"))
    graph export `"`output_directory'/timing_threads`threads'.png"', ///
        replace width(1800)
}

display as text _newline "Median command-time summary"
list rows threads fevc_seconds matlab_seconds fevc_over_matlab       ///
    matlab_wrapper_seconds corrected_max_scaled, noobs abbreviate(24)
restore
set processors `original_processors'

quietly count if fevc_status == "fail" | matlab_status == "fail"
local failures = r(N)
local total_pairs = _N
if `failures' {
    display as error ///
        "FEVC_MANUAL_BENCHMARK|FAIL|failed_pairs=`failures'|output=`output_directory'"
    exit 459
}
display as result ///
    "FEVC_MANUAL_BENCHMARK|PASS|pairs=`total_pairs'|output=`output_directory'"
