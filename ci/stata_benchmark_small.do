version 18.0
clear all
set more off
set varabbrev off

adopath ++ "vckss"
do "vckss/tests/stata/test_perf_exact_terminal.do"
do "vckss/tests/stata/test_perf_batch_runtime.do"
display as result "VCKSS SMALL BENCHMARK PROFILE PASS"
