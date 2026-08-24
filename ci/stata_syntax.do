version 18.0
clear all
set more off
set varabbrev off

adopath ++ "vckss"
do "vckss/tests/stata/test_load.do"
display as result "VCKSS SYNTAX PROFILE PASS"
