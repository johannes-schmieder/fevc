version 18
clear all
set more off

display as result "STATA_CI_OK"
display as text "stata_version=" c(stata_version)
display as text "stata_edition=" c(edition_real)
display as text "stata_os=" c(os)
display as text "stata_machine_type=" c(machine_type)

assert c(stata_version) >= 18
