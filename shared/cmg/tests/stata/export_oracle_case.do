version 18.0
clear all
set more off
set varabbrev off

args repository_root input_csv output_csv coarse_max
if strtrim(`"`repository_root'"') == "" | ///
    strtrim(`"`input_csv'"') == "" | ///
    strtrim(`"`output_csv'"') == "" {
    exit 198
}
capture confirm integer number `coarse_max'
if _rc | `coarse_max' < 1 exit 198

quietly import delimited using `"`input_csv'"', clear varnames(1)
confirm variable worker firm weight
assert worker == floor(worker) & worker >= 1
assert firm == floor(firm) & firm >= 1
assert weight > 0 & weight < .

mata: mata clear
do `"`repository_root'/shared/cmg/generated/cmg_test.mata"'

mata:
void cmgtest__export_oracle_case(real scalar coarse_max)
{
    struct cmgtest__cells scalar cells
    struct cmgtest__graph scalar graph
    struct cmgtest__options scalar options
    struct cmgtest__hierarchy scalar hierarchy
    struct cmgtest__apply_result scalar result
    real colvector worker, firm, weight

    worker = st_data(.,"worker")
    firm = st_data(.,"firm")
    weight = st_data(.,"weight")
    cells = cmgtest__cells_prepare(worker,firm,weight,
        (1::max(worker)),(1::max(firm)))
    if (cells.status != "CONVERGED") {
        errprintf("cell preparation failed: %s: %s\n",
            cells.status,cells.message)
        exit(3498)
    }
    graph = cmgtest__hybrid_build(cells)
    if (graph.status != "CONVERGED") {
        errprintf("hybrid build failed: %s: %s\n",
            graph.status,graph.message)
        exit(3498)
    }
    options = cmgtest__options_default()
    options.coarse_max = coarse_max
    hierarchy = cmgtest__hierarchy_build(graph,options)
    if (hierarchy.status != "CONVERGED") {
        errprintf("hierarchy build failed: %s: %s\n",
            hierarchy.status,hierarchy.message)
        exit(3498)
    }
    result = cmgtest__apply_kss(hierarchy,I(cells.n_firm))
    if (result.status != "CONVERGED") {
        errprintf("KSS pullback failed: %s: %s\n",
            result.status,result.message)
        exit(3498)
    }
    st_matrix("cmg_case_cycle",result.value)
    st_matrix("cmg_case_schur",cmgtest__dense_hybrid_schur(graph):*
        graph.weight_scale)
}

cmgtest__export_oracle_case(strtoreal(st_local("coarse_max")))
end

clear
quietly svmat double cmg_case_cycle, names(cycle)
quietly svmat double cmg_case_schur, names(schur)
generate long row = _n
format cycle* schur* %24.17g
order row
export delimited using `"`output_csv'"', replace datafmt
di as result "CMG GENERIC ORACLE CASE EXPORT PASS"
exit 0
