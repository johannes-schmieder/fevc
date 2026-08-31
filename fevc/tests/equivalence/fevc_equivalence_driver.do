version 18.0

// One invocation runs exactly one case.  The Python orchestrator starts a
// fresh Stata process for every pre-rename/post-rename case pair.
args role case_id package_root output_path pass_path nonce command private_prefix

if !inlist("`role'", "baseline", "candidate") exit 198
if "`role'" == "baseline" {
    if "`command'" != "vckss" | "`private_prefix'" != "vckss" exit 198
}
else {
    if "`command'" != "fevc" | "`private_prefix'" != "vckss" exit 198
}
if strtrim("`case_id'") == "" | strtrim("`package_root'") == "" exit 198
if strtrim("`output_path'") == "" | strtrim("`pass_path'") == "" exit 198
if !regexm("`nonce'", "^[0-9a-f]+$") | strlen("`nonce'") != 48 exit 198

set more off
set varabbrev off
set linesize 255
capture log close _all
discard
mata: mata clear

capture mkdir "sysdirs"
capture mkdir "sysdirs/personal"
capture mkdir "sysdirs/plus"
capture mkdir "sysdirs/oldplace"
capture mkdir "sysdirs/site"
sysdir set PERSONAL "`c(pwd)'/sysdirs/personal"
sysdir set PLUS "`c(pwd)'/sysdirs/plus"
sysdir set OLDPLACE "`c(pwd)'/sysdirs/oldplace"
sysdir set SITE "`c(pwd)'/sysdirs/site"
adopath ++ "`package_root'"

quietly which `command'
quietly findfile `command'.ado
local resolved_ado `"`r(fn)'"'
if `"`resolved_ado'"' != `"`package_root'/`command'.ado"' exit 601

global VKEQ_ROLE `"`role'"'
global VKEQ_CASE `"`case_id'"'
global VKEQ_PACKAGE_ROOT `"`package_root'"'
global VKEQ_COMMAND `"`command'"'
global VKEQ_PREFIX `"`private_prefix'"'

quietly do `"`package_root'/`private_prefix'_rng.mata"'

capture program drop _vkeq_emit
program define _vkeq_emit
    version 18.0
    args case kind name row col value
    if strpos(`"`case'`kind'`name'`value'"', char(9)) exit 459
    if strpos(`"`case'`kind'`name'`value'"', char(10)) exit 459
    if strpos(`"`case'`kind'`name'`value'"', char(13)) exit 459
    file write vkeqout `"`case'"' _tab `"`kind'"' _tab       ///
        `"`name'"' _tab (`row') _tab (`col') _tab `"`value'"' _n
end

capture program drop _vkeq_emit_scalar_value
program define _vkeq_emit_scalar_value
    version 18.0
    args case kind name row col scalar_name
    local value_hex : display %21x scalar(`scalar_name')
    local value_hex = strtrim("`value_hex'")
    _vkeq_emit `"`case'"' `"`kind'"' `"`name'"' `row' `col' ///
        `"`value_hex'"'
end

capture program drop _vkeq_emit_matrix
program define _vkeq_emit_matrix
    version 18.0
    args case name
    capture matrix __vkeq_matrix = e(`name')
    if _rc {
        di as error "advertised e(`name') matrix could not be copied"
        exit 459
    }

    local rows = rowsof(__vkeq_matrix)
    local cols = colsof(__vkeq_matrix)
    local row_names : rownames __vkeq_matrix
    local col_names : colnames __vkeq_matrix
    local row_eq : roweq __vkeq_matrix
    local col_eq : coleq __vkeq_matrix
    _vkeq_emit `"`case'"' "matrix_meta" `"`name':rownames"' 0 0 ///
        `"`row_names'"'
    _vkeq_emit `"`case'"' "matrix_meta" `"`name':colnames"' 0 0 ///
        `"`col_names'"'
    _vkeq_emit `"`case'"' "matrix_meta" `"`name':roweq"' 0 0 ///
        `"`row_eq'"'
    _vkeq_emit `"`case'"' "matrix_meta" `"`name':coleq"' 0 0 ///
        `"`col_eq'"'
    scalar __vkeq_rows = `rows'
    scalar __vkeq_cols = `cols'
    _vkeq_emit_scalar_value `"`case'"' "matrix_meta"           ///
        `"`name':rows"' 0 0 __vkeq_rows
    _vkeq_emit_scalar_value `"`case'"' "matrix_meta"           ///
        `"`name':cols"' 0 0 __vkeq_cols

    forvalues row = 1/`rows' {
        forvalues col = 1/`cols' {
            scalar __vkeq_value = el(__vkeq_matrix, `row', `col')
            local is_timing =                                   ///
                ("`name'" == "prep_profile" &                  ///
                    inrange(`col', 1, 7)) |                     ///
                ("`name'" == "prep_boundary_profile" &         ///
                    inrange(`col', 1, 11)) |                    ///
                ("`name'" == "rhs_profile" &                   ///
                    inrange(`col', 1, 8)) |                     ///
                ("`name'" == "route_diagnostics" &             ///
                    inlist(`col', 3, 22)) |                      ///
                ("`name'" == "scale_receipt" &                 ///
                    inlist(`col', 10, 11))
            local is_runtime_footprint =                         ///
                ("`name'" == "resource_components" & `col' == 1) | ///
                ("`name'" == "resource_forecasts" &             ///
                    inlist(`col', 1, 2, 3, 4, 5, 7))
            if inlist("$VKEQ_CASE", "cmg_forced_cheap_1200x300", ///
                    "cmg_auto_cheap_1200x300") {                  ///
                local is_runtime_footprint = `is_runtime_footprint' | ///
                    ("`name'" == "resource_components" &         ///
                        `row' == 2 & `col' == 5) |                 ///
                    ("`name'" == "route_diagnostics" &           ///
                        `row' == 1 & `col' == 25)
            }
            if `is_timing' {
                if !missing(scalar(__vkeq_value)) &              ///
                        scalar(__vkeq_value) < 0 {
                    di as error "invalid registered matrix timing: `name'[`row',`col']"
                    exit 459
                }
                _vkeq_emit_scalar_value `"`case'"' "timing"       ///
                    `"`name'"' `row' `col' __vkeq_value
            }
            else if `is_runtime_footprint' {
                if scalar(__vkeq_value) >= . | scalar(__vkeq_value) < 0 {
                    di as error "invalid omitted runtime footprint: `name'[`row',`col']"
                    exit 459
                }
                _vkeq_emit_scalar_value `"`case'"' "matrix"       ///
                    `"`name'"' `row' `col' __vkeq_value
            }
            else {
                _vkeq_emit_scalar_value `"`case'"' "matrix"    ///
                    `"`name'"' `row' `col' __vkeq_value
            }
        }
    }
    capture matrix drop __vkeq_matrix
    capture scalar drop __vkeq_rows
    capture scalar drop __vkeq_cols
    capture scalar drop __vkeq_value
end

capture program drop _vkeq_state_before
program define _vkeq_state_before
    version 18.0
    quietly _datasignature
    global VKEQ_DATA_SIGNATURE `"`r(datasignature)'"'
    local sortedby : sortedby
    global VKEQ_SORTEDBY `"`sortedby'"'
    global VKEQ_FILENAME `"`c(filename)'"'
    global VKEQ_FILEDATE `"`c(filedate)'"'
    global VKEQ_CHANGED = c(changed)
    local data_label : data label
    local y_label : variable label y
    local worker_label : variable label worker
    local y_format : format y
    local worker_value_label : value label worker
    local worker_value_10 ""
    local worker_value_20 ""
    local worker_value_30 ""
    local worker_value_999 ""
    if `"`worker_value_label'"' != "" {
        local worker_value_10 : label `worker_value_label' 10
        local worker_value_20 : label `worker_value_label' 20
        local worker_value_30 : label `worker_value_label' 30
        local worker_value_999 : label `worker_value_label' 999
    }
    local dta_char : char _dta[kss_scale_command]
    local y_char : char y[kss_scale_command]
    global VKEQ_DATA_LABEL `"`data_label'"'
    global VKEQ_Y_LABEL `"`y_label'"'
    global VKEQ_WORKER_LABEL `"`worker_label'"'
    global VKEQ_Y_FORMAT `"`y_format'"'
    global VKEQ_WORKER_VALUE_LABEL `"`worker_value_label'"'
    global VKEQ_WORKER_VALUE_10 `"`worker_value_10'"'
    global VKEQ_WORKER_VALUE_20 `"`worker_value_20'"'
    global VKEQ_WORKER_VALUE_30 `"`worker_value_30'"'
    global VKEQ_WORKER_VALUE_999 `"`worker_value_999'"'
    global VKEQ_DTA_CHAR `"`dta_char'"'
    global VKEQ_Y_CHAR `"`y_char'"'
end

capture program drop _vkeq_state_after
program define _vkeq_state_after
    version 18.0
    args case
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"${VKEQ_DATA_SIGNATURE}"'
    local sortedby : sortedby
    assert `"`sortedby'"' == `"${VKEQ_SORTEDBY}"'
    assert `"`c(filename)'"' == `"${VKEQ_FILENAME}"'
    assert `"`c(filedate)'"' == `"${VKEQ_FILEDATE}"'
    assert c(changed) == ${VKEQ_CHANGED}
    local data_label : data label
    local y_label : variable label y
    local worker_label : variable label worker
    local y_format : format y
    local worker_value_label : value label worker
    local worker_value_10 ""
    local worker_value_20 ""
    local worker_value_30 ""
    local worker_value_999 ""
    if `"`worker_value_label'"' != "" {
        local worker_value_10 : label `worker_value_label' 10
        local worker_value_20 : label `worker_value_label' 20
        local worker_value_30 : label `worker_value_label' 30
        local worker_value_999 : label `worker_value_label' 999
    }
    local dta_char : char _dta[kss_scale_command]
    local y_char : char y[kss_scale_command]
    assert `"`data_label'"' == `"${VKEQ_DATA_LABEL}"'
    assert `"`y_label'"' == `"${VKEQ_Y_LABEL}"'
    assert `"`worker_label'"' == `"${VKEQ_WORKER_LABEL}"'
    assert `"`y_format'"' == `"${VKEQ_Y_FORMAT}"'
    assert `"`worker_value_label'"' == `"${VKEQ_WORKER_VALUE_LABEL}"'
    assert `"`worker_value_10'"' == `"${VKEQ_WORKER_VALUE_10}"'
    assert `"`worker_value_20'"' == `"${VKEQ_WORKER_VALUE_20}"'
    assert `"`worker_value_30'"' == `"${VKEQ_WORKER_VALUE_30}"'
    assert `"`worker_value_999'"' == `"${VKEQ_WORKER_VALUE_999}"'
    assert `"`dta_char'"' == `"${VKEQ_DTA_CHAR}"'
    assert `"`y_char'"' == `"${VKEQ_Y_CHAR}"'
    _vkeq_emit `"`case'"' "assertion" "data_restored" 0 0 "PASS"
    _vkeq_emit `"`case'"' "assertion" "sort_restored" 0 0 "PASS"
end

capture program drop _vkeq_rng_before
program define _vkeq_rng_before
    version 18.0
    set rng mt64s
    set rngstream 1
    set seed 1101
    quietly mata: runiform(7,1)
    set rngstream 2
    set seed 2202
    quietly mata: runiform(11,1)
    set rngstream 3
    set seed 3303
    quietly mata: runiform(13,1)
    // Stata 19 exposes kiss32 as the default alias after a command-level
    // save/restore.  Select the canonical spelling so algorithm restoration
    // is tested exactly rather than failing on the alias spelling alone.
    set rng default
    set seed 20260816
    set sortseed 20260818
    // Prime and then recapture all latent mt64s streams.  The second snapshot
    // is the actual caller state presented to the command.
    if "$VKEQ_ROLE" == "baseline"                              ///
        mata: VKEQ_RNG_PRIME = vckss_rng__capture_full()
    else mata: VKEQ_RNG_PRIME = vckss_rng__capture_full()
    if "$VKEQ_ROLE" == "baseline"                              ///
        mata: VKEQ_RNG_BEFORE = vckss_rng__capture_full()
    else mata: VKEQ_RNG_BEFORE = vckss_rng__capture_full()
    mata: assert(VKEQ_RNG_PRIME.status == "OK")
    mata: assert(VKEQ_RNG_BEFORE.status == "OK")
end

capture program drop _vkeq_rng_after
program define _vkeq_rng_after
    version 18.0
    args case
    if "$VKEQ_ROLE" == "baseline"                              ///
        mata: VKEQ_RNG_AFTER = vckss_rng__capture_full()
    else mata: VKEQ_RNG_AFTER = vckss_rng__capture_full()
    mata: st_numscalar("__vkeq_rng_ok",                          ///
        VKEQ_RNG_AFTER.status == "OK")
    mata: st_numscalar("__vkeq_rng_algorithm",                   ///
        VKEQ_RNG_AFTER.active.algorithm ==                        ///
            VKEQ_RNG_BEFORE.active.algorithm)
    mata: st_numscalar("__vkeq_rng_stream",                      ///
        VKEQ_RNG_AFTER.active.stream == VKEQ_RNG_BEFORE.active.stream)
    mata: st_numscalar("__vkeq_rng_state",                       ///
        VKEQ_RNG_AFTER.active.state == VKEQ_RNG_BEFORE.active.state)
    mata: st_numscalar("__vkeq_rng_sort_state",                  ///
        VKEQ_RNG_AFTER.sort_state == VKEQ_RNG_BEFORE.sort_state)
    mata: st_numscalar("__vkeq_rng_stream1",                     ///
        VKEQ_RNG_AFTER.mt64s_stream1_state ==                     ///
            VKEQ_RNG_BEFORE.mt64s_stream1_state)
    mata: st_numscalar("__vkeq_rng_stream2",                     ///
        VKEQ_RNG_AFTER.mt64s_stream2_state ==                     ///
            VKEQ_RNG_BEFORE.mt64s_stream2_state)
    mata: st_numscalar("__vkeq_rng_selected",                    ///
        VKEQ_RNG_AFTER.mt64s_selected_stream_state ==             ///
            VKEQ_RNG_BEFORE.mt64s_selected_stream_state)
    foreach check in ok algorithm stream state sort_state stream1 stream2 selected {
        if scalar(__vkeq_rng_`check') != 1 {
            di as error "RNG restoration failed: `check'"
            if "`check'" == "algorithm" {
                mata: st_local("__vkeq_before",                  ///
                    VKEQ_RNG_BEFORE.active.algorithm)
                mata: st_local("__vkeq_after",                   ///
                    VKEQ_RNG_AFTER.active.algorithm)
                di as error "before=`__vkeq_before' after=`__vkeq_after'"
            }
            exit 459
        }
        capture scalar drop __vkeq_rng_`check'
    }
    _vkeq_emit `"`case'"' "assertion" "rng_restored" 0 0 "PASS"
end

capture program drop _vkeq_emit_econtents
program define _vkeq_emit_econtents
    version 18.0
    args case success

    capture matrix list e(V)
    if !_rc exit 459
    _vkeq_emit `"`case'"' "assertion" "no_e_V" 0 0 "PASS"

    if "$VKEQ_ROLE" == "baseline" {
        capture mata: st_numscalar("__vkeq_cmg_api", vckss_cmg__api_level())
        if !_rc mata: st_local("__vkeq_cmg_design",               ///
            vckss_cmg__design_label())
    }
    else {
        capture mata: st_numscalar("__vkeq_cmg_api", vckss_cmg__api_level())
        if !_rc mata: st_local("__vkeq_cmg_design",               ///
            vckss_cmg__design_label())
    }
    if !_rc {
        _vkeq_emit_scalar_value `"`case'"' "metadata"             ///
            "cmg_api_level" 0 0 __vkeq_cmg_api
        _vkeq_emit `"`case'"' "metadata" "cmg_design_label" 0 0 ///
            `"`__vkeq_cmg_design'"'
        capture scalar drop __vkeq_cmg_api
    }

    local macro_names : e(macros)
    foreach name of local macro_names {
        local value `"`e(`name')'"'
        local kind "local"
        if inlist("`name'", "cmd", "cmdline", "version")          ///
            local kind "metadata"
        _vkeq_emit `"`case'"' `"`kind'"' `"`name'"' 0 0 `"`value'"'
    }

    local timing_scalars graph_seconds fit_seconds leverage_seconds        ///
        target_seconds correction_seconds setup_seconds                    ///
        preconditioner_seconds schur_seconds preconditioner_apply_seconds  ///
        pcg_seconds solver_backend_seconds compression_seconds             ///
        life_transition_seconds life_work_seconds life_restore_seconds     ///
        rng_seconds sample_selection_seconds validation_seconds
    local runtime_footprint_scalars life_mem_before_bytes                 ///
        life_mem_cleared_bytes life_mem_work_bytes life_mem_restored_bytes ///
        memory_forecast_bytes resource_mem_admit_bytes                    ///
        resource_numerical_peak_bytes resource_peak_bytes                 ///
        resource_raw_stata_bytes resource_restore_peak_bytes              ///
        resource_select_peak_bytes resource_transition_peak_bytes
    if inlist("$VKEQ_CASE", "cmg_forced_cheap_1200x300",          ///
            "cmg_auto_cheap_1200x300")                             ///
        local runtime_footprint_scalars `runtime_footprint_scalars' ///
            resource_cmg_hierarchy_bytes resource_routed_solver_bytes ///
            route_forecast_peak_bytes
    local scalar_names : e(scalars)
    foreach name of local scalar_names {
        scalar __vkeq_value = e(`name')
        local is_timing = strpos(" `timing_scalars' ", " `name' ") > 0
        local is_runtime_footprint = strpos(                       ///
            " `runtime_footprint_scalars' ", " `name' ") > 0
        if `is_timing' {
            local timing_not_applicable =                         ///
                "`name'" == "rng_seconds" & scalar(__vkeq_value) == .
            if (missing(scalar(__vkeq_value)) &                   ///
                    !`timing_not_applicable') |                   ///
                    scalar(__vkeq_value) < 0 {
                di as error "invalid registered timing scalar: `name'"
                exit 459
            }
            _vkeq_emit_scalar_value `"`case'"' "timing" `"`name'"' ///
                0 0 __vkeq_value
        }
        else if `is_runtime_footprint' {
            local footprint_not_applicable =                     ///
                inlist("`name'", "life_mem_cleared_bytes",      ///
                    "life_mem_work_bytes",                       ///
                    "life_mem_restored_bytes") &                 ///
                scalar(__vkeq_value) == .
            if (missing(scalar(__vkeq_value)) &                   ///
                    !`footprint_not_applicable') |                ///
                    scalar(__vkeq_value) < 0 {
                di as error "invalid omitted runtime footprint: `name'"
                exit 459
            }
            _vkeq_emit_scalar_value `"`case'"' "scalar" `"`name'"' ///
                0 0 __vkeq_value
        }
        else {
            _vkeq_emit_scalar_value `"`case'"' "scalar" `"`name'"' ///
                0 0 __vkeq_value
        }
    }
    _vkeq_emit `"`case'"' "assertion" "timing_values_valid" 0 0 "PASS"
    capture scalar drop __vkeq_value

    local matrix_names : e(matrices)
    foreach name of local matrix_names {
        if "`name'" == "V" exit 459
        _vkeq_emit_matrix `"`case'"' `"`name'"'
    }
    _vkeq_emit `"`case'"' "assertion"                            ///
        "matrix_timing_values_valid" 0 0 "PASS"

    local function_names : e(functions)
    _vkeq_emit `"`case'"' "e_functions" "names" 0 0            ///
        `"`function_names'"'
    local has_sample = strpos(" `function_names' ", " sample ") > 0

    if `success' {
        if !`has_sample' {
            di as error "successful command did not post e(sample)"
            exit 459
        }
        _vkeq_emit `"`case'"' "assertion"                       ///
            "sample_function_posted" 0 0 "PASS"
        tempvar sample
        quietly generate byte `sample' = e(sample)
        quietly count if `sample' != expected_sample
        assert r(N) == 0
        forvalues observation = 1/`=_N' {
            scalar __vkeq_sample = `sample'[`observation']
            local observation_id = obsid[`observation']
            _vkeq_emit_scalar_value `"`case'"' "sample" "e_sample" ///
                `observation_id' 0 __vkeq_sample
        }
        _vkeq_emit `"`case'"' "assertion" "sample_valid" 0 0 "PASS"
        capture scalar drop __vkeq_sample
    }
    else {
        if `has_sample' {
            di as error "failed command unexpectedly posted e(sample)"
            exit 459
        }
        _vkeq_emit `"`case'"' "assertion"                         ///
            "sample_function_absent" 0 0 "PASS"
    }
end

capture program drop _vkeq_finish
program define _vkeq_finish
    version 18.0
    args case fit_rc success
    _vkeq_rng_after `"`case'"'
    _vkeq_state_after `"`case'"'
    _vkeq_emit `"`case'"' "outcome" "rc" 0 0 `"`fit_rc'"'
    _vkeq_emit_econtents `"`case'"' `success'
end

capture program drop _vkeq_common_fixture
program define _vkeq_common_fixture
    version 18.0
    clear
    set obs 24
    generate long obsid = _n
    generate long worker = floor((_n-1)/4)
    generate long time = mod(_n-1,4)
    generate double c1 = time-1.5
    generate byte c2 = time==2
    generate long firm = .
    generate long match = .
    generate double noise = .
    local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
    local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
    local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
    forvalues row = 1/24 {
        replace firm = `: word `row' of `firms'' in `row'
        replace match = `: word `row' of `matches'' in `row'
        replace noise = `: word `row' of `noises'' in `row'
    }
    generate double y = 1.5+.3*worker-.2*firm+.4*c1-.15*c2+noise
    generate long frequency = cond(mod(_n-1,3)==0,2,1)
    generate double target = .5+(_n-1)/24
    generate byte expected_sample = 1
    sort firm worker obsid
end

capture program drop _vkeq_scale_fixture
program define _vkeq_scale_fixture
    version 18.0
    clear
    set obs 28
    generate long obsid = _n
    generate byte scope = _n <= 27
    generate byte cell = ceil(_n/3) if scope
    generate byte within_cell = mod(_n-1,3)+1 if scope
    generate byte worker_index = floor((cell-1)/3)+1 if scope
    generate byte firm_index = mod(cell-1,3)+1 if scope
    generate long worker = 10*worker_index if scope
    generate long firm = 100*firm_index if scope
    generate long match = 1000+cell if scope
    replace match = 101 in 1/2
    replace match = 102 in 3
    generate long frequency = 1+mod(obsid,3) if scope
    generate double target = frequency*(1+mod(obsid,3)/4) if scope
    generate double y = 2+.7*worker_index-.3*firm_index +          ///
        .11*within_cell+.03*worker_index*firm_index+.007*obsid^2 if scope
    generate double control = (within_cell-2)*(1+.1*worker_index) + ///
        .02*firm_index*within_cell if scope
    generate long atom_key = obsid
    generate byte expected_sample = scope
    replace worker = 999 in 28
    replace firm = 9999 in 28
    replace match = 99999 in 28
    replace frequency = 1 in 28
    replace target = 1 in 28
    replace y = -123 in 28
    replace control = 17 in 28
    replace atom_key = 999999 in 28
    label data "KSS compressed command lifecycle fixture"
    label variable y "Outcome with caller metadata"
    label variable worker "Worker identifier"
    label define worker_label 10 "worker one" 20 "worker two"   ///
        30 "worker three" 999 "out of scope"
    label values worker worker_label
    format y %13.6f
    char _dta[kss_scale_command] "caller dataset characteristic"
    char y[kss_scale_command] "caller variable characteristic"
    sort scope firm obsid
    tempfile caller_source
    quietly save `"`caller_source'"', replace
    quietly replace y = y+.125 in 1
    assert c(changed) == 1
end

capture program drop _vkeq_cmg_fixture
program define _vkeq_cmg_fixture
    version 18.0
    clear
    set obs 3600
    generate long obsid = _n
    generate long worker = floor((_n-1)/3)+1
    generate byte link = mod(_n-1,3)
    generate long firm = mod(worker-1+cond(link==2,17,link),300)+1
    generate double y = sin(worker/37)+cos(firm/19)+link/101
    generate byte expected_sample = 1
    sort firm worker obsid
end

capture program drop _vkeq_small_fixture
program define _vkeq_small_fixture
    version 18.0
    clear
    set obs 6
    generate double y = .
    generate long worker = cond(_n<=3,1,2)
    generate long firm = mod(_n-1,3)+1
    generate long observation_key = _n
    local outcomes 0 1 6 11 13 14
    forvalues row = 1/6 {
        replace y = `: word `row' of `outcomes'' in `row'
    }
    generate long obsid = _n
    generate byte expected_sample = 1
    sort firm worker obsid
end

capture program drop _vkeq_cross_failure_fixture
program define _vkeq_cross_failure_fixture
    version 18.0
    clear
    set obs 6
    generate double y = _n
    generate long worker = ceil(_n/2)
    generate long firm = mod(_n-1,2)+1
    generate long match = .
    local matches 10 11 10 21 30 31
    forvalues row = 1/6 {
        replace match = `: word `row' of `matches'' in `row'
    }
    generate long obsid = _n
    generate byte expected_sample = 0
    sort firm worker obsid
end

capture program drop _vkeq_eight_fixture
program define _vkeq_eight_fixture
    version 18.0
    clear
    set obs 8
    generate double y = .9+.1*_n
    generate long worker = cond(_n<=4,1,2)
    generate long firm = 1+mod(floor((_n-1)/2),2)
    generate long match = 10*worker+firm
    generate long frequency = 1
    generate double target = 1
    generate long obsid = _n
    generate byte expected_sample = 0
    sort firm worker obsid
end

file open vkeqout using `"`output_path'"', write replace text
file write vkeqout "case_id" _tab "kind" _tab "name" _tab      ///
    "row" _tab "col" _tab "value" _n

_vkeq_emit `"`case_id'"' "metadata" "role" 0 0 `"`role'"'
_vkeq_emit `"`case_id'"' "metadata" "command" 0 0 `"`command'"'
_vkeq_emit `"`case_id'"' "metadata" "private_prefix" 0 0       ///
    `"`private_prefix'"'
_vkeq_emit `"`case_id'"' "metadata" "package_root" 0 0         ///
    `"`package_root'"'
_vkeq_emit `"`case_id'"' "metadata" "resolved_ado" 0 0         ///
    `"`resolved_ado'"'
_vkeq_emit `"`case_id'"' "metadata" "stata_version" 0 0        ///
    `"`c(stata_version)'"'
_vkeq_emit `"`case_id'"' "metadata" "stata_flavor" 0 0         ///
    `"`c(flavor)'"'
_vkeq_emit `"`case_id'"' "metadata" "os" 0 0 `"`c(os)'"'
_vkeq_emit `"`case_id'"' "metadata" "processors" 0 0            ///
    `"`c(processors)'"'
_vkeq_emit `"`case_id'"' "metadata" "processors_lic" 0 0        ///
    `"`c(processors_lic)'"'

local executed = 0

if "`case_id'" == "exact_match_joint" {
    _vkeq_common_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y c1 c2, worker(worker) firm(firm) ///
        deletion(match) deletionid(match) algorithm(exact)          ///
        nuisance(joint) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "exact_match_fixed_fw_target" {
    _vkeq_common_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y c1 c2 [fw=frequency],          ///
        worker(worker) firm(firm) deletion(match) deletionid(match) ///
        targetweight(target) algorithm(exact) nuisance(fixedoffset) ///
        nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "exact_observation_joint_fw_target" {
    _vkeq_common_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y c1 c2 [fw=frequency],          ///
        worker(worker) firm(firm) deletion(observation)             ///
        targetweight(target) algorithm(exact) nuisance(joint)      ///
        nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "jla_match_generic_controls" {
    _vkeq_common_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y c1 c2 [fw=frequency],          ///
        worker(worker) firm(firm) deletion(match) deletionid(match) ///
        targetweight(target) algorithm(jla) nuisance(joint)        ///
        engine(generic) preconditioner(diagonal) probes(40)        ///
        batch(7) seed(20260818) tolerance(1e-12)                   ///
        backend(mata) rng(stata) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "jla_observation_generic_fixed" {
    _vkeq_common_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y c1 c2 [fw=frequency],          ///
        worker(worker) firm(firm) deletion(observation)             ///
        targetweight(target) algorithm(jla) nuisance(fixedoffset)  ///
        engine(generic) preconditioner(diagonal) probes(40)        ///
        batch(7) seed(20260819) tolerance(1e-12)                   ///
        backend(mata) rng(stata) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "compressed_match_fw_target" {
    _vkeq_scale_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y [fw=frequency] if scope,       ///
        worker(worker) firm(firm) deletion(match) deletionid(match) ///
        targetweight(target) probeorder(atom_key) algorithm(jla)   ///
        engine(compressed) preconditioner(diagonal) memory_gib(4)  ///
        wallseconds(3600) probes(40) batch(7) seed(8675309)        ///
        tolerance(1e-10) backend(mata) rng(stata) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    assert "`e(engine_selected)'" == "compressed"
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "rust_exact_match_joint" {
    _vkeq_common_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y c1 c2, worker(worker) firm(firm) ///
        deletion(match) deletionid(match) algorithm(exact)          ///
        nuisance(joint) backend(rust) rng(counter_v1) engine(auto)  ///
        nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    assert "`e(backend_selected)'" == "rust"
    assert "`e(rng_requested)'" == "counter_v1"
    assert "`e(rng_selected)'" == "NOT_APPLICABLE"
    assert "`e(algorithm)'" == "exact"
    assert e(probes) == 0
    assert e(seed) == 0
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "rust_compressed_match_fw_target" {
    _vkeq_scale_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y [fw=frequency] if scope,       ///
        worker(worker) firm(firm) deletion(match) deletionid(match) ///
        targetweight(target) algorithm(jla)                        ///
        engine(compressed) preconditioner(diagonal) memory_gib(4)  ///
        probes(40) batch(7) seed(8675309)                           ///
        tolerance(1e-10) backend(rust) rng(counter_v1) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    assert "`e(backend_selected)'" == "rust"
    assert "`e(rng_selected)'" == "counter_v1"
    assert "`e(engine_selected)'" == "compressed"
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "cmg_forced_cheap_1200x300" {
    _vkeq_cmg_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y, worker(worker) firm(firm)     ///
        deletion(match) algorithm(jla) engine(generic)             ///
        preconditioner(cmg) memory_gib(4) probes(8) batch(8)       ///
        seed(8675309) tolerance(1e-10) maxiter(10000)              ///
        backend(mata) rng(stata) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    assert "`e(preconditioner_selected)'" == "CMG"
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "cmg_auto_cheap_1200x300" {
    _vkeq_cmg_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y, worker(worker) firm(firm)     ///
        deletion(match) algorithm(jla) engine(generic)             ///
        preconditioner(auto) memory_gib(4) probes(8) batch(8)      ///
        seed(8675309) tolerance(1e-10) maxiter(10000)              ///
        backend(mata) rng(stata) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    assert "`e(preconditioner_selected)'" == "CMG"
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "auto_diagonal_small" {
    _vkeq_small_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y, worker(worker) firm(firm)     ///
        deletion(match) algorithm(jla) probeorder(observation_key) ///
        probes(40) engine(generic) batch(17) preconditioner(auto)  ///
        backend(mata) rng(stata) memory_gib(1) seed(8675309)       ///
        tolerance(1e-10) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 0
    assert "`e(preconditioner_selected)'" == "DIAGONAL"
    _vkeq_finish `"`case_id'"' `fit_rc' 1
    local executed = 1
}

if "`case_id'" == "failure_cross_coordinate_match" {
    _vkeq_cross_failure_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y, worker(worker) firm(firm)     ///
        deletion(match) deletionid(match) algorithm(exact) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 198
    assert "`e(withholding_status)'" == "CROSS_COORDINATE_MATCH"
    _vkeq_finish `"`case_id'"' `fit_rc' 0
    local executed = 1
}

if "`case_id'" == "failure_exact_size_limit" {
    _vkeq_eight_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y, worker(worker) firm(firm)     ///
        deletion(match) deletionid(match) algorithm(exact)         ///
        exact_limit(2) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 198
    assert "`e(withholding_status)'" == "EXACT_SIZE_LIMIT"
    _vkeq_finish `"`case_id'"' `fit_rc' 0
    local executed = 1
}

if "`case_id'" == "failure_singular_nuisance" {
    _vkeq_eight_fixture
    generate byte constant = 1
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y constant, worker(worker) firm(firm) ///
        deletion(observation) algorithm(jla) probes(5) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 498
    assert "`e(withholding_status)'" == "SINGULAR_NUISANCE_BLOCK"
    _vkeq_finish `"`case_id'"' `fit_rc' 0
    local executed = 1
}

if "`case_id'" == "failure_fastpath_controls" {
    _vkeq_scale_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y control [fw=frequency] if scope, ///
        worker(worker) firm(firm) deletion(match) deletionid(match) ///
        targetweight(target) probeorder(atom_key) algorithm(jla)    ///
        engine(compressed) probes(40) seed(8675309) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 498
    assert "`e(withholding_status)'" == "FASTPATH_CONTROLS"
    _vkeq_finish `"`case_id'"' `fit_rc' 0
    local executed = 1
}

if "`case_id'" == "failure_invalid_memory" {
    _vkeq_small_fixture
    ereturn clear
    _vkeq_state_before
    _vkeq_rng_before
    capture quietly $VKEQ_COMMAND y, worker(worker) firm(firm)     ///
        memory_gib(0) nodisplay
    local fit_rc = _rc
    assert `fit_rc' == 198
    assert "`e(withholding_status)'" == "INVALID_MEMORY_ENVELOPE"
    _vkeq_finish `"`case_id'"' `fit_rc' 0
    local executed = 1
}

assert `executed' == 1
file close vkeqout

local terminal_marker ///
    "VCKSS_RENAME_EQ_V2_PASS::`nonce'::`role'::`case_id'"
local pass_temporary `"`pass_path'.tmp"'
file open vkeqpass using `"`pass_temporary'"', write replace text
file write vkeqpass `"`terminal_marker'"' _n
file close vkeqpass
copy `"`pass_temporary'"' `"`pass_path'"', replace
erase `"`pass_temporary'"'
di as result `"`terminal_marker'"'
