version 18.0
clear all
set more off
set varabbrev off

adopath ++ "fevc"
do "fevc/tests/stata/test_perf_exact_terminal.do"
do "fevc/tests/stata/test_perf_batch_runtime.do"
display as result "FEVC SMALL BENCHMARK PROFILE PASS"
