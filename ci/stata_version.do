version 15.0
clear all
set more off

display as result "STATA_CI_OK"
display as text "stata_version=" c(stata_version)
display as text "stata_edition=" cond(c(MP),"MP",cond(c(SE),"SE","IC"))
display as text "stata_os=" c(os)
display as text "stata_machine_type=" c(machine_type)

assert c(stata_version) >= 15
