version 18.0
clear all
set more off
set varabbrev off

args repository_root output_csv processors_arg core_path
local requested_processors = real("`processors_arg'")
if !inlist(`requested_processors', 4, 8) {
    di as error "hierarchy stress requires four or eight processors"
    exit 198
}
capture set processors `requested_processors'
if c(processors) != `requested_processors' | c(MP) != 1 {
    di as error "Stata/MP license lacks requested hierarchy-stress capacity"
    exit 459
}
do `"`repository_root'/fevc/cmg/benchmarks/hierarchy_scale_stress.do"' ///
    `"`repository_root'"' `"`output_csv'"' scc 65536 8 128 `"`core_path'"'
