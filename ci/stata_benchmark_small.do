version 18.0
clear all
set more off
set varabbrev off

adopath ++ "varcomp_kss"
do "varcomp_kss/tests/stata/test_perf_exact_terminal.do"
do "varcomp_kss/tests/stata/test_perf_batch_runtime.do"
display as result "VARCOMP_KSS SMALL BENCHMARK PROFILE PASS"
