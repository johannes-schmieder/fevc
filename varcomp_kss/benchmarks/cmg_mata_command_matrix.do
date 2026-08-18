version 18.0
clear all
set more off
set varabbrev off

args repository_root output_csv firms_arg degree_first_arg degree_last_arg ///
    probes_arg processors_arg

local firms = real("`firms_arg'")
local degree_first = real("`degree_first_arg'")
local degree_last = real("`degree_last_arg'")
local probes = real("`probes_arg'")
local requested_processors = real("`processors_arg'")
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" | ///
    missing(`firms') | `firms' < 32 | `firms' != floor(`firms') |          ///
    missing(`degree_first') | missing(`degree_last') |                     ///
    !inrange(`degree_first',2,7) | !inrange(`degree_last',2,7) |            ///
    `degree_first' > `degree_last' | missing(`probes') | `probes' < 2 |     ///
    `probes' != floor(`probes') | missing(`requested_processors') |        ///
    `requested_processors' < 1 |                                          ///
    `requested_processors' != floor(`requested_processors') {
    di as error "usage: do cmg_mata_command_matrix.do root output firms " ///
        "degree_first degree_last probes processors"
    exit 198
}

capture set processors `requested_processors'
if c(MP) != 1 | c(processors) != `requested_processors' exit 459
adopath ++ `"`repository_root'/varcomp_kss"'
local workers = 40*`firms'

tempname handle
postfile `handle' byte degree double firms workers stored_rows physical_rows ///
    requested_probes selected_batch command_seconds setup_seconds            ///
    schur_seconds preconditioner_seconds pcg_seconds solver_iterations       ///
    solver_max_residual hierarchy_levels hybrid_vertices hybrid_edges        ///
    terminal_vertices str40 status str16 engine str16 preconditioner          ///
    str120 routing_reason using `"`output_csv'.dta"', replace

forvalues degree = `degree_first'/`degree_last' {
    clear
    quietly set obs `=`workers'*`degree''
    generate long deletion_unit = _n
    generate long worker = floor((_n-1)/`degree')+1
    generate byte slot = mod(_n-1,`degree')+1
    generate byte layer = floor((worker-1)/`firms')
    generate long base_firm = mod(worker-1,`firms')
    generate long firm_offset = 0
    local offset_band = floor(`firms'/4)
    quietly replace firm_offset = 1+mod(layer,`offset_band'-1)     ///
        if slot == 2
    quietly replace firm_offset = ceil(`firms'/3)+                 ///
        mod(97*layer,`offset_band') if slot == 3
    quietly replace firm_offset = ceil(2*`firms'/3)+               ///
        mod(193*layer,`offset_band') if slot == 4
    quietly replace firm_offset = `firms'-1 if slot == 5
    quietly replace firm_offset = `firms'-2 if slot == 6
    quietly replace firm_offset = ceil(2*`firms'/3)-1 if slot == 7
    assert inrange(firm_offset,0,`firms'-1)
    generate long firm = mod(base_firm+firm_offset,`firms')+1
    bysort worker firm: assert _N == 1
    generate byte frequency = 8
    generate double target_weight = 1+mod(deletion_unit,17)/17
    generate double outcome = sin(worker/97)+cos(firm/31)+          ///
        slot/101+sin(deletion_unit/113)
    quietly count
    local stored_rows = r(N)
    local physical_rows = 8*`stored_rows'

    quietly timer clear 99
    quietly timer on 99
    capture noisily varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
        deletion(match) deletionid(deletion_unit)                          ///
        targetweight(target_weight) algorithm(jla) engine(compressed)      ///
        preconditioner(cmg) probes(`probes') batch(8) memory_gib(8)        ///
        seed(20260818) tolerance(1e-10) maxiter(20000) wallseconds(3600)   ///
        nodisplay
    local command_rc = _rc
    quietly timer off 99
    quietly timer list 99
    local command_seconds = r(t99)
    quietly timer clear 99
    local status `"`e(status)'"'
    local engine `"`e(engine_selected)'"'
    local preconditioner `"`e(preconditioner_selected)'"'
    local routing_reason `"`e(routing_reason)'"'
    foreach name in batch setup_seconds schur_seconds                   ///
        preconditioner_apply_seconds pcg_seconds solver_iterations      ///
        solver_max_residual route_hierarchy_levels route_hybrid_vertices ///
        route_hybrid_edges route_terminal_vertices {
        local `name' = .
        capture local `name' = e(`name')
    }
    post `handle' (`degree') (`firms') (`workers') (`stored_rows')      ///
        (`physical_rows') (`probes') (`batch') (`command_seconds')      ///
        (`setup_seconds') (`schur_seconds')                             ///
        (`preconditioner_apply_seconds') (`pcg_seconds')                ///
        (`solver_iterations') (`solver_max_residual')                   ///
        (`route_hierarchy_levels') (`route_hybrid_vertices')            ///
        (`route_hybrid_edges') (`route_terminal_vertices')              ///
        (`"`status'"') (`"`engine'"') (`"`preconditioner'"')          ///
        (`"`routing_reason'"')
    if `command_rc' != 0 {
        postclose `handle'
        di as error "degree `degree' KSS command failed with rc=`command_rc'"
        exit `command_rc'
    }
    assert `"`status'"' == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
    assert `"`engine'"' == "compressed"
    assert `"`preconditioner'"' == "CMG"
    assert `solver_max_residual' <= 1e-9
    di as text "degree=" `degree' " command=" %9.3f `command_seconds' ///
        "s setup=" %9.3f `setup_seconds' "s maxres="                 ///
        %10.3e `solver_max_residual'
}
postclose `handle'

use `"`output_csv'.dta"', clear
export delimited using `"`output_csv'"', replace
erase `"`output_csv'.dta"'
assert status == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
assert engine == "compressed"
assert preconditioner == "CMG"
assert solver_max_residual <= 1e-9
di as result "CMG MATA COMMAND MATRIX PASS"
exit 0
