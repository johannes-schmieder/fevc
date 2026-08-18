version 18.0
clear all
set more off
set varabbrev off

args experiment_id workers_arg firms_arg cells_per_worker_arg ///
    rows_per_cell_arg connectivity output_dta output_receipt

local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local cells_per_worker = real("`cells_per_worker_arg'")
local rows_per_cell = real("`rows_per_cell_arg'")
if !ustrregexm("`experiment_id'", "^[A-Za-z0-9._-]+$") |      ///
    missing(`workers') | `workers' < 40 |                        ///
    `workers' != floor(`workers') |                              ///
    missing(`firms') | `firms' < 8 | `firms' != floor(`firms') | ///
    `workers' != 40*`firms' |                                    ///
    !inrange(`cells_per_worker',2,4) |                            ///
    `cells_per_worker' != floor(`cells_per_worker') |            ///
    !inlist(`rows_per_cell',1,8) |                               ///
    !inlist("`connectivity'","strong","weak") {
    di as error "invalid KSS-NUMOPT-2 synthetic design"
    exit 198
}

local cells = `workers'*`cells_per_worker'
local rows = `cells'*`rows_per_cell'
local frequency_per_row = cond(`rows_per_cell'==1,8,1)
if `rows' > 2000000000 {
    di as error "synthetic row count exceeds registered long-ID bound"
    exit 198
}

set obs `rows'
generate long observation_key = _n
generate long deletion_unit = floor((_n-1)/`rows_per_cell')+1
generate byte replicate = mod(_n-1,`rows_per_cell')+1
generate byte cell_slot = mod(deletion_unit-1,`cells_per_worker')+1
generate long worker = floor((deletion_unit-1)/`cells_per_worker')+1
generate byte layer = floor((worker-1)/`firms')
generate long base_firm = mod(worker-1,`firms')
generate long firm_offset = cell_slot-1
if "`connectivity'" == "strong" {
    local offset_band = floor(`firms'/4)
    quietly replace firm_offset = 1+mod(layer,`offset_band'-1) ///
        if cell_slot == 2
    quietly replace firm_offset = ceil(`firms'/3)+             ///
        mod(97*layer,`offset_band') if cell_slot == 3
    quietly replace firm_offset = ceil(2*`firms'/3)+           ///
        mod(193*layer,`offset_band') if cell_slot == 4
}
assert inrange(firm_offset,0,`firms'-1)
generate long firm = mod(base_firm+firm_offset,`firms')+1
by worker firm, sort: assert _N == `rows_per_cell'
generate byte frequency = cond(`rows_per_cell'==1,8,1)
generate double target_weight =                              ///
    (1+mod(deletion_unit,17)/17)/`rows_per_cell'
generate double y_minus_xb = sin(worker/97)+cos(firm/31)+     ///
    .03*replicate+sin(deletion_unit/113)
sort observation_key
isid observation_key
assert worker[1] == 1 & worker[_N] == `workers'
quietly summarize firm, meanonly
assert r(min) == 1 & r(max) == `firms'
drop replicate cell_slot layer base_firm firm_offset
save `"`output_dta'"', replace

tempname receipt
file open `receipt' using `"`output_receipt'"', write text replace
file write `receipt' "key" _tab "value" _n
file write `receipt' "receipt_version" _tab "KSS-NUMOPT-2-INPUT-V1" _n
file write `receipt' "experiment_id" _tab "`experiment_id'" _n
file write `receipt' "connectivity" _tab "`connectivity'" _n
file write `receipt' "workers" _tab "`workers'" _n
file write `receipt' "firms" _tab "`firms'" _n
file write `receipt' "cells_per_worker" _tab "`cells_per_worker'" _n
file write `receipt' "rows_per_cell" _tab "`rows_per_cell'" _n
file write `receipt' "cells" _tab "`cells'" _n
file write `receipt' "rows" _tab "`rows'" _n
file write `receipt' "frequency_per_row" _tab ///
    "`frequency_per_row'" _n
file close `receipt'

di as result "KSS_NUMOPT2_INPUT_PASS `experiment_id'"
exit 0
