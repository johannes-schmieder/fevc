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
    di as error "invalid comparative-scaling input-generator arguments"
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
local firms = cond("`structure'"=="weak_d3",`workers'+1,`workers'/40)
assert `workers'==floor(`workers') & `firms'==floor(`firms')
set obs `rows'
generate long observation_key = _n
generate long match = _n
generate long worker = floor((_n-1)/`degree')+1
generate byte period = mod(_n-1,`degree')+1
generate long firm = .
local weak_hub_firms = 0
local weak_ring_firms = 0
local weak_hub_incidences = 0
local weak_ring_incidences = 0
local topology_contract "multi_offset_long_range_v1"
if "`connectivity'"=="strong" {
    generate long layer = floor((worker-1)/`firms')
    generate long base_firm = mod(worker-1,`firms')
    generate long offset = period-1
    replace offset = 0 if period==1
    replace offset = 1+mod(layer,floor(`firms'/4)-1) if period==2
    replace offset = ceil(`firms'/3)+mod(97*layer,floor(`firms'/4)) if period==3
    replace offset = ceil(2*`firms'/3)+mod(193*layer,floor(`firms'/4)) if period==4
    replace offset = `firms'-1 if period==5
    replace offset = ceil(2*`firms'/3)-1 if period==6
    replace firm = mod(base_firm+offset,`firms')+1
    drop layer base_firm offset
}
else {
    // One hub plus a ring of worker-specific firms gives each worker three
    // distinct incidences: hub, own ring firm, and next ring firm. The graph
    // is connected and leave-out redundant, is well conditioned at the frozen
    // largest row count, and remains operator-free while its canonical edge
    // count exceeds CMG's frozen connected-vector routing threshold.
    assert `degree'==3 & `firms'==`workers'+1
    replace firm = 1 if period==1
    replace firm = worker+1 if period==2
    replace firm = mod(worker,`workers')+2 if period==3
    local weak_hub_firms = 1
    local weak_ring_firms = `workers'
    local weak_hub_incidences = `workers'
    local weak_ring_incidences = 2*`workers'
    quietly count if firm==1
    assert r(N)==`weak_hub_incidences'
    bysort firm: generate long weak_firm_incidences = _N
    assert weak_firm_incidences==2 if firm>1
    drop weak_firm_incidences
    local topology_contract "hub_ring_leaveout_bottleneck_v1"
}
bysort worker firm: assert _N==1
generate double y = mod(worker,257)/16 + mod(firm,127)/32 + ///
    period/64 + mod(match,13)/128
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
generate str48 schema = "VCKSS-COMPARATIVE-SCALING-INPUT-V4"
generate str16 structure = "`structure'"
generate str8 connectivity = "`connectivity'"
generate str48 topology_contract = "`topology_contract'"
generate long rows = `rows'
generate long workers = `workers'
generate long firms = `firms'
generate byte cells_per_worker = `degree'
generate byte weak_hub_firms = `weak_hub_firms'
generate long weak_ring_firms = `weak_ring_firms'
generate long weak_hub_incidences = `weak_hub_incidences'
generate long weak_ring_incidences = `weak_ring_incidences'
generate long coefficient_cells = `rows'
generate str32 sample_contract = "same_literal_match_rows_v2"
generate str32 target_contract = "uniform_stored_rows_v1"
export delimited using `"`receipt_csv'"', replace
di as result "VCKSS_COMPARATIVE_SCALING_INPUT_PASS `structure' rows=`rows'"
exit 0
