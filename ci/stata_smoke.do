version 18
clear all
set more off

display as result "STATA_CI_OK"
display as text "stata_version=" c(stata_version)
display as text "stata_os=" c(os)
display as text "stata_machine_type=" c(machine_type)

quietly set obs 4
generate double x = _n
generate double y = 2 * x + 1
quietly summarize y
assert r(N) == 4
assert abs(r(mean) - 6) < 1e-12
