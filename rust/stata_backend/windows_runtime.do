version 18.0
clear all
set more off
set processors 2
set varabbrev off
assert c(stata_version) == 19
assert c(MP) == 1
assert `"`c(os)'"' == "Windows"
local rc_root `"`c(pwd)'"'
capture confirm file "windows-ci.status"
assert _rc == 601
tempname rc_stage rc_status
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "build" _n
file close `rc_stage'
// The guarded controller owns one licensed process and the 900-second limit.
shell powershell.exe -NoProfile -ExecutionPolicy Bypass -File "rust/stata_backend/build_windows_ci.ps1"
confirm file "windows-build.status"
file open `rc_status' using "windows-build.status", read text
file read `rc_status' rc_line
file close `rc_status'
assert `"`rc_line'"' == "FEVC_WINDOWS_BUILD=PASS"
file open `rc_status' using "windows-runtime.profile", read text
file read `rc_status' rc_profile
file close `rc_status'
assert inlist(`"`rc_profile'"', "smoke", "full")
file open `rc_status' using "windows-qualification.group", read text
file read `rc_status' rc_group
file close `rc_status'
assert inlist("`rc_group'","smoke","legacy","cmg_point","cmg_attachments","cmg_lifecycle","performance","installation")
file open `rc_status' using "windows-completed-tests.txt", write text replace
file write `rc_status' "pe_import_export" _n
file close `rc_status'
local rc_plus `"`rc_root'/windows-plus"'
mkdir `"`rc_plus'"'
sysdir set PLUS `"`rc_plus'"'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "clean_install" _n
file close `rc_stage'
if "`rc_group'"=="installation" {
    quietly net install fevc, from(`"`rc_root'/windows-incumbent"') replace
    confirm file `"`rc_plus'/f/fevc_rust_windows_x64.plugin"'
}
quietly net install fevc, from(`"`rc_root'/windows-package"') replace
confirm file `"`rc_plus'/f/fevc_rust_windows_x64.plugin"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "clean_install" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "lifecycle" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_plugin.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "lifecycle" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "point_mean" _n
file close `rc_stage'
do "fevc/tests/stata/test_windows_runtime_smoke.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "point_mean" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "projection_mean" _n
file close `rc_stage'
do "fevc/tests/stata/test_projection_mean_native.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "projection_mean" _n
file close `rc_status'
quietly fevc_rust probe
assert r(component_centering_api) == 1
assert r(component_mixed_api) == 1
if `"`rc_profile'"' == "full" & "`rc_group'"=="legacy" {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "observation_component" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_component_inference.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "observation_component" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "individual_component" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_individual_inference.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "individual_component" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "match_component" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_match_component_inference.do" `"`rc_plus'/f"'
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' mata structured_common
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' mata structured_leverage
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' rust structured_common
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' rust structured_leverage
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "match_component" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "pooled_deletion" _n
file close `rc_stage'
do "fevc/tests/stata/test_pooled_deletion.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "pooled_deletion" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "mcse_modes" _n
file close `rc_stage'
do "fevc/tests/stata/test_mcse_modes.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "mcse_modes" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "mcse_attachments" _n
file close `rc_stage'
do "fevc/tests/stata/test_mcse_attachments.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "mcse_attachments" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "centering_mean" _n
file close `rc_stage'
do "fevc/tests/stata/test_centering_mean.do" `"`rc_plus'/f"' rust
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "centering_mean" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "centering_exact" _n
file close `rc_stage'
do "fevc/tests/stata/test_centering_exact.do" `"`rc_plus'/f"' rust
do "fevc/tests/stata/test_component_centering_exact.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "centering_exact" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "centering_jla" _n
file close `rc_stage'
do "fevc/tests/stata/test_centering_jla.do" `"`rc_plus'/f"' rust
do "fevc/tests/stata/test_component_centering_native.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "centering_jla" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "centering_options" _n
file close `rc_stage'
do "fevc/tests/stata/test_centering_options.do" `"`rc_plus'/f"' rust
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "centering_options" _n
file close `rc_status'
}
if inlist("`rc_group'","smoke","cmg_point","installation") {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_boundary" _n
file close `rc_stage'
do "fevc/tests/stata/test_windows_full_cmg_boundary.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_boundary" _n
file close `rc_status'
}
if "`rc_group'"=="cmg_point" {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_point_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_point_routes_full_cmg.do" `"`rc_plus'/f"' 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_point_1" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_point_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_point_routes_full_cmg.do" `"`rc_plus'/f"' 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_point_2" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_controlled_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_controlled_full_cmg.do" `"`rc_plus'/f"' 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_controlled_1" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_controlled_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_controlled_full_cmg.do" `"`rc_plus'/f"' 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_controlled_2" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_mcse_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_full_cmg_default_mcse.do" `"`rc_plus'/f"' 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_mcse_1" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_mcse_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_full_cmg_default_mcse.do" `"`rc_plus'/f"' 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_mcse_2" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_pooled" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_pooled_full_cmg.do" `"`rc_plus'/f"' "1 2"
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_pooled" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_v2" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_full_cmg_v2.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_v2" _n
file close `rc_status'
}
if "`rc_group'"=="cmg_attachments" {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_attachments_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_full_cmg_attachments.do" `"`rc_plus'/f"' 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_attachments_1" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_attachments_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_full_cmg_attachments.do" `"`rc_plus'/f"' 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_attachments_2" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_pooled_common_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' rust structured_common cmg 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_pooled_common_1" _n
file close `rc_status'

file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_pooled_common_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' rust structured_common cmg 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_pooled_common_2" _n
file close `rc_status'

file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_pooled_leverage_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' rust structured_leverage cmg 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_pooled_leverage_1" _n
file close `rc_status'

file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_pooled_leverage_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_pooled_component_inference.do" `"`rc_plus'/f"' rust structured_leverage cmg 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_pooled_leverage_2" _n
file close `rc_status'

}
if "`rc_group'"=="cmg_lifecycle" {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_lifecycle_1" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_full_cmg_lifecycle.do" `"`rc_plus'/f"' 1
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_lifecycle_1" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "cmg_lifecycle_2" _n
file close `rc_stage'
do "fevc/tests/stata/test_rust_full_cmg_lifecycle.do" `"`rc_plus'/f"' 2
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "cmg_lifecycle_2" _n
file close `rc_status'
}
if "`rc_group'"=="performance" {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "complete_command_measurements" _n
file close `rc_stage'
do "fevc/tests/stata/test_windows_cmg_performance.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "complete_command_measurements" _n
file close `rc_status'
}
if "`rc_group'"=="installation" {
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "replacement_install" _n
file close `rc_stage'
do "fevc/tests/stata/test_windows_runtime_smoke.do" `"`rc_plus'/f"'
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "replacement_install" _n
file close `rc_status'
}
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "registry_idle" _n
file close `rc_stage'
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
file open `rc_status' using "windows-completed-tests.txt", write text append
file write `rc_status' "registry_idle" _n
file close `rc_status'
file open `rc_stage' using "windows-ci.stage", write text replace
file write `rc_stage' "project_receipt" _n
file close `rc_stage'
shell powershell.exe -NoProfile -ExecutionPolicy Bypass -File "rust/stata_backend/write_windows_checks.ps1"
confirm file "windows-project-checks.json"
exit 0
