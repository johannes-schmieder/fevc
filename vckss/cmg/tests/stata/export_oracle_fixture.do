version 18.0
clear all
set more off
set varabbrev off

args repository_root output_csv
if strtrim(`"`repository_root'"') == "" | strtrim(`"`output_csv'"') == "" {
    exit 198
}

mata: mata clear
do `"`repository_root'/vckss/cmg/generated/cmg_test.mata"'

mata:
void cmgtest__export_fixture()
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__apply_result scalar result
    real colvector worker, firm, weight
    real scalar vertices, edge

    vertices = 8
    worker = J(2*(vertices-1),1,.)
    firm = J(2*(vertices-1),1,.)
    weight = J(2*(vertices-1),1,1)
    for (edge=1; edge<vertices; edge++) {
        worker[2*edge-1] = edge
        worker[2*edge] = edge
        firm[2*edge-1] = edge
        firm[2*edge] = edge+1
    }
    cells = cmgtest__cells_prepare(worker,firm,weight,
        (101::107),(201::208))
    graph = cmgtest__hybrid_build(cells)
    options = cmgtest__options_default()
    options.coarse_max = 2
    hierarchy = cmgtest__hierarchy_build(graph,options)
    assert(hierarchy.status == "CONVERGED")
    result = cmgtest__apply_kss(hierarchy,I(vertices))
    assert(result.status == "CONVERGED")
    st_matrix("cmg_fixture",result.value)
}

cmgtest__export_fixture()
end

quietly svmat double cmg_fixture, names(cmg_fixture)
generate long row = _n
order row
export delimited using `"`output_csv'"', replace
di as result "CMG ORACLE FIXTURE EXPORT PASS"
exit 0
