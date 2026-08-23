version 18.0
clear all
set more off
set varabbrev off

adopath ++ "varcomp_kss"
do "varcomp_kss/tests/stata/test_load.do"
display as result "VARCOMP_KSS SYNTAX PROFILE PASS"
