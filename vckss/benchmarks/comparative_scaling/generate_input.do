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
local weak_leaf_firms = cond("`structure'"=="weak_d3",`workers'/5,0)
local firms = cond("`structure'"=="weak_d3",`weak_leaf_firms'+6,`workers'/40)
assert `workers'==floor(`workers') & `firms'==floor(`firms')
set obs `rows'
generate long observation_key = _n
generate long match = _n
generate long worker = floor((_n-1)/`degree')+1
generate byte period = mod(_n-1,`degree')+1
generate long firm = .
local weak_hub_firms = 0
local weak_panel_layers = 0
local weak_hub_leaf_edges = 0
local weak_hub_tree_edges = 0
local weak_canonical_edges = 0
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
    // Five worker panels share one leaf firm and connect it to six hub firms.
    // The panel hub pairs form the tree (1,2), (1,3), (2,4), (3,5), (4,6).
    // Every worker therefore has two distinct hubs plus its panel leaf. Each
    // leaf sees every hub across its five workers, yielding six hub-leaf edges
    // per leaf and a first-level full contraction with no retained operators.
    assert `degree'==3 & mod(`workers',5)==0
    assert `weak_leaf_firms'==`workers'/5 & `firms'==`weak_leaf_firms'+6
    generate byte weak_panel = floor((worker-1)/`weak_leaf_firms')
    generate long weak_leaf = mod(worker-1,`weak_leaf_firms')+7
    generate byte weak_hub_a = .
    generate byte weak_hub_b = .
    replace weak_hub_a = 1 if weak_panel==0
    replace weak_hub_b = 2 if weak_panel==0
    replace weak_hub_a = 1 if weak_panel==1
    replace weak_hub_b = 3 if weak_panel==1
    replace weak_hub_a = 2 if weak_panel==2
    replace weak_hub_b = 4 if weak_panel==2
    replace weak_hub_a = 3 if weak_panel==3
    replace weak_hub_b = 5 if weak_panel==3
    replace weak_hub_a = 4 if weak_panel==4
    replace weak_hub_b = 6 if weak_panel==4
    assert inrange(weak_panel,0,4) & weak_hub_a<weak_hub_b
    replace firm = weak_hub_a if period==1
    replace firm = weak_hub_b if period==2
    replace firm = weak_leaf if period==3
    local weak_hub_firms = 6
    local weak_panel_layers = 5
    local weak_hub_leaf_edges = 6*`weak_leaf_firms'
    local weak_hub_tree_edges = 5
    local weak_canonical_edges = `weak_hub_leaf_edges'+`weak_hub_tree_edges'
    bysort firm: generate long weak_firm_incidences = _N
    assert weak_firm_incidences==5 if firm>6
    quietly count if firm<=6
    assert r(N)==2*`workers'
    drop weak_panel weak_leaf weak_hub_a weak_hub_b weak_firm_incidences
    local topology_contract "six_hub_leaf_panel_vector_v1"
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
generate str48 schema = "VCKSS-COMPARATIVE-SCALING-INPUT-V5"
generate str16 structure = "`structure'"
generate str8 connectivity = "`connectivity'"
generate str48 topology_contract = "`topology_contract'"
generate long rows = `rows'
generate long workers = `workers'
generate long firms = `firms'
generate byte cells_per_worker = `degree'
generate byte weak_hub_firms = `weak_hub_firms'
generate long weak_leaf_firms = `weak_leaf_firms'
generate byte weak_panel_layers = `weak_panel_layers'
generate long weak_hub_leaf_edges = `weak_hub_leaf_edges'
generate byte weak_hub_tree_edges = `weak_hub_tree_edges'
generate long weak_canonical_edges = `weak_canonical_edges'
generate long coefficient_cells = `rows'
generate str32 sample_contract = "same_literal_match_rows_v2"
generate str32 target_contract = "uniform_stored_rows_v1"
export delimited using `"`receipt_csv'"', replace
di as result "VCKSS_COMPARATIVE_SCALING_INPUT_PASS `structure' rows=`rows'"
exit 0
