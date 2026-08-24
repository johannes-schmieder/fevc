version 18.0
clear all
set more off
set varabbrev off

adopath ++ "vckss"
do "vckss/tests/stata/test_load.do"
do "vckss/tests/stata/test_solver_prod_fixes.do"
do "vckss/tests/stata/test_full_rhs_certificate.do"
do "vckss/tests/stata/test_scale_engine_reductions.do"
display as result "VCKSS MATA UNIT PROFILE PASS"
