version 18.0
clear all
set more off

args repository_root
if strtrim(`"`repository_root'"') == "" exit 198

mata: mata clear
do `"`repository_root'/fevc/vckss_cmg.mata"'
mata: assert(vckss_cmg__api_level() == 8)
mata: assert(vckss_cmg__numeric_mode() == "off")

mata: mata clear
do `"`repository_root'/fevc/cmg/generated/cmg_test.mata"'
mata: assert(cmgtest__api_level() == 8)
mata: assert(cmgtest__numeric_mode() == "on")

di as result "CMG NAMESPACE COMPILE TEST PASS"
exit 0
