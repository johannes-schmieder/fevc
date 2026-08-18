version 18.0
clear all
set more off
set varabbrev off

capture confirm file "varcomp_kss/varcomp_kss.mata"
if _rc {
    di as error "run the scale compression test from the repository root"
    exit 601
}

quietly do "varcomp_kss/varcomp_kss.mata"
quietly do "varcomp_kss/varcomp_kss_scale.mata"

mata:
assert(vckss_scale__api_level() == 3)
assert(vckss_scale__build_id() ==
    "varcomp-kss-scale-api3-numopt2-dual-order")
void test_scale_compression()
{
    real scalar n_rows
    real colvector worker, firm, deletion, frequency, outcome, target
    real colvector bad_deletion, permutation, inverse_permutation
    real matrix row_values, cell_values, base_rhs, compressed_rhs
    real matrix gamma, base_schur, compressed_schur, coefficient
    real matrix base_prediction, cell_prediction, residual, full_rhs
    real matrix manual_residual, worker_lhs, firm_lhs, alpha
    real matrix compact_schur
    struct vckss_fe_design scalar base
    struct vckss_scale_design scalar design, reordered, compact
    struct vckss_scale_diagnostic scalar diagnostic

    worker = (1\1\1\1\2\2\2\2\3\3\3\3)
    firm = (1\1\2\2\1\1\3\3\2\2\3\3)
    deletion = (10\11\12\12\20\20\21\22\30\30\31\31)
    frequency = (2\1\1\3\2\2\1\4\1\2\3\1)
    outcome = (.2\-.1\.8\1.1\-.4\.3\1.4\1\-.2\.5\.9\1.3)
    target = frequency
    target[2] = target[2]/2
    target[8] = target[8]/2
    target[12] = 1+2.2204460492503131e-16
    n_rows = rows(worker)

    diagnostic = vckss_scale__diagnose(
        worker,firm,deletion,frequency,target)
    assert(diagnostic.status == "CONVERGED")
    assert(diagnostic.coefficient_cells == 6)
    assert(diagnostic.deletion_units == 8)
    assert(diagnostic.target_strata == 9)
    assert(diagnostic.max_units_per_cell == 2)
    assert(diagnostic.cross_cell_deletion_units == 0)
    assert(!diagnostic.cells_equal_deletion_units)

    design = vckss_scale__prepare(
        worker,firm,deletion,frequency,outcome,target,1e-10)
    assert(design.status == "CONVERGED")
    assert(design.coefficient_cells == 6)
    assert(design.deletion_units == 8)
    assert(rows(design.unit_cell) == 8)
    assert(sum(design.unit_cell:==1) == 2)
    assert(design.strata.count == 9)
    assert(rows(design.unit_cell_panel) == design.coefficient_cells)
    assert(mreldif(vckss_scale__stable_groupsum(
        design.unit_outcome_sum,design.unit_cell_order,
        design.unit_cell_panel),design.cell_outcome_sum) <= 2e-15)
    assert(mreldif(vckss_scale__strata_to_cells(
        design.strata,design.strata.target_mass,
        design.coefficient_cells),design.cell_target_mass) <= 2e-15)
    assert(vckss_scale__stable_colsum((1e16\1\-1e16))[1] == 1)
    assert(vckss_scale__version() == "0.1.0-experimental")

    base = vckss__fe_prepare(worker,firm,frequency,1e-10)
    assert(base.status == "CONVERGED")
    row_values = (outcome, sin(1::n_rows))
    cell_values = vckss_scale__collapse_rows(design,row_values)
    base_rhs = vckss__fe_transpose(base,row_values)
    compressed_rhs = vckss_scale__fe_transpose(design,cell_values)
    assert(mreldif(base_rhs,compressed_rhs) <= 2e-15)

    gamma = ((.4,-.1)\(-.2,.3)\(0,0))
    base_schur = vckss__fe_schur_action(base,gamma)
    compressed_schur = vckss_scale__fe_schur_action(design,gamma)
    assert(mreldif(base_schur,compressed_schur) <= 2e-15)
    assert(mreldif(base.schur_diagonal,design.schur_diagonal) <= 2e-15)

    coefficient = ((.3,-.2)\(-.1,.4)\(.5,.2)\(.7,-.3)\(-.6,.8))
    base_prediction = vckss__fe_predict(base,coefficient)
    cell_prediction = vckss_scale__fe_predict(design,coefficient)
    assert(mreldif(base_prediction,cell_prediction[design.row_to_cell,.]) <=
        2e-15)
    assert(mreldif(
        vckss_scale__weighted_rss(design,cell_prediction),
        colsum(frequency:*(outcome:-base_prediction):^2)) <= 2e-15)

    residual = vckss_scale__fe_full_residual(
        design,base_rhs,coefficient)
    full_rhs = vckss_scale__fe_full_rhs(design,base_rhs)
    alpha = coefficient[1..design.worker_levels,.]
    gamma = vckss_scale__full_firm(design,coefficient)
    worker_lhs = vckss__group_sum(
        frequency:*(alpha[worker,.]:+gamma[firm,.]),
        base.worker_order,base.worker_panel)
    firm_lhs = vckss__group_sum(
        frequency:*(alpha[worker,.]:+gamma[firm,.]),
        base.firm_order,base.firm_panel)
    manual_residual = full_rhs-(worker_lhs\firm_lhs)
    assert(mreldif(residual,manual_residual) <= 3e-15)
    assert(cols(vckss_scale__fe_full_relres(
        design,base_rhs,coefficient)) == 2)

    coefficient = vckss_scale__fe_from_firm(
        design,base_rhs,gamma)
    residual = vckss_scale__fe_full_residual(
        design,base_rhs,coefficient)
    assert(max(abs(residual[1..design.worker_levels,.])) <= 3e-15)

    // Canonical aggregates do not depend on raw row order.
    permutation = (12\3\7\1\11\6\8\2\10\5\4\9)
    inverse_permutation = invorder(permutation)
    reordered = vckss_scale__prepare(
        worker[permutation],firm[permutation],deletion[permutation],
        frequency[permutation],outcome[permutation],target[permutation],1e-10)
    assert(reordered.status == "CONVERGED")
    assert(mreldif(reordered.cell_frequency,design.cell_frequency) == 0)
    assert(mreldif(reordered.cell_outcome_sum,design.cell_outcome_sum) <= 2e-15)
    assert(mreldif(reordered.unit_frequency,design.unit_frequency) == 0)
    assert(mreldif(reordered.unit_outcome_sum,design.unit_outcome_sum) <= 2e-15)
    assert(mreldif(
        reordered.row_to_cell[inverse_permutation],design.row_to_cell) == 0)

    compact = vckss_scale__compact(design)
    assert(rows(compact.row_to_cell) == 0)
    assert(rows(compact.row_to_unit) == 0)
    assert(rows(compact.strata.row_to_stratum) == 0)
    assert(rows(compact.worker_key) == 0)
    assert(rows(compact.firm_key) == 0)
    assert(rows(compact.deletion_key) == 0)
    assert(rows(compact.unit_outcome_mean) == 0)
    assert(rows(compact.unit_outcome_centered_ss) == 0)
    assert(rows(compact.unit_target_mass) == 0)
    assert(rows(compact.unit_cell_order) == 0)
    assert(rows(compact.strata.target_mass) == 0)
    compressed_schur = vckss_scale__fe_schur_action(design,gamma)
    compact_schur = vckss_scale__fe_schur_action(compact,gamma)
    assert(mreldif(compact_schur,compressed_schur) == 0)

    // Exact scale grouping never merges adjacent floating-point values.
    assert(target[11]/frequency[11] != target[12]/frequency[12])
    assert(sum(design.strata.cell:==6) == 2)

    // The independent diagnostic and constructor both reject a declared
    // deletion unit that spans two coefficient coordinates.
    bad_deletion = deletion
    bad_deletion[5] = bad_deletion[1]
    diagnostic = vckss_scale__diagnose(
        worker,firm,bad_deletion,frequency,target)
    assert(diagnostic.status == "CONVERGED")
    assert(diagnostic.cross_cell_deletion_units == 1)
    design = vckss_scale__prepare(
        worker,firm,bad_deletion,frequency,outcome,target,1e-10)
    assert(design.status == "FASTPATH_CROSS_CELL_BLOCK")
}

test_scale_compression()
end

di as result "PASS test_scale_compression.do"
exit 0
