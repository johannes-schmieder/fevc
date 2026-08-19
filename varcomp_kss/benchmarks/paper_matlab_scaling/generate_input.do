version 18.0
clear all
set more off
set varabbrev off

args output_csv receipt_csv structure connectivity rows_arg degree_arg
local rows = real("`rows_arg'")
local degree = real("`degree_arg'")
if strtrim(`"`output_csv'"') == "" | strtrim(`"`receipt_csv'"') == "" | ///
   !inlist(`rows',7680,30720,122880,491520,1966080) |              ///
   !inlist(`degree',2,3,6) |                                        ///
   !inlist("`structure'","strong_d2","strong_d3","strong_d6","weak_d3") {
    di as error "invalid paper MATLAB-scaling input-generator arguments"
    exit 198
}
if ("`structure'"=="strong_d2" & ("`connectivity'"!="strong" | `degree'!=2)) | ///
   ("`structure'"=="strong_d3" & ("`connectivity'"!="strong" | `degree'!=3)) | ///
   ("`structure'"=="strong_d6" & ("`connectivity'"!="strong" | `degree'!=6)) | ///
   ("`structure'"=="weak_d3"   & ("`connectivity'"!="weak"   | `degree'!=3)) {
    di as error "graph structure contract changed"
    exit 198
}

local workers = `rows'/`degree'
local firms = `workers'/40
assert `workers'==floor(`workers') & `firms'==floor(`firms')
set obs `rows'
generate long observation_key = _n
generate long match = _n
generate long worker = floor((_n-1)/`degree')+1
generate byte period = mod(_n-1,`degree')+1
generate long layer = floor((worker-1)/`firms')
generate long base_firm = mod(worker-1,`firms')
generate long offset = period-1
if "`connectivity'"=="strong" {
    replace offset = 0 if period==1
    replace offset = 1+mod(layer,floor(`firms'/4)-1) if period==2
    replace offset = ceil(`firms'/3)+mod(97*layer,floor(`firms'/4)) if period==3
    replace offset = ceil(2*`firms'/3)+mod(193*layer,floor(`firms'/4)) if period==4
    replace offset = `firms'-1 if period==5
    replace offset = ceil(2*`firms'/3)-1 if period==6
}
generate long firm = mod(base_firm+offset,`firms')+1
bysort worker firm: assert _N==1
generate double y = mod(worker,257)/16 + mod(firm,127)/32 + ///
    period/64 + mod(match,13)/128
drop layer base_firm offset
isid observation_key
isid worker firm
assert _N==`rows'
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
generate str40 schema = "PAPER-MATLAB-SCALING-INPUT-V1"
generate str16 structure = "`structure'"
generate str8 connectivity = "`connectivity'"
generate long rows = `rows'
generate long workers = `workers'
generate long firms = `firms'
generate byte cells_per_worker = `degree'
generate long coefficient_cells = `rows'
generate str32 sample_contract = "same_literal_match_rows_v1"
generate str32 target_contract = "uniform_stored_rows_v1"
export delimited using `"`receipt_csv'"', replace
di as result "PAPER_MATLAB_SCALING_INPUT_PASS `structure' rows=`rows'"
exit 0
