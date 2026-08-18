version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "varcomp_kss/varcomp_kss_lifecycle.ado"
if _rc {
    capture confirm file "../../varcomp_kss_lifecycle.ado"
    if _rc {
        di as error "run from the repository root or varcomp_kss/tests/stata"
        exit 601
    }
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/varcomp_kss"'
adopath ++ `"`pkgroot'"'
quietly do `"`pkgroot'/varcomp_kss_lifecycle.ado"'

// Installed lifecycle code must remain pure Stata/Mata.  Keep this broad:
// even a commented occurrence forces reviewers to inspect the source before
// weakening the assertion.
assert strpos(lower(fileread(`"`pkgroot'/varcomp_kss_lifecycle.ado"')), ///
    "shell") == 0

capture program drop _kss_test_scale_lifecycle_work
program define _kss_test_scale_lifecycle_work
    version 18.0
    syntax [, FAIL]
    assert _N == 0
    assert c(k) == 0
    mata: __kss_lifecycle_payload = J(5000, 1, 7)
    scalar __kss_lifecycle_callback_ran = 1
    if "`fail'" != "" error 459
end

// The public internal hook is inert unless the SCC wrapper explicitly sets
// KSS_PHASE_FILE.  Direct-path worker tests below exercise enabled evidence
// mode without changing this Stata process's environment.
local configured_phase_file : environment KSS_PHASE_FILE
if strtrim(`"`configured_phase_file'"') == "" {
    _vckss_lifecycle_phase, phase(import_selection)
    assert `"`r(status)'"' == "DISABLED"
}
else {
    _vckss_lifecycle_phase, phase(import_selection)
    assert `"`r(status)'"' == "WRITTEN"
    assert `"`r(phase)'"' == "import_selection"
    assert fileread(`"`configured_phase_file'"') == "import_selection"
    capture erase `"`configured_phase_file'"'
}

tempfile phase_marker
foreach registered_phase in import_selection compression_transition      ///
    numerical restoration {
    _vckss_lifecycle_phase_write, phase(`registered_phase')       ///
        phasefile(`"`phase_marker'"')
    assert `"`r(status)'"' == "WRITTEN"
    assert `"`r(phase)'"' == "`registered_phase'"
    assert fileread(`"`phase_marker'"') == "`registered_phase'"
}

capture noisily _vckss_lifecycle_phase_write, phase(target)      ///
    phasefile(`"`phase_marker'"')
assert _rc == 198
assert fileread(`"`phase_marker'"') == "restoration"

capture noisily _vckss_lifecycle_phase_write, phase(numerical)   ///
    phasefile("relative-phase.marker")
assert _rc == 198
capture noisily _vckss_lifecycle_phase_write, phase(numerical)   ///
    phasefile("/kss-phase-outside-temp.marker")
assert _rc == 198
local traversal_path `"`c(tmpdir)'/../kss-phase-escape.marker"'
capture noisily _vckss_lifecycle_phase_write, phase(numerical)   ///
    phasefile(`"`traversal_path'"')
assert _rc == 198

local missing_parent `"`c(tmpdir)'/kss-lifecycle-no-such-dir/phase.marker"'
capture noisily _vckss_lifecycle_phase_write, phase(numerical)   ///
    phasefile(`"`missing_parent'"')
assert _rc == 603

clear
input long(obsid worker firm) double(y) byte(sample)
4 2 2 1.4 0
2 1 2 1.2 1
5 3 1 1.5 1
1 1 1 1.1 1
3 2 1 1.3 0
6 3 2 1.6 1
end
label data "KSS lifecycle fixture"
label variable y "Outcome with lifecycle metadata"
label define worker_label 1 "one" 2 "two" 3 "three"
label values worker worker_label
format y %12.5f
char _dta[kss_lifecycle] "dataset characteristic"
char y[kss_lifecycle] "variable characteristic"
sort worker obsid

// Give the caller a real filename and then make an unsaved change.  Native
// preserve/restore must retain both pieces of state.
tempfile caller_source
quietly save `"`caller_source'"', replace
quietly replace y = y + .125 in 1
assert c(changed) == 1
local caller_filename `"`c(filename)'"'
local caller_filedate `"`c(filedate)'"'
local caller_sortedby : sortedby
local caller_data_label : data label
local caller_y_label : variable label y
local caller_y_format : format y
local caller_dta_char : char _dta[kss_lifecycle]
local caller_y_char : char y[kss_lifecycle]
quietly _datasignature
local caller_signature `"`r(datasignature)'"'
quietly _datasignature sample, nonames
local caller_sample_signature `"`r(datasignature)'"'
quietly count if sample
local caller_sample_N = r(N)

// Native preserve forced to Stata's disk implementation is the qualifying
// lifecycle: the raw data disappear while the callback runs and every
// inspected caller property returns afterward.
varcomp_kss_lifecycle, method(preserve) sample(sample)                 ///
    callback(_kss_test_scale_lifecycle_work) forcedisk certify
local preserve_method `"`r(method)'"'
local preserve_tmpdir `"`r(stata_tmpdir)'"'
scalar preserve_transition_seconds = r(transition_seconds)
scalar preserve_work_seconds = r(work_seconds)
scalar preserve_restore_seconds = r(restore_seconds)
scalar preserve_memory_before = r(mem_before_data_used_bytes)
scalar preserve_memory_cleared = r(mem_cleared_data_used_bytes)
scalar preserve_memory_restored = r(mem_restored_data_used_bytes)
assert "`preserve_method'" == "preserve"
assert strlen(`"`preserve_tmpdir'"') > 0
assert r(preserve_forced_disk) == 1
assert r(lifecycle_work_rc) == 0
assert r(sample_restored) == 1
assert r(data_restored) == 1
assert r(filename_restored) == 1
assert r(filedate_restored) == 1
assert r(changed_restored) == 1
assert preserve_transition_seconds >= 0
assert preserve_work_seconds >= 0
assert preserve_restore_seconds >= 0
assert preserve_memory_cleared < preserve_memory_before
assert preserve_memory_restored >= preserve_memory_cleared
assert scalar(__kss_lifecycle_callback_ran) == 1

quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
quietly _datasignature sample, nonames
assert `"`r(datasignature)'"' == `"`caller_sample_signature'"'
assert `"`c(filename)'"' == `"`caller_filename'"'
assert `"`c(filedate)'"' == `"`caller_filedate'"'
assert c(changed) == 1
local restored_sortedby : sortedby
local restored_data_label : data label
local restored_y_label : variable label y
local restored_y_format : format y
local restored_dta_char : char _dta[kss_lifecycle]
local restored_y_char : char y[kss_lifecycle]
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
assert `"`restored_data_label'"' == `"`caller_data_label'"'
assert `"`restored_y_label'"' == `"`caller_y_label'"'
assert `"`restored_y_format'"' == `"`caller_y_format'"'
assert `"`restored_dta_char'"' == `"`caller_dta_char'"'
assert `"`restored_y_char'"' == `"`caller_y_char'"'

// Callback failure is propagated only after native restore has completed.
capture noisily varcomp_kss_lifecycle, method(preserve) sample(sample) ///
    callback(_kss_test_scale_lifecycle_work) callbackoptions(fail) ///
    forcedisk certify
assert _rc == 459
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
assert `"`c(filename)'"' == `"`caller_filename'"'
assert c(changed) == 1
quietly count if sample
assert r(N) == `caller_sample_N'

// The tempfile alternative restores DTA contents, metadata, sort state,
// changed state, and the exact sample marker using save/clear/use.  As Stata
// documents, use changes c(filename); the prototype reports that fact so
// this route cannot silently claim exact caller-state restoration.
varcomp_kss_lifecycle, method(tempfile) sample(sample)                 ///
    callback(_kss_test_scale_lifecycle_work) certify
local tempfile_method `"`r(method)'"'
scalar tempfile_filename_restored = r(filename_restored)
scalar tempfile_changed_restored = r(changed_restored)
scalar tempfile_sample_restored = r(sample_restored)
scalar tempfile_data_restored = r(data_restored)
assert "`tempfile_method'" == "tempfile"
assert tempfile_filename_restored == 0
assert tempfile_changed_restored == 1
assert tempfile_sample_restored == 1
assert tempfile_data_restored == 1
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
local tempfile_sortedby : sortedby
local tempfile_data_label : data label
local tempfile_y_label : variable label y
local tempfile_y_format : format y
local tempfile_dta_char : char _dta[kss_lifecycle]
local tempfile_y_char : char y[kss_lifecycle]
assert `"`tempfile_sortedby'"' == `"`caller_sortedby'"'
assert `"`tempfile_data_label'"' == `"`caller_data_label'"'
assert `"`tempfile_y_label'"' == `"`caller_y_label'"'
assert `"`tempfile_y_format'"' == `"`caller_y_format'"'
assert `"`tempfile_dta_char'"' == `"`caller_dta_char'"'
assert `"`tempfile_y_char'"' == `"`caller_y_char'"'

capture noisily varcomp_kss_lifecycle, method(tempfile) sample(sample) ///
    callback(_kss_test_scale_lifecycle_work) callbackoptions(fail) ///
    certify
assert _rc == 459
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
assert c(changed) == 1
quietly count if sample
assert r(N) == `caller_sample_N'

// The final restored marker supports the ordinary ereturn-post e(sample)
// route.  ereturn post consumes its marker, so exercise this after every
// lifecycle test that still needs the variable itself.
tempname posted
matrix `posted' = J(1, 1, 0)
ereturn post `posted', esample(sample)
quietly count if e(sample)
assert r(N) == `caller_sample_N'
ereturn clear

capture mata: mata drop __kss_lifecycle_payload
capture scalar drop __kss_lifecycle_callback_ran
capture scalar drop preserve_transition_seconds
capture scalar drop preserve_work_seconds
capture scalar drop preserve_restore_seconds
capture scalar drop preserve_memory_before
capture scalar drop preserve_memory_cleared
capture scalar drop preserve_memory_restored
capture scalar drop tempfile_filename_restored
capture scalar drop tempfile_changed_restored
capture scalar drop tempfile_sample_restored
capture scalar drop tempfile_data_restored
capture program drop _kss_test_scale_lifecycle_work

quietly cd `"`oldpwd'"'
di as result "PASS test_scale_lifecycle.do"
exit 0
