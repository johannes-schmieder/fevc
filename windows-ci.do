version 18.0
clear all
set more off

// Keep startup/license text on the machine. Only fixed stage names and the
// numeric Stata return code enter the project failure record.
tempname rc_stage
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "startup" _n
file close `rc_stage'
capture noisily do "rust/stata_backend/windows_runtime.do"
local rc_result = _rc
if `rc_result' {
    shell powershell.exe -NoProfile -ExecutionPolicy Bypass -File "rust/stata_backend/write_windows_failure.ps1" -ReturnCode `rc_result'
    exit `rc_result'
}

// PASS is written only after the complete selected profile and its receipt.
confirm file "windows-project-checks.json"
tempname rc_status
file open `rc_status' using "windows-ci.status", write text
file write `rc_status' "WINDOWS_CI=PASS" _n
file close `rc_status'
display as result "FEVC WINDOWS RUNTIME PROFILE PASS"
exit 0
