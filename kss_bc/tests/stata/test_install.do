version 18.0
clear all
set more off
set varabbrev off

args package_source install_root
if strtrim(`"`package_source'"') == "" | strtrim(`"`install_root'"') == "" {
    di as error "usage: do test_install.do package_source install_root"
    exit 198
}

sysdir set PLUS `"`install_root'/"'
net install kss_bc, from(`"`package_source'"') replace
discard
mata: mata clear

capture findfile kss_bc.ado
assert _rc == 0
local installed_ado `"`r(fn)'"'
assert strpos(`"`installed_ado'"',`"`install_root'"') == 1
capture findfile kss_bc.mata
assert _rc == 0
capture findfile kss_bc.sthlp
assert _rc == 0

capture noisily kss_bc, version
assert _rc == 0
assert "`e(version)'" == "0.1.0-dev"

clear
input double(y worker firm match)
1.0 1 1 11
1.1 1 1 11
1.2 1 2 12
1.3 1 2 12
1.4 2 1 21
1.5 2 1 21
1.6 2 2 22
1.7 2 2 22
end
kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert e(N) == 8

di as result "KSS_BC INSTALL TEST PASS"
exit 0
