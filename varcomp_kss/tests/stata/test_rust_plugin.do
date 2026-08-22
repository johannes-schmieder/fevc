version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'
di as txt "RUST_PLUGIN_PLATFORM os=`c(os)' machine=`c(machine_type)'"

varcomp_kss_rust clear
varcomp_kss_rust probe
assert r(abi_compiled) == 1
assert r(abi_runtime) == 1
assert r(core_ready_flags) == 255
assert r(support_flags) == 38
assert r(deterministic_parallelism) == 1
varcomp_kss_rust selftest
if "$VCKSS_RUST_PLUGIN_HANDLE" != "" {
    di as error "Rust developer wrapper leaked a global plugin sentinel"
    exit 9
}

set obs 4
forvalues variable = 1/6 {
    generate double empty_input`variable' = `variable'
}
capture noisily varcomp_kss_rust prepare empty_input1 empty_input2       ///
    empty_input3 empty_input4 empty_input5 empty_input6 if 0,            ///
    cleanup generate(rust_keep_failed)
assert _rc == 198
capture confirm variable rust_keep_failed
assert _rc == 111
varcomp_kss_rust snapshot
assert r(state) == 0

clear
set obs 60
generate double worker = cond(_n <= 48, floor((_n - 1) / 4) + 1,       ///
    101 + floor((_n - 49) / 3))
generate double firm = cond(_n <= 48, mod(_n - 1, 4) + 1,               ///
    101 + mod(_n - 49, 3))
generate double deletion = _n
generate double outcome = worker - firm
generate double frequency = 1
generate double target_weight = 1
generate long original_order = _n
generate long permutation_key = mod(37 * _n, 61)
sort permutation_key
varcomp_kss_rust prepare worker firm deletion outcome frequency          ///
    target_weight, cleanup generate(rust_keep_disconnected)
local disconnected_handle = r(handle)
assert r(input_rows) == 60
assert r(retained_rows) == 48
assert r(memory_limit_bytes) == 4294967296
assert r(graph_degree_workers_removed) >= 0
assert r(graph_artic_workers_removed) >= 0
assert rust_keep_disconnected == (worker <= 12)
varcomp_kss_rust release `disconnected_handle'
varcomp_kss_rust snapshot
assert r(state) == 0

clear
set obs 48
generate double worker = floor((_n - 1) / 4) + 1
generate double firm = mod(_n - 1, 4) + 1
generate double deletion = _n
generate double outcome = cond(mod(worker + firm, 2) == 0, 1, -1) * ///
    (2 * (worker - 1) + firm)
generate double frequency = mod(firm - 1, 2) + 1
generate double target_weight = mod(worker - 1, 3) + firm

varcomp_kss_rust prepare worker firm deletion outcome frequency             ///
    target_weight, cleanup generate(rust_keep) memorygib(1)
local handle = r(handle)
assert `handle' > 0
assert r(input_rows) == 48
assert r(retained_rows) == 48
assert r(memory_limit_bytes) == 1073741824
assert r(controls_count) == 0
assert r(deletion_mode_code) == 1
assert rust_keep == 1

varcomp_kss_rust snapshot
assert r(state) == 1
assert r(handle) == `handle'

varcomp_kss_rust solve `handle', seed(91827) probes(6)                     ///
    leveragebatch(3) targetbatch(2) route(exact)
varcomp_kss_rust snapshot
assert r(state) == 3

varcomp_kss_rust result `handle'
matrix result = r(result)
assert rowsof(result) == 4
assert colsof(result) == 4
forvalues column = 1/4 {
    assert abs(result[1,`column'] - result[2,`column'] -                  ///
        result[3,`column']) <= 1e-10
    assert result[4,`column'] >= 0
    assert result[4,`column'] < .
}
assert r(seed) == 91827
assert r(probes) == 6
assert r(requested_route) == 1
assert r(selected_route) == 1
assert r(solver_fallback) == 0
assert r(leverage_probes_accepted) == 6
assert r(target_probes_accepted) == 6
assert r(full_fit_complete_residual) <= 1e-9
assert r(max_complete_residual) <= 1e-9
assert r(accounting_residual) <= 1e-10
assert r(exact_diagnostic_flags) == 256
assert r(working_fit_complete_residual) == 0
assert r(inverse_sqrt_relative_residual) == 0
assert r(maker_relative_residual) == 0
assert r(actual_accounting_residual) <= 1e-10

varcomp_kss_rust release `handle'
varcomp_kss_rust release `handle'
varcomp_kss_rust snapshot
assert r(state) == 0

di as result "VARCOMP_KSS RUST PLUGIN PASS"
exit 0
