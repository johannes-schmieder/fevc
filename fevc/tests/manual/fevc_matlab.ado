program define fevc_matlab, rclass sortpreserve
    version 18.0
    syntax varname(numeric) [if] [in],                         ///
        WORKer(varname numeric) FIRM(varname numeric)          ///
        ORDER(varlist numeric min=1) MATLABRoot(string)        ///
        [ CONTROLS(varlist numeric) DELETION(string)           ///
          ALGorithm(string) PROBES(integer 200)                ///
          SEED(integer 8675309) THREADS(integer 1)             ///
          MATLAB(string) CMGCACHE(string) ]

    if strtrim(`"`matlabroot'"') == "" {
        display as error "matlabroot() must name the maintained LeaveOutTwoWay checkout"
        exit 198
    }
    if strtrim(`"`matlab'"') == "" local matlab "matlab"
    if strtrim(`"`deletion'"') == "" local deletion "match"
    if strtrim(`"`algorithm'"') == "" local algorithm "default"
    local deletion = lower(strtrim(`"`deletion'"'))
    local algorithm = lower(strtrim(`"`algorithm'"'))
    if !inlist("`deletion'", "match", "observation") {
        display as error "deletion() must be match or observation"
        exit 198
    }
    if !inlist("`algorithm'", "default", "exact", "jla") {
        display as error "algorithm() must be default, exact, or jla"
        exit 198
    }
    if `probes' < 2 | `probes' != floor(`probes') {
        display as error "probes() must be an integer of at least two"
        exit 198
    }
    if `seed' < 0 | `seed' != floor(`seed') {
        display as error "seed() must be a nonnegative integer"
        exit 198
    }
    if `threads' < 1 | `threads' != floor(`threads') {
        display as error "threads() must be a positive integer"
        exit 198
    }

    capture confirm file `"`matlabroot'/codes/leave_out_KSS.m"'
    if _rc {
        display as error ///
            `"matlabroot() does not contain codes/leave_out_KSS.m: `matlabroot'"'
        exit 601
    }
    capture findfile fevc_manual_matlab.m
    if _rc {
        display as error ///
            "fevc_manual_matlab.m is not on the ado-path; add fevc/tests/manual"
        exit 601
    }
    local bridge `"`r(fn)'"'

    if strtrim(`"`cmgcache'"') == "" {
        local cmgcache `"`c(tmpdir)'fevc-manual-cmg-cache"'
    }
    foreach path in bridge matlabroot cmgcache matlab {
        if strpos(`"``path''"', char(39)) | strpos(`"``path''"', char(34)) {
            display as error ///
                "MATLAB bridge paths and matlab() may not contain quote characters"
            exit 198
        }
    }

    marksample touse
    markout `touse' `worker' `firm' `order' `controls'
    quietly count if `touse'
    local input_rows = r(N)
    if `input_rows' < 1 {
        display as error "the requested MATLAB comparison sample is empty"
        exit 2000
    }

    tempfile input output driver log detail
    preserve
    quietly keep if `touse'
    quietly sort `worker' `order' `firm'
    quietly keep `varlist' `worker' `firm' `controls'
    quietly order `varlist' `worker' `firm' `controls'
    quietly export delimited using `"`input'"', replace
    restore

    local matlab_deletion = cond("`deletion'" == "match", "matches", "obs")
    local matlab_algorithm "`algorithm'"
    if "`algorithm'" == "jla" local matlab_algorithm "JLA"
    tempname driver_handle
    file open `driver_handle' using `"`driver'"', write replace text
    file write `driver_handle' ///
        `"addpath(fileparts('`bridge''));"' _n
    local matlab_call ///
        `"fevc_manual_matlab('`input'','`output'','`matlabroot'','`matlab_deletion'','`matlab_algorithm'',`probes',`seed',`threads','`cmgcache'','`detail'');"'
    file write `driver_handle' `"`matlab_call'"' _n
    file close `driver_handle'

    local started = clock(c(current_date) + " " + c(current_time), "DMY hms")
    local shell_command ///
        `""`matlab'" -batch "eval(fileread('`driver''))" > "`log'" 2>&1"'
    quietly shell `shell_command'
    local finished = clock(c(current_date) + " " + c(current_time), "DMY hms")
    local wrapper_seconds = (`finished' - `started') / 1000

    capture confirm file `"`output'"'
    if _rc {
        display as error "MATLAB did not produce a result; its captured output follows"
        capture noisily type `"`log'"'
        exit 499
    }

    preserve
    quietly import delimited using `"`output'"', clear varnames(1) ///
        bindquote(strict)
    capture assert _N == 1
    if _rc {
        restore
        display as error "MATLAB bridge returned a malformed result file"
        exit 499
    }
    foreach variable in corrected_worker corrected_firm              ///
        corrected_covariance corrected_total command_seconds         ///
        pool_startup_seconds pool_teardown_seconds mex_setup_seconds ///
        input_rows retained_units pool_workers mex_compiled          ///
        target_identity_scaled_error {
        capture confirm numeric variable `variable'
        if _rc {
            restore
            display as error ///
                "MATLAB bridge result is missing numeric field `variable'"
            exit 499
        }
    }
    foreach variable in schema matlab_version matlab_release core_file ///
        selected_algorithm {
        capture confirm string variable `variable'
        if _rc {
            restore
            display as error ///
                "MATLAB bridge result is missing text field `variable'"
            exit 499
        }
    }
    local r_worker = corrected_worker[1]
    local r_firm = corrected_firm[1]
    local r_covariance = corrected_covariance[1]
    local r_total = corrected_total[1]
    local r_command = command_seconds[1]
    local r_pool_start = pool_startup_seconds[1]
    local r_pool_stop = pool_teardown_seconds[1]
    local r_mex = mex_setup_seconds[1]
    local r_input = input_rows[1]
    local r_retained = retained_units[1]
    local r_workers = pool_workers[1]
    local r_compiled = mex_compiled[1]
    local r_identity = target_identity_scaled_error[1]
    local r_schema = schema[1]
    local r_version = matlab_version[1]
    local r_release = matlab_release[1]
    local r_core = core_file[1]
    local r_selected_algorithm = selected_algorithm[1]
    restore

    if `"`r_schema'"' != "FEVC-MANUAL-MATLAB-V1" {
        display as error "MATLAB bridge returned an unsupported result schema"
        exit 499
    }
    if `r_input' != `input_rows' {
        display as error "MATLAB bridge input-row receipt does not match Stata"
        exit 499
    }
    if `r_identity' > 1e-12 {
        display as error "MATLAB result failed the four-target accounting identity"
        exit 459
    }

    tempname kss
    matrix `kss' = (`r_worker', `r_firm', `r_covariance', `r_total')
    matrix colnames `kss' = worker firm covariance total
    return matrix kss = `kss'
    return scalar worker = `r_worker'
    return scalar firm = `r_firm'
    return scalar covariance = `r_covariance'
    return scalar total = `r_total'
    return scalar command_seconds = `r_command'
    return scalar wrapper_seconds = `wrapper_seconds'
    return scalar pool_startup_seconds = `r_pool_start'
    return scalar pool_teardown_seconds = `r_pool_stop'
    return scalar mex_setup_seconds = `r_mex'
    return scalar input_rows = `r_input'
    return scalar retained_units = `r_retained'
    return scalar pool_workers = `r_workers'
    return scalar mex_compiled = `r_compiled'
    return scalar target_identity_scaled_error = `r_identity'
    return local schema `"`r_schema'"'
    return local matlab_version `"`r_version'"'
    return local matlab_release `"`r_release'"'
    return local core_file `"`r_core'"'
    return local algorithm "`algorithm'"
    return local selected_algorithm `"`r_selected_algorithm'"'
    return local deletion "`deletion'"
end
