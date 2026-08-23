version 18.0
clear all
set more off
set varabbrev off

adopath ++ "varcomp_kss"
do "varcomp_kss/tests/stata/test_load.do"
do "varcomp_kss/tests/stata/test_solver_prod_fixes.do"
do "varcomp_kss/tests/stata/test_full_rhs_certificate.do"
do "varcomp_kss/tests/stata/test_scale_engine_reductions.do"
display as result "VARCOMP_KSS MATA UNIT PROFILE PASS"
