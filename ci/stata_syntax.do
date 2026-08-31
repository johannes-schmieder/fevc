version 18.0
clear all
set more off
set varabbrev off

adopath ++ "fevc"
do "fevc/tests/stata/test_load.do"
display as result "FEVC SYNTAX PROFILE PASS"
