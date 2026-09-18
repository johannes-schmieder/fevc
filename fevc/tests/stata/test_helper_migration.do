version 18.0
clear all
set more off
set varabbrev off

args legacy_source package_source install_root
assert `"`legacy_source'"' != "" & `"`package_source'"' != "" & `"`install_root'"' != ""
sysdir set PLUS `"`install_root'/"'
net set ado `"`install_root'"'

local old_helpers : dir `"`legacy_source'"' files "_fevc*.ado"
local helper_count : word count `old_helpers'
assert `helper_count' > 0
quietly net install fevc, from(`"`legacy_source'"')
foreach old of local old_helpers {
    confirm file `"`install_root'/_/`old'"'
}

// Uninstall while the old package's complete file inventory is still registered.
ado uninstall fevc
discard
mata: mata clear
quietly net install fevc, from(`"`package_source'"')
foreach old of local old_helpers {
    capture confirm file `"`install_root'/_/`old'"'
    assert _rc == 601
    local renamed = "fevc_" + substr("`old'",6,.)
    confirm file `"`install_root'/f/`renamed'"'
}
local new_helpers : dir `"`install_root'/f"' files "fevc__*.ado"
local installed_count : word count `new_helpers'
assert `installed_count' == `helper_count'
findfile fevc.ado
assert strpos(`"`r(fn)'"',`"`install_root'/f/"') == 1

clear
input double(y worker firm)
1.0 1 1
1.1 1 1
1.2 1 2
1.3 1 2
1.4 2 1
1.5 2 1
1.6 2 2
1.7 2 2
end
foreach deletion in observation match {
    quietly fevc y, worker(worker) firm(firm) deletion(`deletion') ///
        algorithm(exact) backend(mata) nodisplay
    assert e(N) == 8
    assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
    quietly estat decomposition
}
di as result "FEVC HELPER MIGRATION PASS"
exit 0
