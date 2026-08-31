version 15.0
clear all
set more off

capture log close _all
local capability_log : environment VCS_STATA_CAPABILITY_LOG
local capability_receipt : environment VCS_STATA_CAPABILITY_RECEIPT
local required_text : environment VCS_REQUIRED_STATA_PROCESSORS
log using `"`capability_log'"', text replace

local required = real("`required_text'")
local licensed = c(processors_lic)
local initial = c(processors)
local stata_version "`c(stata_version)'"
local flavor "`c(flavor)'"
local mp = c(MP)
local status "FAIL"
if !missing(`required') & `required' >= 1 & `required' == floor(`required') & ///
        `licensed' >= `required' {
    local status "PASS"
}

file open capability using `"`capability_receipt'"', write text replace
file write capability "key" _tab "value" _n
file write capability "schema" _tab "FEVC-STATA-PROCESSOR-CAPABILITY-V1" _n
file write capability "status" _tab "`status'" _n
file write capability "required_processors" _tab "`required'" _n
file write capability "licensed_processors" _tab "`licensed'" _n
file write capability "initial_processors" _tab "`initial'" _n
file write capability "stata_version" _tab "`stata_version'" _n
file write capability "stata_flavor" _tab "`flavor'" _n
file write capability "stata_mp" _tab "`mp'" _n
file close capability

display as text "VCKSS_STATA_PROCESSOR_CAPABILITY status=`status' required=`required' licensed=`licensed' initial=`initial'"
log close
exit, clear
