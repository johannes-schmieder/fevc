version 18.0
clear all
set more off
set varabbrev off

args repository_root output_csv firms_arg degree_first_arg degree_last_arg ///
    algorithm core_path coarse_max_arg

local firms = real("`firms_arg'")
local degree_first = real("`degree_first_arg'")
local degree_last = real("`degree_last_arg'")
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" | ///
    missing(`firms') | `firms' < 32 | `firms' != floor(`firms') |          ///
    missing(`degree_first') | missing(`degree_last') |                     ///
    !inrange(`degree_first',2,7) | !inrange(`degree_last',2,7) |            ///
    `degree_first' > `degree_last' |                                       ///
    !inlist("`algorithm'","STEINER_MATA","API5_REFERENCE") {
    di as error "usage: do degree_hierarchy_matrix.do root output firms " ///
        "degree_first degree_last STEINER_MATA|API5_REFERENCE " ///
        "[core] [coarse_max]"
    exit 198
}
if strtrim(`"`core_path'"') == "" {
    local core_path `"`repository_root'/varcomp_kss/cmg/generated/cmg_test.mata"'
}

mata: mata clear
quietly do `"`core_path'"'

mata:
void cmgdegree__run(
    real scalar firms,
    real scalar degree,
    string scalar algorithm)
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__level scalar terminal
    real colvector cell, worker, slot, layer, base, offset, firm, weight
    real matrix timing
    real scalar workers, band, graph_seconds, setup_seconds
    real scalar fallback_levels, steiner_levels, minimum_reduction

    workers = 40*firms
    cell = (1::(workers*degree))
    worker = ceil(cell:/degree)
    slot = 1:+mod(cell:-1,degree)
    layer = floor((worker:-1):/firms)
    base = mod(worker:-1,firms)
    band = floor((firms-1)/(degree-1))
    offset = J(rows(cell),1,0)
    if (degree >= 2) {
        offset = (slot :> 1) :* (1:+(slot:-2):*band:+
            mod((89:+16:*slot):*layer,band))
    }
    assert(min(offset) == 0 & max(offset) <= firms-1)
    firm = 1:+mod(base:+offset,firms)
    weight = 1:+mod(cell,17):/17

    timer_clear(1)
    timer_on(1)
    cells = cmgtest__cells_prepare(
        worker,firm,weight,(1::workers),(1::firms))
    if (cells.status == "CONVERGED") graph = cmgtest__hybrid_build(cells)
    else graph = cmgtest__empty_graph()
    timer_off(1)
    timing = timer_value(1)
    graph_seconds = timing[1,1]

    options = cmgtest__options_resource(56*1024^3,
        max((graph.n_vertex,1)),61)
    if (strtoreal(st_local("coarse_max_arg")) < .) {
        options.coarse_max = strtoreal(st_local("coarse_max_arg"))
    }
    timer_clear(2)
    timer_on(2)
    if (graph.status == "CONVERGED") {
        hierarchy = cmgtest__hierarchy_mode(graph,options,algorithm)
    }
    else hierarchy = cmgtest__empty_hierarchy()
    timer_off(2)
    timing = timer_value(2)
    setup_seconds = timing[1,1]

    fallback_levels = sum(hierarchy.attempted_method :==
        "NORMALIZED_HEAVY_FALLBACK")
    steiner_levels = sum(hierarchy.attempted_method :==
        "GPL_STEINER_FOREST")
    minimum_reduction = .
    if (hierarchy.n_level > 1) {
        minimum_reduction = min(hierarchy.attempted_level_table[
            (1::(hierarchy.n_level-1)),6])
    }
    st_numscalar("cmgdegree_workers",workers)
    st_numscalar("cmgdegree_cells",rows(cell))
    st_numscalar("cmgdegree_vertices",graph.n_vertex)
    st_numscalar("cmgdegree_edges",graph.n_edge)
    st_numscalar("cmgdegree_graph_seconds",graph_seconds)
    st_numscalar("cmgdegree_setup_seconds",setup_seconds)
    st_numscalar("cmgdegree_levels",hierarchy.n_level)
    st_numscalar("cmgdegree_attempts",hierarchy.attempted_n_level)
    st_numscalar("cmgdegree_fallback",fallback_levels)
    st_numscalar("cmgdegree_steiner",steiner_levels)
    st_numscalar("cmgdegree_min_reduction",minimum_reduction)
    st_numscalar("cmgdegree_edge_complexity",hierarchy.edge_complexity)
    st_numscalar("cmgdegree_vertex_complexity",hierarchy.vertex_complexity)
    st_numscalar("cmgdegree_structural_bytes",hierarchy.structural_bytes)
    if (hierarchy.status == "CONVERGED") {
        terminal = *hierarchy.level[hierarchy.n_level]
        st_numscalar("cmgdegree_terminal",terminal.graph.n_vertex)
    }
    else st_numscalar("cmgdegree_terminal",.)
    st_local("cmgdegree_cells_status",cells.status)
    st_local("cmgdegree_graph_status",graph.status)
    st_local("cmgdegree_hierarchy_status",hierarchy.status)
    st_local("cmgdegree_hierarchy_message",hierarchy.message)
}
end

tempname handle
postfile `handle' str24 algorithm byte degree double firms workers cells ///
    vertices edges graph_seconds setup_seconds levels attempted_levels ///
    terminal_vertices fallback_levels steiner_levels minimum_reduction ///
    edge_complexity vertex_complexity structural_bytes                 ///
    str32 cells_status str32 graph_status str40 hierarchy_status       ///
    str120 hierarchy_message using `"`output_csv'.dta"', replace

forvalues degree = `degree_first'/`degree_last' {
    mata: cmgdegree__run(`firms',`degree',"`algorithm'")
    post `handle' ("`algorithm'") (`degree') (`firms')                 ///
        (scalar(cmgdegree_workers)) (scalar(cmgdegree_cells))          ///
        (scalar(cmgdegree_vertices)) (scalar(cmgdegree_edges))         ///
        (scalar(cmgdegree_graph_seconds))                              ///
        (scalar(cmgdegree_setup_seconds)) (scalar(cmgdegree_levels))   ///
        (scalar(cmgdegree_attempts)) (scalar(cmgdegree_terminal))      ///
        (scalar(cmgdegree_fallback)) (scalar(cmgdegree_steiner))       ///
        (scalar(cmgdegree_min_reduction))                              ///
        (scalar(cmgdegree_edge_complexity))                            ///
        (scalar(cmgdegree_vertex_complexity))                          ///
        (scalar(cmgdegree_structural_bytes))                           ///
        ("`cmgdegree_cells_status'") ("`cmgdegree_graph_status'")    ///
        ("`cmgdegree_hierarchy_status'")                              ///
        (`"`cmgdegree_hierarchy_message'"')
    di as text "degree=" `degree' " algorithm=`algorithm' graph="   ///
        %8.3f scalar(cmgdegree_graph_seconds) "s setup="              ///
        %9.3f scalar(cmgdegree_setup_seconds) "s status="            ///
        "`cmgdegree_hierarchy_status'"
}
postclose `handle'

use `"`output_csv'.dta"', clear
export delimited using `"`output_csv'"', replace
erase `"`output_csv'.dta"'
assert cells_status == "CONVERGED"
assert graph_status == "CONVERGED"
assert hierarchy_status == "CONVERGED"
assert inrange(minimum_reduction,.20,1) if levels > 1
assert edge_complexity <= 12
assert vertex_complexity <= 5
di as result "CMG DEGREE HIERARCHY MATRIX PASS"
exit 0
