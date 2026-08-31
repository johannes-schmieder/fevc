version 18.0
clear all
set more off
set varabbrev off

args repository_root output_csv firms_arg degree_arg algorithm core_path
local firms = real("`firms_arg'")
local degree = real("`degree_arg'")
if strtrim(`"`repository_root'"') == "" |                       ///
    strtrim(`"`output_csv'"') == "" | missing(`firms') |       ///
    `firms' < 32 | `firms' != floor(`firms') |                  ///
    !inrange(`degree',2,7) |                                    ///
    !inlist("`algorithm'","STEINER_MATA","API5_REFERENCE") {
    di as error "usage: do hierarchy_level_profile.do root output firms " ///
        "degree STEINER_MATA|API5_REFERENCE [core]"
    exit 198
}
if strtrim(`"`core_path'"') == "" {
    local core_path `"`repository_root'/fevc/cmg/generated/cmg_test.mata"'
}

mata: mata clear
quietly do `"`core_path'"'

mata:
void cmglevelprofile__run(
    real scalar firms,
    real scalar degree,
    string scalar algorithm)
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    real colvector cell, worker, slot, layer, base, offset, firm, weight
    real colvector method
    real scalar workers, band, row

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
    firm = 1:+mod(base:+offset,firms)
    weight = 1:+mod(cell,17):/17
    cells = cmgtest__cells_prepare(
        worker,firm,weight,(1::workers),(1::firms))
    graph = cmgtest__hybrid_build(cells)
    options = cmgtest__options_resource(56*1024^3,
        max((graph.n_vertex,1)),61)
    hierarchy = cmgtest__hierarchy_mode(graph,options,algorithm)
    method = J(hierarchy.attempted_n_level,1,0)
    for (row=1; row<=hierarchy.attempted_n_level; row++) {
        if (hierarchy.attempted_method[row] == "GPL_STEINER_FOREST") {
            method[row] = 1
        }
        else if (hierarchy.attempted_method[row] == "SCREENED_FOREST") {
            method[row] = 2
        }
        else if (hierarchy.attempted_method[row] ==
                 "NORMALIZED_HEAVY_FALLBACK") method[row] = 3
        else if (hierarchy.attempted_method[row] == "DENSE_TERMINAL") {
            method[row] = 4
        }
    }
    st_matrix("cmglevelprofile_table",
        (hierarchy.attempted_level_table,method))
    st_local("cmglevelprofile_status",hierarchy.status)
}
end

mata: cmglevelprofile__run(`firms',`degree',"`algorithm'")
if "`cmglevelprofile_status'" != "CONVERGED" {
    di as error "hierarchy level profile failed: `cmglevelprofile_status'"
    exit 498
}

clear
svmat double cmglevelprofile_table, names(cmg)
rename cmg1 attempt
rename cmg2 vertices
rename cmg3 edges
rename cmg4 components
rename cmg5 coarse_vertices
rename cmg6 reduction
rename cmg7 edge_complexity
rename cmg8 vertex_complexity
rename cmg9 terminal
rename cmg10 method_code
generate str32 method = cond(method_code==1,"GPL_STEINER_FOREST",       ///
    cond(method_code==2,"SCREENED_FOREST",                             ///
    cond(method_code==3,"NORMALIZED_HEAVY_FALLBACK",                   ///
    cond(method_code==4,"DENSE_TERMINAL","NONE"))))
drop method_code
generate double firms = `firms'
generate byte degree = `degree'
generate str24 algorithm = "`algorithm'"
order algorithm firms degree attempt vertices edges components             ///
    coarse_vertices reduction edge_complexity vertex_complexity terminal method
export delimited using `"`output_csv'"', replace
di as result "CMG HIERARCHY LEVEL PROFILE PASS"
exit 0
