version 18.0
clear all
set more off
set varabbrev off

args output_csv receipt_csv firms_arg
local firms = real("`firms_arg'")
if strtrim(`"`output_csv'"') == "" | strtrim(`"`receipt_csv'"') == "" | ///
    missing(`firms') | !inlist(`firms',64,256,1024) {
    di as error "invalid PREP-BND-1 input-generator arguments"
    exit 198
}

local workers = 40*`firms'
local degree = 3
local rows = `workers'*`degree'
set obs `rows'
generate long observation_key = _n
generate long match = _n
generate long worker = floor((_n-1)/`degree')+1
generate byte period = mod(_n-1,`degree')+1
generate long layer = floor((worker-1)/`firms')
generate long base_firm = mod(worker-1,`firms')
generate long offset = 0
replace offset = 1+mod(layer,floor(`firms'/4)-1) if period==2
replace offset = ceil(`firms'/3)+mod(97*layer,floor(`firms'/4)) if period==3
generate long firm = mod(base_firm+offset,`firms')+1
generate double y = mod(worker,257)/16 + mod(firm,127)/32 + ///
    period/64 + mod(match,13)/128
drop layer base_firm offset
isid observation_key
isid worker firm
assert _N == `rows'
quietly summarize worker, meanonly
assert r(min)==1 & r(max)==`workers'
quietly summarize firm, meanonly
assert r(min)==1 & r(max)==`firms'
sort worker period firm
format y %24.17g
order observation_key worker firm period match y
export delimited using `"`output_csv'"', replace

clear
set obs 1
generate str32 schema = "PREP-BND-1-INPUT-V1"
generate long rows = `rows'
generate long workers = `workers'
generate long firms = `firms'
generate byte cells_per_worker = `degree'
generate long coefficient_cells = `rows'
generate str24 sample_contract = "same_literal_rows_v1"
generate str24 target_contract = "uniform_stored_rows_v1"
export delimited using `"`receipt_csv'"', replace
di as result "PREP_BND1_MATLAB_INPUT_PASS F`firms' rows=`rows'"
exit 0
