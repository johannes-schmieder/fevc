*! kss_bc experimental compressed-cell runtime 0.1.0 16aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

/*
The objects in this file deliberately keep three different indices:

  * worker--firm coefficient cells;
  * declared deletion units; and
  * exact target-scale strata within coefficient cells.

None of those counts is inferred from another.  kssbc_scale__compact() drops
the preparation-only row maps, canonical source keys, redundant unit moments,
and target-group work arrays before the numerical phase.  The remaining
arrays are sufficient for the no-control two-way FE operator and its complete
original-equation residual certificate.
*/

string scalar kssbc_scale__version()
{
    return("0.1.0-experimental")
}

real scalar kssbc_scale__api_level()
{
    return(2)
}

string scalar kssbc_scale__build_id()
{
    return("kss-bc-scale-api2-cached-state")
}

struct kssbc_scale_id_map
{
    string scalar status
    real scalar levels
    real colvector key
    real colvector dense
    real colvector row_order
    real matrix panel
}

struct kssbc_scale_target_strata
{
    string scalar status
    string scalar message
    real scalar count
    real colvector cell
    real colvector per_copy_mass
    real colvector physical_count
    real colvector target_mass
    real colvector row_to_stratum
    real colvector row_order
    real matrix row_panel
    real colvector cell_order
    real matrix cell_panel
}

struct kssbc_scale_diagnostic
{
    string scalar status
    string scalar message
    real scalar n_rows
    real scalar n_physical
    real scalar worker_levels
    real scalar firm_levels
    real scalar coefficient_cells
    real scalar deletion_units
    real scalar target_strata
    real scalar cross_cell_deletion_units
    real scalar max_units_per_cell
    real scalar max_rows_per_cell
    real scalar max_rows_per_deletion_unit
    real scalar cells_equal_deletion_units
    real scalar row_cell_ratio
    real scalar leverage_rng_calls_per_probe
    real scalar target_rng_calls_per_probe
}

struct kssbc_scale_design
{
    string scalar status
    string scalar message
    real scalar n_rows
    real scalar n_physical
    real scalar worker_levels
    real scalar firm_levels
    real scalar coefficient_cells
    real scalar deletion_units
    real scalar target_weight_sum
    real scalar rank_tolerance

    real colvector worker_key
    real colvector firm_key
    real colvector deletion_key

    real colvector cell_worker
    real colvector cell_firm
    real colvector cell_frequency
    real colvector cell_outcome_sum
    real colvector cell_outcome_mean
    real colvector cell_outcome_centered_ss
    real colvector cell_target_mass

    real colvector unit_cell
    real colvector unit_frequency
    real colvector unit_outcome_sum
    real colvector unit_outcome_mean
    real colvector unit_outcome_centered_ss
    real colvector unit_target_mass
    real colvector unit_cell_order
    real matrix unit_cell_panel

    real colvector worker_order
    real matrix worker_panel
    real colvector firm_order
    real matrix firm_panel
    real colvector worker_weight
    real colvector firm_weight
    real colvector schur_diagonal
    real scalar preconditioner_ratio

    real colvector row_to_cell
    real colvector row_to_unit
    real colvector cell_row_order
    real matrix cell_row_panel
    real colvector unit_row_order
    real matrix unit_row_panel

    struct kssbc_scale_target_strata scalar strata
    struct kssbc_scale_diagnostic scalar diagnostic
}

struct kssbc_scale_runtime_state
{
    string scalar status
    string scalar message
    struct kssbc_scale_design scalar design
    string colvector unit_semantic_key
    string colvector stratum_semantic_key
}

real scalar kssbc_scale__exact_integer_total(real colvector value)
{
    real scalar exact_limit, one, row, total

    if (cols(value) != 1 | rows(value) == 0 | hasmissing(value)) return(.)
    exact_limit = 9007199254740992
    total = 0
    for (row=1; row<=rows(value); row++) {
        one = value[row]
        if (one <= 0 | one != floor(one) | one > exact_limit-total) return(.)
        total = total+one
    }
    return(total)
}

real scalar kssbc_scale__rng_call_count(real colvector trials)
{
    real scalar atom, calls, maximum_trials

    maximum_trials = 100000000000
    if (cols(trials) != 1 | rows(trials) < 1 | hasmissing(trials) |
        min(trials) < 0 | any(trials :!= floor(trials))) return(.)
    if (max(trials) <= maximum_trials) return(any(trials :> 0))
    calls = 0
    for (atom=1; atom<=rows(trials); atom++) {
        calls = calls+ceil(trials[atom]/maximum_trials)
    }
    return(calls)
}

struct kssbc_scale_id_map scalar kssbc_scale__canonical_ids(
    real colvector identifier)
{
    struct kssbc_scale_id_map scalar out
    real scalar group, position

    out.status = "INVALID_IDENTIFIER"
    out.levels = .
    out.key = J(0,1,.)
    out.dense = J(rows(identifier),1,.)
    out.row_order = J(0,1,.)
    out.panel = J(0,2,.)
    if (cols(identifier) != 1 | rows(identifier) == 0 |
        hasmissing(identifier)) return(out)

    out.row_order = order(identifier,1)
    out.panel = panelsetup(identifier[out.row_order],1)
    out.levels = rows(out.panel)
    out.key = identifier[out.row_order[out.panel[.,1]]]
    for (group=1; group<=out.levels; group++) {
        for (position=out.panel[group,1]; position<=out.panel[group,2];
            position++) {
            out.dense[out.row_order[position]] = group
        }
    }
    out.status = "CONVERGED"
    return(out)
}

real colvector kssbc_scale__row_group_map(
    real colvector row_order,
    real matrix panel,
    real scalar n_rows)
{
    real scalar group, position
    real colvector out

    out = J(n_rows,1,.)
    for (group=1; group<=rows(panel); group++) {
        for (position=panel[group,1]; position<=panel[group,2]; position++) {
            out[row_order[position]] = group
        }
    }
    return(out)
}

real matrix kssbc_scale__key_panel(real matrix sorted_key)
{
    real scalar row
    real colvector group_code

    if (rows(sorted_key) == 0 | cols(sorted_key) == 0 |
        hasmissing(sorted_key)) return(J(0,2,.))
    group_code = J(rows(sorted_key),1,1)
    for (row=2; row<=rows(sorted_key); row++) {
        group_code[row] = group_code[row-1] +
            (sum(sorted_key[row,.]:!=sorted_key[row-1,.]) > 0)
    }
    return(panelsetup(group_code,1))
}

/* Neumaier accumulation.  Its correction term retains small addends when a
   later large addend would otherwise erase them. */
real matrix kssbc_scale__stable_groupsum(
    real matrix values,
    real colvector row_order,
    real matrix panel)
{
    real scalar column, correction, delta, group, one, position, subtotal
    real scalar updated
    real matrix out

    if (rows(values) != rows(row_order) | rows(panel) == 0 |
        cols(values) == 0 | hasmissing(values)) return(J(0,0,.))
    out = J(rows(panel),cols(values),0)
    for (group=1; group<=rows(panel); group++) {
        for (column=1; column<=cols(values); column++) {
            subtotal = 0
            correction = 0
            for (position=panel[group,1]; position<=panel[group,2];
                position++) {
                one = values[row_order[position],column]
                updated = subtotal+one
                if (abs(subtotal) >= abs(one)) {
                    delta = (subtotal-updated)+one
                }
                else delta = (one-updated)+subtotal
                correction = correction+delta
                subtotal = updated
            }
            out[group,column] = subtotal+correction
        }
    }
    return(out)
}

real colvector kssbc_scale__integer_group_sum(
    real colvector values,
    real colvector row_order,
    real matrix panel)
{
    real scalar group, position, subtotal
    real colvector out

    if (rows(values) != rows(row_order) | rows(panel) == 0 |
        hasmissing(values) | min(values) <= 0 |
        any(values:!=floor(values))) return(J(0,1,.))
    out = J(rows(panel),1,0)
    for (group=1; group<=rows(panel); group++) {
        subtotal = 0
        for (position=panel[group,1]; position<=panel[group,2]; position++) {
            subtotal = subtotal+values[row_order[position]]
        }
        out[group] = subtotal
    }
    return(out)
}

real rowvector kssbc_scale__stable_colsum(real matrix values)
{
    real scalar column, correction, delta, one, row, subtotal, updated
    real rowvector out

    if (rows(values) == 0 | cols(values) == 0 | hasmissing(values)) {
        return(J(1,0,.))
    }
    out = J(1,cols(values),0)
    for (column=1; column<=cols(values); column++) {
        subtotal = 0
        correction = 0
        for (row=1; row<=rows(values); row++) {
            one = values[row,column]
            updated = subtotal+one
            if (abs(subtotal) >= abs(one)) {
                delta = (subtotal-updated)+one
            }
            else delta = (one-updated)+subtotal
            correction = correction+delta
            subtotal = updated
        }
        out[column] = subtotal+correction
    }
    return(out)
}

struct kssbc_scale_target_strata scalar kssbc_scale__strata_prepare(
    real colvector row_to_cell,
    real scalar coefficient_cells,
    real colvector frequency,
    real colvector target_weight)
{
    struct kssbc_scale_target_strata scalar out
    real colvector per_copy, first

    out.status = "INVALID_INPUT"
    out.message = "invalid exact target-stratum input"
    out.count = .
    out.cell = J(0,1,.)
    out.per_copy_mass = J(0,1,.)
    out.physical_count = J(0,1,.)
    out.target_mass = J(0,1,.)
    out.row_to_stratum = J(0,1,.)
    out.row_order = J(0,1,.)
    out.row_panel = J(0,2,.)
    out.cell_order = J(0,1,.)
    out.cell_panel = J(0,2,.)
    if (cols(row_to_cell) != 1 | rows(row_to_cell) == 0 |
        rows(frequency) != rows(row_to_cell) |
        rows(target_weight) != rows(row_to_cell) |
        missing(kssbc_scale__exact_integer_total(frequency)) |
        hasmissing(row_to_cell) | hasmissing(target_weight) |
        min(row_to_cell) != 1 | max(row_to_cell) != coefficient_cells |
        min(target_weight) < 0) return(out)

    per_copy = target_weight:/frequency
    if (hasmissing(per_copy)) return(out)
    out.row_order = order((row_to_cell,per_copy),(1,2))
    out.row_panel = kssbc_scale__key_panel(
        (row_to_cell,per_copy)[out.row_order,.])
    out.count = rows(out.row_panel)
    first = out.row_order[out.row_panel[.,1]]
    out.cell = row_to_cell[first]
    out.per_copy_mass = per_copy[first]
    out.physical_count = kssbc_scale__integer_group_sum(
        frequency,out.row_order,out.row_panel)
    out.target_mass = kssbc_scale__stable_groupsum(
        target_weight,out.row_order,out.row_panel)
    out.row_to_stratum = kssbc_scale__row_group_map(
        out.row_order,out.row_panel,rows(row_to_cell))
    out.cell_order = order(out.cell,1)
    out.cell_panel = panelsetup(out.cell[out.cell_order],1)
    if (rows(out.physical_count) != out.count |
        rows(out.target_mass) != out.count |
        rows(out.cell_panel) != coefficient_cells |
        hasmissing(out.target_mass)) return(out)
    out.status = "CONVERGED"
    out.message = "exact target-scale strata prepared"
    return(out)
}

struct kssbc_scale_diagnostic scalar kssbc_scale__diagnose(
    real colvector worker,
    real colvector firm,
    real colvector deletion_id,
    real colvector frequency,
    real colvector target_weight)
{
    struct kssbc_scale_diagnostic scalar out
    struct kssbc_scale_id_map scalar worker_map, firm_map, deletion_map
    struct kssbc_scale_target_strata scalar strata
    real scalar group, position
    real colvector cell_order, cell_first, row_to_cell, unit_cell, unit_order
    real colvector units_per_cell, rows_per_cell, rows_per_unit
    real colvector unit_frequency
    real matrix cell_panel, unit_cell_panel

    out.status = "INVALID_INPUT"
    out.message = "invalid compressed-design diagnostic input"
    out.n_rows = rows(worker)
    out.n_physical = .
    out.worker_levels = .
    out.firm_levels = .
    out.coefficient_cells = .
    out.deletion_units = .
    out.target_strata = .
    out.cross_cell_deletion_units = .
    out.max_units_per_cell = .
    out.max_rows_per_cell = .
    out.max_rows_per_deletion_unit = .
    out.cells_equal_deletion_units = .
    out.row_cell_ratio = .
    out.leverage_rng_calls_per_probe = .
    out.target_rng_calls_per_probe = .
    if (cols(worker) != 1 | rows(worker) == 0 |
        rows(firm) != rows(worker) | rows(deletion_id) != rows(worker) |
        rows(frequency) != rows(worker) |
        rows(target_weight) != rows(worker) | hasmissing(worker) |
        hasmissing(firm) | hasmissing(deletion_id) |
        hasmissing(target_weight) | min(target_weight) < 0) return(out)
    out.n_physical = kssbc_scale__exact_integer_total(frequency)
    if (missing(out.n_physical)) return(out)

    worker_map = kssbc_scale__canonical_ids(worker)
    firm_map = kssbc_scale__canonical_ids(firm)
    deletion_map = kssbc_scale__canonical_ids(deletion_id)
    if (worker_map.status != "CONVERGED" |
        firm_map.status != "CONVERGED" |
        deletion_map.status != "CONVERGED" |
        firm_map.levels < 2) return(out)
    out.worker_levels = worker_map.levels
    out.firm_levels = firm_map.levels
    out.deletion_units = deletion_map.levels

    cell_order = order((worker_map.dense,firm_map.dense),(1,2))
    cell_panel = kssbc_scale__key_panel(
        (worker_map.dense,firm_map.dense)[cell_order,.])
    out.coefficient_cells = rows(cell_panel)
    cell_first = cell_order[cell_panel[.,1]]
    row_to_cell = kssbc_scale__row_group_map(
        cell_order,cell_panel,rows(worker))
    rows_per_cell = cell_panel[.,2]-cell_panel[.,1]:+1
    rows_per_unit = deletion_map.panel[.,2]-deletion_map.panel[.,1]:+1
    unit_frequency = kssbc_scale__integer_group_sum(
        frequency,deletion_map.row_order,deletion_map.panel)
    out.max_rows_per_cell = max(rows_per_cell)
    out.max_rows_per_deletion_unit = max(rows_per_unit)

    unit_cell = J(out.deletion_units,1,.)
    out.cross_cell_deletion_units = 0
    for (group=1; group<=out.deletion_units; group++) {
        unit_cell[group] = row_to_cell[
            deletion_map.row_order[deletion_map.panel[group,1]]]
        for (position=deletion_map.panel[group,1]+1;
            position<=deletion_map.panel[group,2]; position++) {
            if (row_to_cell[deletion_map.row_order[position]] !=
                unit_cell[group]) {
                out.cross_cell_deletion_units =
                    out.cross_cell_deletion_units+1
                break
            }
        }
    }
    unit_order = order(unit_cell,1)
    unit_cell_panel = panelsetup(unit_cell[unit_order],1)
    units_per_cell = J(out.coefficient_cells,1,0)
    for (group=1; group<=rows(unit_cell_panel); group++) {
        units_per_cell[unit_cell[unit_order[unit_cell_panel[group,1]]]] =
            unit_cell_panel[group,2]-unit_cell_panel[group,1]+1
    }
    out.max_units_per_cell = max(units_per_cell)
    out.cells_equal_deletion_units =
        (out.coefficient_cells == out.deletion_units)
    out.row_cell_ratio = out.n_rows/out.coefficient_cells

    strata = kssbc_scale__strata_prepare(
        row_to_cell,out.coefficient_cells,frequency,target_weight)
    if (strata.status != "CONVERGED") return(out)
    out.target_strata = strata.count
    out.leverage_rng_calls_per_probe =
        kssbc_scale__rng_call_count(unit_frequency)
    out.target_rng_calls_per_probe =
        kssbc_scale__rng_call_count(strata.physical_count)
    if (missing((out.leverage_rng_calls_per_probe,
        out.target_rng_calls_per_probe))) return(out)
    out.status = "CONVERGED"
    out.message = "cell and deletion-unit counts independently certified"
    return(out)
}

struct kssbc_scale_design scalar kssbc_scale__prepare(
    real colvector worker,
    real colvector firm,
    real colvector deletion_id,
    real colvector frequency,
    real colvector outcome,
    real colvector target_weight,
    real scalar rank_tolerance)
{
    struct kssbc_scale_design scalar out
    struct kssbc_scale_id_map scalar worker_map, firm_map, deletion_map
    real colvector first, weighted_outcome, centered, adjustment
    real scalar group

    out.status = "INVALID_INPUT"
    out.message = "invalid compressed two-way design"
    out.n_rows = rows(worker)
    out.n_physical = .
    out.worker_levels = .
    out.firm_levels = .
    out.coefficient_cells = .
    out.deletion_units = .
    out.target_weight_sum = .
    out.rank_tolerance = rank_tolerance
    out.worker_key = J(0,1,.)
    out.firm_key = J(0,1,.)
    out.deletion_key = J(0,1,.)
    out.cell_worker = J(0,1,.)
    out.cell_firm = J(0,1,.)
    out.cell_frequency = J(0,1,.)
    out.cell_outcome_sum = J(0,1,.)
    out.cell_outcome_mean = J(0,1,.)
    out.cell_outcome_centered_ss = J(0,1,.)
    out.cell_target_mass = J(0,1,.)
    out.unit_cell = J(0,1,.)
    out.unit_frequency = J(0,1,.)
    out.unit_outcome_sum = J(0,1,.)
    out.unit_outcome_mean = J(0,1,.)
    out.unit_outcome_centered_ss = J(0,1,.)
    out.unit_target_mass = J(0,1,.)
    out.unit_cell_order = J(0,1,.)
    out.unit_cell_panel = J(0,2,.)
    out.worker_order = J(0,1,.)
    out.worker_panel = J(0,2,.)
    out.firm_order = J(0,1,.)
    out.firm_panel = J(0,2,.)
    out.worker_weight = J(0,1,.)
    out.firm_weight = J(0,1,.)
    out.schur_diagonal = J(0,1,.)
    out.preconditioner_ratio = .
    out.row_to_cell = J(0,1,.)
    out.row_to_unit = J(0,1,.)
    out.cell_row_order = J(0,1,.)
    out.cell_row_panel = J(0,2,.)
    out.unit_row_order = J(0,1,.)
    out.unit_row_panel = J(0,2,.)
    out.strata.status = "INVALID_INPUT"
    out.strata.message = "exact target strata have not been prepared"
    out.strata.count = .
    out.strata.cell = J(0,1,.)
    out.strata.per_copy_mass = J(0,1,.)
    out.strata.physical_count = J(0,1,.)
    out.strata.target_mass = J(0,1,.)
    out.strata.row_to_stratum = J(0,1,.)
    out.strata.row_order = J(0,1,.)
    out.strata.row_panel = J(0,2,.)
    out.strata.cell_order = J(0,1,.)
    out.strata.cell_panel = J(0,2,.)
    out.diagnostic.status = "INVALID_INPUT"
    out.diagnostic.message = "invalid compressed-design diagnostic input"
    out.diagnostic.n_rows = rows(worker)
    out.diagnostic.n_physical = .
    out.diagnostic.worker_levels = .
    out.diagnostic.firm_levels = .
    out.diagnostic.coefficient_cells = .
    out.diagnostic.deletion_units = .
    out.diagnostic.target_strata = .
    out.diagnostic.cross_cell_deletion_units = .
    out.diagnostic.max_units_per_cell = .
    out.diagnostic.max_rows_per_cell = .
    out.diagnostic.max_rows_per_deletion_unit = .
    out.diagnostic.cells_equal_deletion_units = .
    out.diagnostic.row_cell_ratio = .
    out.diagnostic.leverage_rng_calls_per_probe = .
    out.diagnostic.target_rng_calls_per_probe = .
    if (cols(worker) != 1 | rows(worker) == 0 |
        rows(firm) != rows(worker) | rows(deletion_id) != rows(worker) |
        rows(frequency) != rows(worker) | rows(outcome) != rows(worker) |
        rows(target_weight) != rows(worker) | cols(outcome) != 1 |
        hasmissing(worker) | hasmissing(firm) | hasmissing(deletion_id) |
        hasmissing(outcome) | hasmissing(target_weight) |
        min(target_weight) < 0 | missing(rank_tolerance) |
        rank_tolerance <= 0) return(out)
    out.n_physical = kssbc_scale__exact_integer_total(frequency)
    if (missing(out.n_physical)) return(out)

    worker_map = kssbc_scale__canonical_ids(worker)
    firm_map = kssbc_scale__canonical_ids(firm)
    deletion_map = kssbc_scale__canonical_ids(deletion_id)
    if (worker_map.status != "CONVERGED" |
        firm_map.status != "CONVERGED" |
        deletion_map.status != "CONVERGED" |
        firm_map.levels < 2) return(out)
    out.worker_levels = worker_map.levels
    out.firm_levels = firm_map.levels
    out.deletion_units = deletion_map.levels
    out.worker_key = worker_map.key
    out.firm_key = firm_map.key
    out.deletion_key = deletion_map.key

    out.cell_row_order = order(
        (worker_map.dense,firm_map.dense),(1,2))
    out.cell_row_panel = kssbc_scale__key_panel(
        (worker_map.dense,firm_map.dense)[out.cell_row_order,.])
    out.coefficient_cells = rows(out.cell_row_panel)
    first = out.cell_row_order[out.cell_row_panel[.,1]]
    out.cell_worker = worker_map.dense[first]
    out.cell_firm = firm_map.dense[first]
    out.row_to_cell = kssbc_scale__row_group_map(
        out.cell_row_order,out.cell_row_panel,out.n_rows)
    out.cell_frequency = kssbc_scale__integer_group_sum(
        frequency,out.cell_row_order,out.cell_row_panel)
    weighted_outcome = frequency:*outcome
    if (rows(out.cell_frequency) != out.coefficient_cells |
        hasmissing(weighted_outcome)) {
        out.status = "NUMERICAL_AGGREGATION"
        out.message = "cell mass or weighted outcomes cannot be represented"
        return(out)
    }
    out.cell_outcome_sum = kssbc_scale__stable_groupsum(
        weighted_outcome,out.cell_row_order,out.cell_row_panel)
    out.cell_target_mass = kssbc_scale__stable_groupsum(
        target_weight,out.cell_row_order,out.cell_row_panel)
    if (rows(out.cell_outcome_sum) != out.coefficient_cells |
        rows(out.cell_target_mass) != out.coefficient_cells |
        hasmissing(out.cell_outcome_sum) | hasmissing(out.cell_target_mass)) {
        out.status = "NUMERICAL_AGGREGATION"
        out.message = "cell sufficient statistics cannot be represented"
        return(out)
    }
    out.cell_outcome_mean = out.cell_outcome_sum:/out.cell_frequency
    centered = frequency:*(outcome:-out.cell_outcome_mean[out.row_to_cell]):^2
    if (hasmissing(out.cell_outcome_mean) | hasmissing(centered)) {
        out.status = "NUMERICAL_AGGREGATION"
        out.message = "cell centered outcomes cannot be represented"
        return(out)
    }
    out.cell_outcome_centered_ss = kssbc_scale__stable_groupsum(
        centered,out.cell_row_order,out.cell_row_panel)
    if (rows(out.cell_outcome_centered_ss) != out.coefficient_cells |
        hasmissing(out.cell_outcome_centered_ss)) {
        out.status = "NUMERICAL_AGGREGATION"
        out.message = "cell centered moments cannot be represented"
        return(out)
    }

    out.unit_row_order = deletion_map.row_order
    out.unit_row_panel = deletion_map.panel
    out.row_to_unit = deletion_map.dense
    first = out.unit_row_order[out.unit_row_panel[.,1]]
    out.unit_cell = out.row_to_cell[first]
    for (group=1; group<=out.deletion_units; group++) {
        if (min(out.row_to_cell[
                out.unit_row_order[out.unit_row_panel[group,1]..
                    out.unit_row_panel[group,2]]]) != out.unit_cell[group] |
            max(out.row_to_cell[
                out.unit_row_order[out.unit_row_panel[group,1]..
                    out.unit_row_panel[group,2]]]) != out.unit_cell[group]) {
            out.status = "FASTPATH_CROSS_CELL_BLOCK"
            out.message = "a deletion unit spans coefficient cells"
            return(out)
        }
    }
    out.unit_cell_order = order(out.unit_cell,1)
    out.unit_cell_panel = panelsetup(
        out.unit_cell[out.unit_cell_order],1)
    if (rows(out.unit_cell_panel) != out.coefficient_cells) return(out)
    out.unit_frequency = kssbc_scale__integer_group_sum(
        frequency,out.unit_row_order,out.unit_row_panel)
    out.unit_outcome_sum = kssbc_scale__stable_groupsum(
        weighted_outcome,out.unit_row_order,out.unit_row_panel)
    if (rows(out.unit_frequency) != out.deletion_units |
        rows(out.unit_outcome_sum) != out.deletion_units |
        hasmissing(out.unit_outcome_sum)) {
        out.status = "NUMERICAL_AGGREGATION"
        out.message = "deletion-unit sufficient statistics cannot be represented"
        return(out)
    }

    out.strata = kssbc_scale__strata_prepare(
        out.row_to_cell,out.coefficient_cells,frequency,target_weight)
    if (out.strata.status != "CONVERGED") return(out)
    out.target_weight_sum =
        kssbc_scale__stable_colsum(target_weight)[1]
    if (missing(out.target_weight_sum) | out.target_weight_sum <= 0) return(out)

    out.worker_order = order(out.cell_worker,1)
    out.worker_panel = panelsetup(out.cell_worker[out.worker_order],1)
    out.firm_order = order(out.cell_firm,1)
    out.firm_panel = panelsetup(out.cell_firm[out.firm_order],1)
    out.worker_weight = kssbc_scale__stable_groupsum(
        out.cell_frequency,out.worker_order,out.worker_panel)
    out.firm_weight = kssbc_scale__stable_groupsum(
        out.cell_frequency,out.firm_order,out.firm_panel)
    if (rows(out.worker_weight) != out.worker_levels |
        rows(out.firm_weight) != out.firm_levels |
        hasmissing(out.worker_weight) | hasmissing(out.firm_weight) |
        min(out.worker_weight) <= 0 | min(out.firm_weight) <= 0) {
        out.status = "NUMERICAL_AGGREGATION"
        out.message = "worker or firm mass cannot be represented"
        return(out)
    }
    adjustment = kssbc_scale__stable_groupsum(
        (out.cell_frequency:^2):/out.worker_weight[out.cell_worker],
        out.firm_order,out.firm_panel)
    out.schur_diagonal = out.firm_weight-adjustment
    if (hasmissing(out.schur_diagonal) | min(out.schur_diagonal) <= 0) {
        out.status = "SINGULAR_INFORMATION"
        out.message = "firm mobility quotient has a nonpositive diagonal"
        return(out)
    }
    out.preconditioner_ratio =
        min(out.schur_diagonal)/max(out.schur_diagonal)
    if (missing(out.preconditioner_ratio) | out.preconditioner_ratio <= 0) {
        out.status = "SINGULAR_INFORMATION"
        out.message = "firm mobility diagonal ratio is invalid"
        return(out)
    }

    out.diagnostic.status = "CONVERGED"
    out.diagnostic.message =
        "constructor counts; use kssbc_scale__diagnose() for independent certification"
    out.diagnostic.n_rows = out.n_rows
    out.diagnostic.n_physical = out.n_physical
    out.diagnostic.worker_levels = out.worker_levels
    out.diagnostic.firm_levels = out.firm_levels
    out.diagnostic.coefficient_cells = out.coefficient_cells
    out.diagnostic.deletion_units = out.deletion_units
    out.diagnostic.target_strata = out.strata.count
    out.diagnostic.cross_cell_deletion_units = 0
    out.diagnostic.max_units_per_cell = max(
        out.unit_cell_panel[.,2]-out.unit_cell_panel[.,1]:+1)
    out.diagnostic.max_rows_per_cell = max(
        out.cell_row_panel[.,2]-out.cell_row_panel[.,1]:+1)
    out.diagnostic.max_rows_per_deletion_unit = max(
        out.unit_row_panel[.,2]-out.unit_row_panel[.,1]:+1)
    out.diagnostic.cells_equal_deletion_units =
        (out.coefficient_cells == out.deletion_units)
    out.diagnostic.row_cell_ratio = out.n_rows/out.coefficient_cells
    out.diagnostic.leverage_rng_calls_per_probe =
        kssbc_scale__rng_call_count(out.unit_frequency)
    out.diagnostic.target_rng_calls_per_probe =
        kssbc_scale__rng_call_count(out.strata.physical_count)
    if (missing((out.diagnostic.leverage_rng_calls_per_probe,
        out.diagnostic.target_rng_calls_per_probe))) {
        out.status = "BINOMIAL_CONTRACT_UNSUPPORTED"
        out.message = "registered compressed RNG call count is not representable"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "canonical compressed two-way design prepared"
    return(out)
}

real matrix kssbc_scale__collapse_rows(
    struct kssbc_scale_design scalar design,
    real matrix row_values)
{
    if (design.status != "CONVERGED" | rows(design.row_to_cell) == 0 |
        rows(row_values) != design.n_rows | cols(row_values) == 0 |
        hasmissing(row_values)) return(J(0,0,.))
    return(kssbc_scale__stable_groupsum(
        row_values,design.cell_row_order,design.cell_row_panel))
}

real matrix kssbc_scale__fe_transpose(
    struct kssbc_scale_design scalar design,
    real matrix cell_values)
{
    real matrix firm_part, worker_part

    if (design.status != "CONVERGED" |
        rows(cell_values) != design.coefficient_cells |
        cols(cell_values) == 0 | hasmissing(cell_values)) return(J(0,0,.))
    worker_part = kssbc_scale__stable_groupsum(
        cell_values,design.worker_order,design.worker_panel)
    firm_part = kssbc_scale__stable_groupsum(
        cell_values,design.firm_order,design.firm_panel)
    return(worker_part \ firm_part[1..(design.firm_levels-1),.])
}

real matrix kssbc_scale__fe_transpose_full(
    struct kssbc_scale_design scalar design,
    real matrix cell_values)
{
    real matrix firm_part, worker_part

    if (design.status != "CONVERGED" |
        rows(cell_values) != design.coefficient_cells |
        cols(cell_values) == 0 | hasmissing(cell_values)) return(J(0,0,.))
    worker_part = kssbc_scale__stable_groupsum(
        cell_values,design.worker_order,design.worker_panel)
    firm_part = kssbc_scale__stable_groupsum(
        cell_values,design.firm_order,design.firm_panel)
    return(worker_part \ firm_part)
}

real matrix kssbc_scale__fe_schur_action(
    struct kssbc_scale_design scalar design,
    real matrix firm_coefficient)
{
    real matrix fitted, firm_sum, worker_mean

    if (design.status != "CONVERGED" |
        rows(firm_coefficient) != design.firm_levels |
        cols(firm_coefficient) == 0 |
        hasmissing(firm_coefficient)) return(J(0,0,.))
    fitted = firm_coefficient[design.cell_firm,.]
    worker_mean = kssbc_scale__stable_groupsum(
        design.cell_frequency:*fitted,
        design.worker_order,design.worker_panel):/design.worker_weight
    firm_sum = kssbc_scale__stable_groupsum(
        design.cell_frequency:*(fitted:-worker_mean[design.cell_worker,.]),
        design.firm_order,design.firm_panel)
    return(firm_sum)
}

real matrix kssbc_scale__full_firm(
    struct kssbc_scale_design scalar design,
    real matrix coefficient)
{
    real matrix out

    if (design.status != "CONVERGED" |
        rows(coefficient) != design.worker_levels+design.firm_levels-1 |
        cols(coefficient) == 0 | hasmissing(coefficient)) return(J(0,0,.))
    out = J(design.firm_levels,cols(coefficient),0)
    out[1..(design.firm_levels-1),.] =
        coefficient[(design.worker_levels+1)..rows(coefficient),.]
    return(out)
}

real matrix kssbc_scale__fe_worker(
    struct kssbc_scale_design scalar design,
    real matrix right_hand_side,
    real matrix firm_coefficient)
{
    real matrix firm_contribution, worker_rhs

    if (design.status != "CONVERGED" |
        rows(right_hand_side) != design.worker_levels+design.firm_levels-1 |
        rows(firm_coefficient) != design.firm_levels |
        cols(right_hand_side) == 0 |
        cols(firm_coefficient) != cols(right_hand_side) |
        hasmissing(right_hand_side) | hasmissing(firm_coefficient)) {
        return(J(0,0,.))
    }
    worker_rhs = right_hand_side[1..design.worker_levels,.]
    firm_contribution = kssbc_scale__stable_groupsum(
        design.cell_frequency:*firm_coefficient[design.cell_firm,.],
        design.worker_order,design.worker_panel)
    return((worker_rhs-firm_contribution):/design.worker_weight)
}

real matrix kssbc_scale__fe_from_firm(
    struct kssbc_scale_design scalar design,
    real matrix right_hand_side,
    real matrix firm_coefficient)
{
    real matrix worker_coefficient

    worker_coefficient = kssbc_scale__fe_worker(
        design,right_hand_side,firm_coefficient)
    if (rows(worker_coefficient) == 0) return(J(0,0,.))
    return(worker_coefficient \ firm_coefficient[1..(design.firm_levels-1),.])
}

real matrix kssbc_scale__fe_predict(
    struct kssbc_scale_design scalar design,
    real matrix coefficient)
{
    real matrix firm_coefficient

    firm_coefficient = kssbc_scale__full_firm(design,coefficient)
    if (rows(firm_coefficient) == 0) return(J(0,0,.))
    return(coefficient[1..design.worker_levels,.][design.cell_worker,.] :+
        firm_coefficient[design.cell_firm,.])
}

real matrix kssbc_scale__fe_full_rhs(
    struct kssbc_scale_design scalar design,
    real matrix right_hand_side)
{
    real matrix firm_rhs, worker_rhs

    if (design.status != "CONVERGED" |
        !(rows(right_hand_side) ==
            design.worker_levels+design.firm_levels-1 |
          rows(right_hand_side) ==
            design.worker_levels+design.firm_levels) |
        cols(right_hand_side) == 0 | hasmissing(right_hand_side)) {
        return(J(0,0,.))
    }
    if (rows(right_hand_side) ==
        design.worker_levels+design.firm_levels) return(right_hand_side)
    worker_rhs = right_hand_side[1..design.worker_levels,.]
    firm_rhs = right_hand_side[(design.worker_levels+1)..rows(right_hand_side),.]
    return(worker_rhs \ firm_rhs \
        (kssbc_scale__stable_colsum(worker_rhs) :-
        kssbc_scale__stable_colsum(firm_rhs)))
}

real matrix kssbc_scale__fe_full_residual(
    struct kssbc_scale_design scalar design,
    real matrix right_hand_side,
    real matrix coefficient)
{
    real matrix alpha, firm_coefficient, firm_lhs, full_rhs, worker_lhs

    if (cols(right_hand_side) != cols(coefficient)) return(J(0,0,.))
    full_rhs = kssbc_scale__fe_full_rhs(design,right_hand_side)
    firm_coefficient = kssbc_scale__full_firm(design,coefficient)
    if (rows(full_rhs) == 0 | rows(firm_coefficient) == 0) return(J(0,0,.))
    alpha = coefficient[1..design.worker_levels,.]
    worker_lhs = design.worker_weight:*alpha +
        kssbc_scale__stable_groupsum(
            design.cell_frequency:*firm_coefficient[design.cell_firm,.],
            design.worker_order,design.worker_panel)
    firm_lhs = design.firm_weight:*firm_coefficient +
        kssbc_scale__stable_groupsum(
            design.cell_frequency:*alpha[design.cell_worker,.],
            design.firm_order,design.firm_panel)
    return(full_rhs-(worker_lhs \ firm_lhs))
}

real rowvector kssbc_scale__fe_full_relres(
    struct kssbc_scale_design scalar design,
    real matrix right_hand_side,
    real matrix coefficient)
{
    real scalar column, residual_norm, rhs_norm
    real matrix full_rhs, residual
    real rowvector out

    full_rhs = kssbc_scale__fe_full_rhs(design,right_hand_side)
    residual = kssbc_scale__fe_full_residual(
        design,right_hand_side,coefficient)
    if (rows(full_rhs) == 0 | rows(residual) == 0) return(J(1,0,.))
    out = J(1,cols(full_rhs),.)
    for (column=1; column<=cols(full_rhs); column++) {
        rhs_norm = sqrt(sum(full_rhs[.,column]:^2))
        residual_norm = sqrt(sum(residual[.,column]:^2))
        if (rhs_norm == 0) out[column] = residual_norm
        else out[column] = residual_norm/rhs_norm
    }
    return(out)
}

real rowvector kssbc_scale__weighted_rss(
    struct kssbc_scale_design scalar design,
    real matrix cell_prediction)
{
    real matrix contribution

    if (design.status != "CONVERGED" |
        rows(cell_prediction) != design.coefficient_cells |
        cols(cell_prediction) == 0 | hasmissing(cell_prediction)) {
        return(J(1,0,.))
    }
    contribution = design.cell_outcome_centered_ss :+
        design.cell_frequency:*(design.cell_outcome_mean:-cell_prediction):^2
    return(kssbc_scale__stable_colsum(contribution))
}

real matrix kssbc_scale__strata_to_cells(
    struct kssbc_scale_target_strata scalar strata,
    real matrix stratum_values,
    real scalar coefficient_cells)
{
    if (strata.status != "CONVERGED" |
        rows(stratum_values) != strata.count | cols(stratum_values) == 0 |
        rows(strata.cell_panel) != coefficient_cells |
        hasmissing(stratum_values)) return(J(0,0,.))
    return(kssbc_scale__stable_groupsum(
        stratum_values,strata.cell_order,strata.cell_panel))
}

struct kssbc_scale_design scalar kssbc_scale__compact(
    struct kssbc_scale_design scalar design)
{
    struct kssbc_scale_design scalar out

    out = design
    out.row_to_cell = J(0,1,.)
    out.row_to_unit = J(0,1,.)
    out.cell_row_order = J(0,1,.)
    out.cell_row_panel = J(0,2,.)
    out.unit_row_order = J(0,1,.)
    out.unit_row_panel = J(0,2,.)
    out.strata.row_to_stratum = J(0,1,.)
    out.strata.row_order = J(0,1,.)
    out.strata.row_panel = J(0,2,.)
    out.worker_key = J(0,1,.)
    out.firm_key = J(0,1,.)
    out.deletion_key = J(0,1,.)
    out.unit_outcome_mean = J(0,1,.)
    out.unit_outcome_centered_ss = J(0,1,.)
    out.unit_target_mass = J(0,1,.)
    out.unit_cell_order = J(0,1,.)
    out.unit_cell_panel = J(0,2,.)
    out.strata.target_mass = J(0,1,.)
    out.strata.cell_order = J(0,1,.)
    out.strata.cell_panel = J(0,2,.)
    return(out)
}

struct kssbc_scale_runtime_state scalar kssbc_srt__empty()
{
    struct kssbc_scale_runtime_state scalar out

    out.status = "EMPTY"
    out.message = "no compressed command state is prepared"
    out.design = kssbc_scale__prepare(
        J(0,1,.),J(0,1,.),J(0,1,.),J(0,1,.),J(0,1,.),J(0,1,.),1e-10)
    out.unit_semantic_key = J(0,1,"")
    out.stratum_semantic_key = J(0,1,"")
    return(out)
}

real colvector kssbc_srt__group_min(
    real colvector value,
    real colvector row_order,
    real matrix panel)
{
    real scalar group
    real colvector out

    if (cols(value) != 1 | rows(value) == 0 | hasmissing(value) |
        rows(row_order) != rows(value) | rows(panel) == 0) {
        return(J(0,1,.))
    }
    out = J(rows(panel),1,.)
    for (group=1; group<=rows(panel); group++) {
        out[group] = min(value[row_order[|
            panel[group,1] \ panel[group,2]|]])
    }
    return(out)
}

string colvector kssbc_srt__keys(
    string scalar prefix,
    real colvector value)
{
    real scalar row
    string colvector out

    if (prefix == "" | cols(value) != 1 | rows(value) == 0 |
        hasmissing(value) | min(value) < 1 |
        any(value :!= floor(value)) |
        rows(uniqrows(sort(value,1))) != rows(value)) {
        return(J(0,1,""))
    }
    out = J(rows(value),1,"")
    for (row=1; row<=rows(value); row++) {
        out[row] = prefix+sprintf("%021.0f",value[row])
    }
    return(out)
}

void kssbc_scale_runtime__reset()
{
    external struct kssbc_scale_runtime_state scalar KSSBC_SCALE_RUNTIME

    KSSBC_SCALE_RUNTIME = kssbc_srt__empty()
}

string scalar kssbc_scale_runtime__status()
{
    external struct kssbc_scale_runtime_state scalar KSSBC_SCALE_RUNTIME

    return(KSSBC_SCALE_RUNTIME.status)
}

void kssbc_srt__prepare(
    string scalar outcome_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar deletion_name,
    string scalar frequency_name,
    string scalar target_name,
    string scalar semantic_rank_name,
    string scalar sample_name,
    real scalar rank_tolerance,
    string scalar diagnostics_name,
    string scalar status_local,
    string scalar message_local)
{
    external struct kssbc_scale_runtime_state scalar KSSBC_SCALE_RUNTIME
    struct kssbc_scale_design scalar design
    struct kssbc_scale_diagnostic scalar diagnostic
    real colvector worker, firm, deletion_id, frequency, outcome, target
    real colvector semantic_rank, unit_rank, stratum_rank
    string colvector unit_key, stratum_key

    KSSBC_SCALE_RUNTIME = kssbc_srt__empty()
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    outcome = st_data(.,outcome_name,sample_name)
    target = st_data(.,target_name,sample_name)
    semantic_rank = st_data(.,semantic_rank_name,sample_name)
    design = kssbc_scale__prepare(
        worker,firm,deletion_id,frequency,outcome,target,rank_tolerance)
    diagnostic = design.diagnostic
    st_matrix(diagnostics_name,(
        diagnostic.n_rows,diagnostic.n_physical,
        diagnostic.worker_levels,diagnostic.firm_levels,
        diagnostic.coefficient_cells,diagnostic.deletion_units,
        diagnostic.target_strata,diagnostic.cross_cell_deletion_units,
        diagnostic.max_units_per_cell,diagnostic.max_rows_per_cell,
        diagnostic.max_rows_per_deletion_unit,
        diagnostic.cells_equal_deletion_units,diagnostic.row_cell_ratio,
        diagnostic.leverage_rng_calls_per_probe,
        diagnostic.target_rng_calls_per_probe))
    if (design.status != "CONVERGED") {
        KSSBC_SCALE_RUNTIME.status = design.status
        KSSBC_SCALE_RUNTIME.message = design.message
        st_local(status_local,KSSBC_SCALE_RUNTIME.status)
        st_local(message_local,KSSBC_SCALE_RUNTIME.message)
        return
    }
    if (diagnostic.status != "CONVERGED" |
        diagnostic.coefficient_cells != design.coefficient_cells |
        diagnostic.deletion_units != design.deletion_units |
        diagnostic.target_strata != design.strata.count) {
        KSSBC_SCALE_RUNTIME.status = "FASTPATH_DIAGNOSTIC_MISMATCH"
        KSSBC_SCALE_RUNTIME.message =
            "prepared compressed-state counts are internally inconsistent"
        st_local(status_local,KSSBC_SCALE_RUNTIME.status)
        st_local(message_local,KSSBC_SCALE_RUNTIME.message)
        return
    }
    if (cols(semantic_rank) != 1 | rows(semantic_rank) != design.n_rows |
        hasmissing(semantic_rank) | min(semantic_rank) < 1 |
        any(semantic_rank :!= floor(semantic_rank))) {
        KSSBC_SCALE_RUNTIME.status = "RNG_SEMANTIC_KEY_INVALID"
        KSSBC_SCALE_RUNTIME.message =
            "canonical retained-row semantic ranks are invalid"
        st_local(status_local,KSSBC_SCALE_RUNTIME.status)
        st_local(message_local,KSSBC_SCALE_RUNTIME.message)
        return
    }
    unit_rank = kssbc_srt__group_min(
        semantic_rank,design.unit_row_order,design.unit_row_panel)
    stratum_rank = kssbc_srt__group_min(
        semantic_rank,design.strata.row_order,design.strata.row_panel)
    unit_key = kssbc_srt__keys("U",unit_rank)
    stratum_key = kssbc_srt__keys("T",stratum_rank)
    if (rows(unit_key) != design.deletion_units |
        rows(stratum_key) != design.strata.count) {
        KSSBC_SCALE_RUNTIME.status = "RNG_SEMANTIC_KEY_INVALID"
        KSSBC_SCALE_RUNTIME.message =
            "deletion-unit or target-stratum semantic keys are not unique"
        st_local(status_local,KSSBC_SCALE_RUNTIME.status)
        st_local(message_local,KSSBC_SCALE_RUNTIME.message)
        return
    }
    KSSBC_SCALE_RUNTIME.design = kssbc_scale__compact(design)
    KSSBC_SCALE_RUNTIME.unit_semantic_key = unit_key
    KSSBC_SCALE_RUNTIME.stratum_semantic_key = stratum_key
    KSSBC_SCALE_RUNTIME.status = "PREPARED"
    KSSBC_SCALE_RUNTIME.message =
        "canonical compressed command state is cached"
    st_local(status_local,KSSBC_SCALE_RUNTIME.status)
    st_local(message_local,KSSBC_SCALE_RUNTIME.message)
}

void kssbc_scale__stata_diag(
    string scalar worker_name,
    string scalar firm_name,
    string scalar deletion_name,
    string scalar frequency_name,
    string scalar target_name,
    string scalar sample_name,
    string scalar diagnostics_name,
    string scalar status_local,
    string scalar message_local)
{
    struct kssbc_scale_diagnostic scalar out

    out = kssbc_scale__diagnose(
        st_data(.,worker_name,sample_name),
        st_data(.,firm_name,sample_name),
        st_data(.,deletion_name,sample_name),
        st_data(.,frequency_name,sample_name),
        st_data(.,target_name,sample_name))
    st_matrix(diagnostics_name,(
        out.n_rows,out.n_physical,out.worker_levels,out.firm_levels,
        out.coefficient_cells,out.deletion_units,out.target_strata,
        out.cross_cell_deletion_units,out.max_units_per_cell,
        out.max_rows_per_cell,out.max_rows_per_deletion_unit,
        out.cells_equal_deletion_units,out.row_cell_ratio,
        out.leverage_rng_calls_per_probe,
        out.target_rng_calls_per_probe))
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

KSSBC_SCALE_RUNTIME = kssbc_srt__empty()

end
