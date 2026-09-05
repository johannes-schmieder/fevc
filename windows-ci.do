version 18.0
clear all
set more off
set processors 2
set varabbrev off
assert `"`c(os)'"' == "Windows"
local rc_root `"`c(pwd)'"'
capture confirm file "windows-ci.status"
assert _rc == 601

// The guarded controller owns the one licensed Stata process. Build tools
// run as children, but every Stata assertion runs in this process.
shell powershell.exe -NoProfile -ExecutionPolicy Bypass -File "rust/stata_backend/build_windows_ci.ps1"
confirm file "windows-build.status"
tempname rc_status
file open `rc_status' using "windows-build.status", read text
file read `rc_status' rc_line
file close `rc_status'
assert `"`rc_line'"' == "FEVC_WINDOWS_BUILD=PASS"

// A fresh PLUS install must resolve the same staged plugin and public helpers.
local rc_plus `"`rc_root'/windows-plus"'
mkdir `"`rc_plus'"'
sysdir set PLUS `"`rc_plus'"'
quietly net install fevc, from(`"`rc_root'/windows-package"') replace
confirm file `"`rc_plus'/f/fevc_rust_windows_x64.plugin"'
do "fevc/tests/stata/test_rust_plugin.do" `"`rc_plus'/f"'
do "fevc/tests/stata/test_rust_match_component_inference.do" `"`rc_plus'/f"'
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
file open `rc_status' using `"`rc_root'/windows-ci.status"', write text
file write `rc_status' "WINDOWS_CI=PASS" _n
file close `rc_status'
display as result "FEVC WINDOWS MATCH RC SMOKE PASS"
