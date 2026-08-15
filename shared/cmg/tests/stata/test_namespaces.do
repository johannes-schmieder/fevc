version 18.0
clear all
set more off

args repository_root
if strtrim(`"`repository_root'"') == "" exit 198

mata: mata clear
do `"`repository_root'/shared/cmg/generated/ppmltalo_cmg_core.mata"'
mata: assert(ppmltalo_cmg__api_level() == 4)

mata: mata clear
do `"`repository_root'/shared/cmg/generated/kssbc_cmg_core.mata"'
mata: assert(kssbc_cmg__api_level() == 4)

di as result "CMG NAMESPACE COMPILE TEST PASS"
exit 0
