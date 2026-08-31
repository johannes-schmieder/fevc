/*
    Referee-facing fevc and maintained-MATLAB performance diagnostics.

    The benchmark measures two slices rather than a full size-by-core grid:
      1. dataset size at eight cores; and
      2. cores at the medium dataset size.

    Each figure has three two-way panels: dense, sparse, and bottleneck.  The
    bottleneck graph consists of two internally connected firm communities
    joined by four bridge workers, so it remains connected after deleting any
    one match but has a deliberately weak cut.

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
capture graph drop _all

/* ---------------------------- User settings ---------------------------- */
local manual_directory    ""          // Empty: search common repository locations.
local matlab_root         ""          // Or set KSS_MATLAB_ROOT in the environment.
local matlab_binary       "matlab"    // Full executable path if needed.
local python_binary       "python3"   // Standard-library-only temporary Ado builder.
local output_directory    "fevc_manual_benchmark_output"
local problem_types       "two_way_dense two_way_sparse two_way_bottleneck"
local dataset_sizes       "12000 24000 48000"
local medium_dataset_size 24000
local fixed_threads       8
local thread_counts       "1 2 4 8 16"
local repetitions         1
local algorithm_settings  "default"  // "default" or "harmonized".
local probes              200        // Used only with "harmonized".
local seed                20260831
/* ----------------------------------------------------------------------- */

/* Automated smoke runs may reduce the grid without editing this file. */
local environment_override : environment FEVC_MANUAL_SIZES
if strtrim(`"`environment_override'"') != "" {
    local dataset_sizes `"`environment_override'"'
}
local environment_override : environment FEVC_MANUAL_MEDIUM_SIZE
if strtrim(`"`environment_override'"') != "" {
    local medium_dataset_size = real(`"`environment_override'"')
}
local environment_override : environment FEVC_MANUAL_FIXED_THREADS
if strtrim(`"`environment_override'"') != "" {
    local fixed_threads = real(`"`environment_override'"')
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
local environment_override : environment FEVC_MANUAL_ALGORITHM_SETTINGS
if strtrim(`"`environment_override'"') != "" {
    local algorithm_settings `"`environment_override'"'
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
local fevc_source `"`r(fn)'"'
local adapter_builder `"`manual_directory'/fevc_manual_build_ado.py"'
capture confirm file `"`adapter_builder'"'
if _rc {
    display as error "The manual Rust thread adapter builder is missing"
    exit 601
}
capture confirm file `"`matlab_root'/codes/leave_out_KSS.m"'
if _rc {
    display as error ///
        "Set matlab_root or KSS_MATLAB_ROOT to a maintained LeaveOutTwoWay checkout"
    exit 601
}

local algorithm_settings = lower(strtrim(`"`algorithm_settings'"'))
if !inlist(`"`algorithm_settings'"', "default", "harmonized") {
    display as error "algorithm_settings must be default or harmonized"
    exit 198
}
local harmonize_settings = (`"`algorithm_settings'"' == "harmonized")
if `repetitions' < 1 | `repetitions' != floor(`repetitions') {
    display as error "repetitions must be a positive integer"
    exit 198
}
if `harmonize_settings' & (`probes' < 2 | `probes' != floor(`probes')) {
    display as error "probes must be an integer of at least two"
    exit 198
}

local allowed_problem_types ///
    "two_way_dense two_way_sparse two_way_bottleneck"
foreach problem_type of local problem_types {
    local valid_problem : list problem_type in allowed_problem_types
    if !`valid_problem' {
        display as error "unknown problem type: `problem_type'"
        exit 198
    }
}

local medium_found 0
foreach rows of local dataset_sizes {
    if real("`rows'") < 2400 | mod(real("`rows'"), 120) != 0 {
        display as error ///
            "each dataset size must be at least 2400 and divisible by 120"
        exit 198
    }
    if real("`rows'") == `medium_dataset_size' local medium_found 1
}
if !`medium_found' {
    display as error "medium_dataset_size must be one of dataset_sizes"
    exit 198
}

local fixed_found 0
foreach requested_threads of local thread_counts {
    if real("`requested_threads'") < 1 |                        ///
       real("`requested_threads'") > 64 |                       ///
       real("`requested_threads'") != floor(real("`requested_threads'")) {
        display as error "thread_counts must contain integers from 1 through 64"
        exit 198
    }
    if real("`requested_threads'") == `fixed_threads' local fixed_found 1
}
if !`fixed_found' | `fixed_threads' < 1 | `fixed_threads' > 64 | ///
        `fixed_threads' != floor(`fixed_threads') {
    display as error "fixed_threads must be an integer in thread_counts"
    exit 198
}

/* Build and load a temporary source-bound Ado.  Only this in-memory manual
   benchmark copy reads the independent native-thread contract; the installed
   fevc command and its public defaults are not modified. */
tempfile manual_fevc_ado manual_adapter_receipt manual_adapter_log
local adapter_command                                      ///
    `""`python_binary'" "`adapter_builder'" --source "`fevc_source'" --output "`manual_fevc_ado'" --receipt "`manual_adapter_receipt'" > "`manual_adapter_log'" 2>&1"'
quietly shell `adapter_command'
capture confirm file `"`manual_fevc_ado'"'
local adapter_rc = _rc
capture confirm file `"`manual_adapter_receipt'"'
local adapter_rc = max(`adapter_rc', _rc)
if `adapter_rc' {
    display as error "Could not build the temporary manual benchmark Ado"
    capture noisily type `"`manual_adapter_log'"'
    exit 499
}
discard
capture noisily run `"`manual_fevc_ado'"'
local adapter_load_rc = _rc
if `adapter_load_rc' {
    display as error "Could not load the temporary manual benchmark Ado"
    exit `adapter_load_rc'
}
local original_manual_thread_contract ///
    `"${FEVC_MANUAL_THREAD_CONTRACT}"'
local original_manual_rust_threads `"${FEVC_MANUAL_RUST_THREADS}"'
local original_manual_stata_threads `"${FEVC_MANUAL_STATA_THREADS}"'
global FEVC_MANUAL_THREAD_CONTRACT ""
global FEVC_MANUAL_RUST_THREADS ""
global FEVC_MANUAL_STATA_THREADS ""

capture mkdir `"`output_directory'"'
capture confirm file `"`output_directory'/manual_benchmark_runs.dta"'
if !_rc display as text "Existing benchmark outputs will be replaced"
/* Remove only the legacy per-thread figures written by the previous driver. */
foreach legacy_threads in 1 2 4 8 16 {
    capture erase `"`output_directory'/timing_threads`legacy_threads'.png"'
}

local original_processors = c(processors)
local stata_thread_cap = min(c(processors_lic), c(processors_mach))
local tuning_mode `"`algorithm_settings'"'
local fevc_algorithm_options
local matlab_algorithm_options
if `harmonize_settings' {
    /* Maintained MATLAB uses fit/probe tolerances 1e-10/1e-6 and a 1,000
       iteration ceiling.  fevc's omitted phase tolerances are the same; only
       its maximum iterations and JLA request need to be made explicit. */
    local fevc_algorithm_options ///
        "algorithm(jla) probes(`probes') maxiter(1000)"
    local matlab_algorithm_options "algorithm(jla) probes(`probes')"
}

frame create __fvmb_runs                                              ///
    str24 problem_type byte panel str12 topology byte fe_dimensions  ///
    long rows long workers long firms int bridge_workers              ///
    int requested_threads int stata_threads                           ///
    int rust_threads_requested int rust_threads_used                  ///
    int repetition                                                     ///
    int sequence str8 first str8 mata_status str8 rust_status         ///
    str8 matlab_status int mata_rc int rust_rc int matlab_rc          ///
    double mata_seconds double rust_seconds double matlab_seconds     ///
    double matlab_wrapper_seconds double matlab_pool_startup_seconds  ///
    double matlab_mex_setup_seconds double mata_matlab_max_abs        ///
    double mata_matlab_max_scaled double rust_matlab_max_abs          ///
    double rust_matlab_max_scaled str12 mata_backend                  ///
    str12 rust_backend str12 mata_algorithm str12 rust_algorithm      ///
    str12 matlab_algorithm str12 tuning_mode str160 note

capture program drop __fvmb_generate
program define __fvmb_generate, rclass
    version 18.0
    args rows problem_type case_seed

    clear
    set rng mt64
    set seed `case_seed'
    local degree 6
    local workers = `rows' / `degree'
    /* Keep the number of firms even so the bottleneck design has two equal
       communities.  The shipped sizes retain 50, 100, and 200 firms. */
    local firms = max(20, 2*floor(`workers' / 80))
    local sparse = (`"`problem_type'"' == "two_way_sparse")
    local bottleneck = (`"`problem_type'"' == "two_way_bottleneck")

    quietly set obs `rows'
    generate long observation_key = _n
    generate long worker = floor((_n-1)/`degree') + 1
    generate byte period = mod(_n-1, `degree') + 1
    generate byte spell = ceil(period/2)
    generate long match = 3*(worker-1) + spell
    generate long layer = floor((worker-1)/`firms')
    generate long base_firm = mod(worker-1, `firms')
    local bridge_workers 0
    if `bottleneck' {
        local half_firms = `firms' / 2
        generate long community_start = ///
            cond(base_firm < `half_firms', 0, `half_firms')
        generate long within_firm = mod(base_firm, `half_firms')
        generate long firm = community_start + within_firm + 1
        replace firm = community_start + ///
            mod(within_firm + 1, `half_firms') + 1 if spell == 2
        replace firm = community_start + ///
            mod(within_firm + 3, `half_firms') + 1 if spell == 3

        /* Four workers provide four independent cross-community matches.
           Deleting any one match therefore does not disconnect the graph. */
        generate byte bridge_worker = base_firm < 2 & layer < 2
        replace firm = `half_firms' + base_firm + 1 ///
            if bridge_worker & spell == 3
        quietly count if bridge_worker & period == 1
        local bridge_workers = r(N)
        drop community_start within_firm
    }
    else {
        generate long offset = 0
        if `sparse' {
            replace offset = 1 if spell == 2
            replace offset = 2 if spell == 3
        }
        else {
            local quarter = max(2, floor(`firms'/4))
            replace offset = 1 + mod(layer, `quarter'-1) if spell == 2
            replace offset = ceil(`firms'/3) + mod(97*layer, `quarter') ///
                if spell == 3
        }
        generate long firm = mod(base_firm + offset, `firms') + 1
        generate byte bridge_worker = 0
    }

    generate double y = mod(worker, 257)/16 + mod(firm, 127)/32 + ///
        period/64 + mod(match, 13)/128
    capture drop offset
    drop layer base_firm bridge_worker spell
    sort worker period firm
    isid observation_key
    isid worker period

    return scalar workers = `workers'
    return scalar firms = `firms'
    return scalar bridge_workers = `bridge_workers'
    return scalar fe_dimensions = 2
    local topology = cond(`bottleneck', "bottleneck", ///
        cond(`sparse', "sparse", "dense"))
    return local topology `"`topology'"'
end

display as text "fevc manual benchmark"
display as text "  Stata:       `c(stata_version)' `c(flavor)'"
display as text `"  MATLAB root: `matlab_root'"'
display as text `"  designs:     `problem_types'"'
display as text `"  sizes:       `dataset_sizes'"'
display as text "  fixed cores: `fixed_threads'"
display as text `"  core levels: `thread_counts'"'
display as text "  medium size: `medium_dataset_size'"
display as text "  repetitions: `repetitions'"
display as text "  settings:    `tuning_mode'"
if `harmonize_settings' display as text ///
    "  matched JLA: `probes' probes; phase tolerances 1e-10/1e-6; maxiter 1000"

local pair_sequence 0
local problem_index 0
foreach problem_type of local problem_types {
    local ++problem_index
    local panel = `problem_index'
    local warmed 0
    foreach rows of local dataset_sizes {
        local case_seed = `seed' + 1000*`problem_index' + `rows'
        quietly __fvmb_generate `rows' `problem_type' `case_seed'
        local workers = r(workers)
        local firms = r(firms)
        local bridge_workers = r(bridge_workers)
        local fe_dimensions = r(fe_dimensions)
        local topology `"`r(topology)'"'

        /* Every size enters the fixed-core slice.  Only the medium size gets
           the remaining core levels.  The shared medium/eight-core cell is
           therefore executed once. */
        local cell_threads `"`fixed_threads'"'
        if `rows' == `medium_dataset_size' {
            foreach requested_threads of local thread_counts {
                if `requested_threads' != `fixed_threads' {
                    local cell_threads ///
                        `"`cell_threads' `requested_threads'"'
                }
            }
        }

        foreach requested_threads of local cell_threads {
            local stata_threads = min(`requested_threads', `stata_thread_cap')
            local mata_capped = (`requested_threads' > `stata_threads')
            local rust_threads = `requested_threads'
            quietly set processors `stata_threads'

            /* Warm command and plugin loading once per design, outside the
               reported timings. */
            if !`warmed' {
                capture quietly fevc y if observation_key <= 2400,      ///
                    worker(worker) firm(firm) deletion(match)           ///
                    deletionid(match) stayers(movers) algorithm(jla)    ///
                    probes(2) seed(`seed') backend(mata) rng(stata)     ///
                    nodisplay
                quietly set processors 1
                global FEVC_MANUAL_THREAD_CONTRACT ///
                    "FEVC-MANUAL-BENCHMARK-THREADS-V1"
                global FEVC_MANUAL_RUST_THREADS "1"
                global FEVC_MANUAL_STATA_THREADS "1"
                capture quietly fevc y if observation_key <= 2400,      ///
                    worker(worker) firm(firm) deletion(match)           ///
                    stayers(movers) probeorder(observation_key)         ///
                    algorithm(jla) engine(auto) preconditioner(auto)    ///
                    batch(auto) probes(2) seed(`seed')                  ///
                    backend(auto) rng(auto)                             ///
                    nodisplay
                global FEVC_MANUAL_THREAD_CONTRACT ///
                    `"`original_manual_thread_contract'"'
                global FEVC_MANUAL_RUST_THREADS ///
                    `"`original_manual_rust_threads'"'
                global FEVC_MANUAL_STATA_THREADS ///
                    `"`original_manual_stata_threads'"'
                quietly set processors `stata_threads'
                local warmed 1
            }

            forvalues repetition = 1/`repetitions' {
                local pair_sequence = `pair_sequence' + 1
                local arms "mata rust matlab"
                if `mata_capped' local arms "rust matlab"
                local arm_count : word count `arms'
                local first_index = mod(`pair_sequence'-1, `arm_count') + 1
                local first : word `first_index' of `arms'
                local mata_rc .
                local rust_rc .
                local matlab_rc .
                local mata_seconds .
                local rust_seconds .
                local matlab_seconds .
                local matlab_wrapper_seconds .
                local matlab_pool_startup .
                local matlab_mex_setup .
                local mata_backend
                local rust_backend
                local mata_algorithm
                local rust_algorithm
                local matlab_algorithm
                local rust_threads_requested .
                local rust_threads_used .
                tempname mata_result rust_result matlab_result

                /* Rotate the first implementation to limit order effects.
                   Above Stata's cap, Rust and MATLAB continue without Mata. */
                forvalues position = 1/`arm_count' {
                    local arm_index = mod(`first_index' + ///
                        `position' - 2, `arm_count') + 1
                    local arm : word `arm_index' of `arms'

                    if `"`arm'"' == "matlab" {
                        capture noisily fevc_matlab y, worker(worker)   ///
                            firm(firm) order(period)                    ///
                            matlabroot(`"`matlab_root'"')              ///
                            matlab(`"`matlab_binary'"')                ///
                            deletion(match) seed(`seed')               ///
                            threads(`requested_threads')               ///
                            `matlab_algorithm_options'
                        local matlab_rc = _rc
                        if !`matlab_rc' {
                            matrix `matlab_result' = r(kss)
                            local matlab_seconds = r(command_seconds)
                            local matlab_wrapper_seconds = r(wrapper_seconds)
                            local matlab_pool_startup = r(pool_startup_seconds)
                            local matlab_mex_setup = r(mex_setup_seconds)
                            local matlab_algorithm `"`r(selected_algorithm)'"'
                        }
                    }
                    else if `"`arm'"' == "mata" {
                        timer clear 80
                        timer on 80
                        capture quietly fevc y,                        ///
                            worker(worker) firm(firm) deletion(match)   ///
                            deletionid(match) stayers(movers)          ///
                            probeorder(observation_key) seed(`seed')    ///
                            backend(mata) rng(stata)                    ///
                            `fevc_algorithm_options' nodisplay
                        local mata_rc = _rc
                        timer off 80
                        quietly timer list 80
                        local mata_seconds = r(t80)
                        timer clear 80
                        if !`mata_rc' {
                            matrix `mata_result' = e(kss)
                            local mata_backend `"`e(backend_selected)'"'
                            local mata_algorithm `"`e(algorithm)'"'
                        }
                    }
                    else if `"`arm'"' == "rust" {
                        global FEVC_MANUAL_THREAD_CONTRACT ///
                            "FEVC-MANUAL-BENCHMARK-THREADS-V1"
                        global FEVC_MANUAL_RUST_THREADS ///
                            "`rust_threads'"
                        global FEVC_MANUAL_STATA_THREADS ///
                            "`stata_threads'"
                        timer clear 81
                        timer on 81
                        capture quietly fevc y,                        ///
                            worker(worker) firm(firm) deletion(match)   ///
                            stayers(movers)                            ///
                            probeorder(observation_key) seed(`seed')    ///
                            backend(auto) rng(auto)                     ///
                            `fevc_algorithm_options' nodisplay
                        local rust_rc = _rc
                        timer off 81
                        global FEVC_MANUAL_THREAD_CONTRACT ///
                            `"`original_manual_thread_contract'"'
                        global FEVC_MANUAL_RUST_THREADS ///
                            `"`original_manual_rust_threads'"'
                        global FEVC_MANUAL_STATA_THREADS ///
                            `"`original_manual_stata_threads'"'
                        quietly timer list 81
                        local rust_seconds = r(t81)
                        timer clear 81
                        if !`rust_rc' {
                            matrix `rust_result' = e(kss)
                            local rust_backend `"`e(backend_selected)'"'
                            local rust_algorithm `"`e(algorithm)'"'
                            capture local rust_threads_requested = ///
                                e(cmg_threads_requested)
                            local rust_thread_receipt_rc = _rc
                            capture local rust_threads_used = ///
                                e(cmg_threads_used)
                            local rust_thread_receipt_rc = ///
                                max(`rust_thread_receipt_rc', _rc)
                            if `rust_thread_receipt_rc' |             ///
                               `"`e(cmg_backend)'"' != "CMG_FULL_V2" | ///
                               `rust_threads_requested' != `rust_threads' | ///
                               `rust_threads_used' != `rust_threads' {
                                local rust_rc 498
                            }
                        }
                    }
                }

                local mata_status = cond(`mata_capped', "capped", ///
                    cond(`mata_rc' == 0, "pass", "fail"))
                local rust_status = cond(`rust_rc' == 0, "pass", "fail")
                local matlab_status = cond(`matlab_rc' == 0, "pass", "fail")
                local mata_max_abs .
                local mata_max_scaled .
                local rust_max_abs .
                local rust_max_scaled .
                local note "descriptive difference; formulas and random draws are not equalized"
                if `mata_capped' {
                    local note ///
                        "fevc Mata omitted: requested cores exceed the Stata processor cap"
                }
                if (`mata_rc' != 0 & !`mata_capped') | `rust_rc' != 0 | ///
                        `matlab_rc' != 0 {
                    local note "one or more estimator calls failed"
                }

                if `mata_rc' == 0 & `matlab_rc' == 0 {
                    local mata_max_abs 0
                    local mata_max_scaled 0
                    forvalues column = 1/4 {
                        local difference = abs(`mata_result'[1,`column'] - ///
                            `matlab_result'[1,`column'])
                        local scale = max(1, abs(`mata_result'[1,`column']), ///
                            abs(`matlab_result'[1,`column']))
                        local mata_max_abs = max(`mata_max_abs', `difference')
                        local mata_max_scaled = max(`mata_max_scaled', ///
                            `difference'/`scale')
                    }
                }
                if `rust_rc' == 0 & `matlab_rc' == 0 {
                    local rust_max_abs 0
                    local rust_max_scaled 0
                    forvalues column = 1/4 {
                        local difference = abs(`rust_result'[1,`column'] - ///
                            `matlab_result'[1,`column'])
                        local scale = max(1, abs(`rust_result'[1,`column']), ///
                            abs(`matlab_result'[1,`column']))
                        local rust_max_abs = max(`rust_max_abs', `difference')
                        local rust_max_scaled = max(`rust_max_scaled', ///
                            `difference'/`scale')
                    }
                }

                frame post __fvmb_runs                              ///
                    (`"`problem_type'"') (`panel') (`"`topology'"') ///
                    (`fe_dimensions') (`rows') (`workers') (`firms') ///
                    (`bridge_workers') (`requested_threads') (`stata_threads') ///
                    (`rust_threads_requested') (`rust_threads_used') ///
                    (`repetition') (`pair_sequence')                ///
                    (`"`first'"')                                  ///
                    (`"`mata_status'"') (`"`rust_status'"')        ///
                    (`"`matlab_status'"') (`mata_rc') (`rust_rc')   ///
                    (`matlab_rc') (`mata_seconds') (`rust_seconds')  ///
                    (`matlab_seconds') (`matlab_wrapper_seconds')    ///
                    (`matlab_pool_startup') (`matlab_mex_setup')     ///
                    (`mata_max_abs') (`mata_max_scaled')             ///
                    (`rust_max_abs') (`rust_max_scaled')             ///
                    (`"`mata_backend'"') (`"`rust_backend'"')      ///
                    (`"`mata_algorithm'"') (`"`rust_algorithm'"')  ///
                    (`"`matlab_algorithm'"')                        ///
                    (`"`tuning_mode'"') (`"`note'"')

                display as text                                     ///
                    "  `problem_type' N=`rows' cores=`requested_threads' repetition=`repetition': " ///
                    as result "Mata `mata_status', Rust `rust_status', MATLAB `matlab_status'"
            }
        }
    }
}

frame change __fvmb_runs
sort panel rows requested_threads repetition
format mata_seconds rust_seconds matlab_seconds ///
    matlab_wrapper_seconds %10.3f
format mata_matlab_max_abs mata_matlab_max_scaled ///
    rust_matlab_max_abs rust_matlab_max_scaled %10.3e
save `"`output_directory'/manual_benchmark_runs.dta"', replace
export delimited using ///
    `"`output_directory'/manual_benchmark_runs.csv"', replace

frame copy __fvmb_runs __fvmb_summary, replace
frame change __fvmb_summary
generate double mata_plot_seconds = mata_seconds if mata_status == "pass"
generate double rust_plot_seconds = rust_seconds if rust_status == "pass"
generate double matlab_plot_seconds = ///
    matlab_seconds if matlab_status == "pass"
collapse (median) mata_seconds=mata_plot_seconds                ///
                  rust_seconds=rust_plot_seconds                ///
                  matlab_seconds=matlab_plot_seconds             ///
                  matlab_wrapper_seconds                        ///
                  mata_matlab_max_abs mata_matlab_max_scaled     ///
                  rust_matlab_max_abs rust_matlab_max_scaled,    ///
    by(problem_type panel topology fe_dimensions rows workers firms ///
       bridge_workers requested_threads stata_threads              ///
       rust_threads_requested rust_threads_used tuning_mode)
generate double mata_over_matlab = mata_seconds / matlab_seconds
generate double rust_over_matlab = rust_seconds / matlab_seconds
label define fvmb_panel 1 "Two-way dense" 2 "Two-way sparse" ///
    3 "Two-way bottleneck"
label values panel fvmb_panel
sort panel requested_threads rows
save `"`output_directory'/manual_benchmark_summary.dta"', replace
export delimited using ///
    `"`output_directory'/manual_benchmark_summary.csv"', replace

local tuning_subtitle "Each estimator uses its default algorithm settings"
if `harmonize_settings' {
    local tuning_subtitle ///
        "Harmonized JLA: `probes' probes; 1e-10/1e-6 tolerances; maxiter 1000"
}

twoway (connected mata_seconds rows if                         ///
            requested_threads == `fixed_threads' & mata_seconds > 0, ///
            sort lcolor(midblue) mcolor(midblue) msymbol(circle))     ///
       (connected rust_seconds rows if                         ///
            requested_threads == `fixed_threads' & rust_seconds > 0, ///
            sort lcolor(forest_green) mcolor(forest_green)            ///
            msymbol(diamond) lpattern(dash))                           ///
       (connected matlab_seconds rows if                       ///
            requested_threads == `fixed_threads' & matlab_seconds > 0, ///
            sort lcolor(cranberry) mcolor(cranberry)                 ///
            msymbol(triangle)),                                      ///
    by(panel, cols(3) compact noiytitle noixtitle imargin(small)      ///
        title("Command time by dataset size at `fixed_threads' cores", ///
            size(medsmall))                                          ///
        subtitle("`tuning_subtitle'", size(small))                   ///
        b1title("Stored observations", size(small))                  ///
        l1title("Median estimator command seconds", size(small))    ///
        note("")                                                     ///
        graphregion(color(white) margin(small)))                      ///
    xscale(log) yscale(log) ylabel(, grid labsize(vsmall))            ///
    xlabel(, labsize(vsmall)) ytitle("") xtitle("")                 ///
    legend(order(1 "fevc (Mata)" 2 "fevc (Rust)"                   ///
        3 "maintained MATLAB") cols(1)                              ///
        position(3) ring(1) size(small) region(lstyle(none)))         ///
    xsize(14) ysize(5.5) name(__fvmb_size, replace)
graph export `"`output_directory'/manual_benchmark_time_by_size.png"', ///
    name(__fvmb_size) width(2600) replace

twoway (connected mata_seconds requested_threads if             ///
            rows == `medium_dataset_size' & mata_seconds > 0,   ///
            sort lcolor(midblue) mcolor(midblue) msymbol(circle))     ///
       (connected rust_seconds requested_threads if             ///
            rows == `medium_dataset_size' & rust_seconds > 0,   ///
            sort lcolor(forest_green) mcolor(forest_green)             ///
            msymbol(diamond) lpattern(dash))                            ///
       (connected matlab_seconds requested_threads if           ///
            rows == `medium_dataset_size' & matlab_seconds > 0, ///
            sort lcolor(cranberry) mcolor(cranberry)                 ///
            msymbol(triangle)),                                      ///
    by(panel, cols(3) compact noiytitle noixtitle imargin(small)      ///
        title("Command time by cores at N=`medium_dataset_size'",   ///
            size(medsmall))                                          ///
        subtitle("`tuning_subtitle'", size(small))                   ///
        b1title("Requested cores", size(small))                      ///
        l1title("Median estimator command seconds", size(small))    ///
        note("Only fevc Mata observations above the Stata processor cap are omitted; Rust uses the requested native threads.", ///
            size(vsmall))                                            ///
        graphregion(color(white) margin(small)))                      ///
    xlabel(`thread_counts', labsize(small)) yscale(log)               ///
    ylabel(, grid labsize(vsmall)) ytitle("") xtitle("")            ///
    legend(order(1 "fevc (Mata)" 2 "fevc (Rust)"                   ///
        3 "maintained MATLAB") cols(1)                              ///
        position(3) ring(1) size(small) region(lstyle(none)))         ///
    xsize(14) ysize(5.5) name(__fvmb_cores, replace)
graph export `"`output_directory'/manual_benchmark_time_by_cores.png"', ///
    name(__fvmb_cores) width(2600) replace

display as text _newline "Median command-time summary"
list problem_type rows requested_threads mata_seconds rust_seconds ///
    matlab_seconds mata_over_matlab rust_over_matlab, noobs abbreviate(24)

frame change __fvmb_runs
quietly count if mata_status == "fail" | rust_status == "fail" | ///
    matlab_status == "fail"
local failures = r(N)
quietly count if mata_status == "capped"
local capped = r(N)
quietly count
local total_pairs = r(N)

set processors `original_processors'
global FEVC_MANUAL_THREAD_CONTRACT ///
    `"`original_manual_thread_contract'"'
global FEVC_MANUAL_RUST_THREADS `"`original_manual_rust_threads'"'
global FEVC_MANUAL_STATA_THREADS `"`original_manual_stata_threads'"'
capture program drop __fvmb_generate
capture graph drop __fvmb_size __fvmb_cores

if `failures' {
    display as error ///
        "FEVC_MANUAL_BENCHMARK|FAIL|pairs=`total_pairs'|failed=`failures'|capped=`capped'|tuning=`tuning_mode'|output=`output_directory'"
    exit 459
}
display as result ///
    "FEVC_MANUAL_BENCHMARK|PASS|pairs=`total_pairs'|failed=0|capped=`capped'|tuning=`tuning_mode'|output=`output_directory'"
