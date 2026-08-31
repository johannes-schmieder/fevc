version 18.0
clear all
set more off
set varabbrev off

adopath ++ "fevc"
do "fevc/tests/stata/test_load.do"
do "fevc/tests/stata/test_solver_prod_fixes.do"
do "fevc/tests/stata/test_full_rhs_certificate.do"
do "fevc/tests/stata/test_scale_engine_reductions.do"
display as result "FEVC MATA UNIT PROFILE PASS"
