*! kss_bc Mata runtime 0.1.0-dev 14aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

string scalar kssbc__version()
{
    return("0.1.0-dev")
}

real scalar kssbc__api_level()
{
    return(9)
}

string scalar kssbc__build_id()
{
    return("kss-bc-api9-coordinatewise-copy-tie-safe")
}

real scalar kssbc__norm2(real matrix value)
{
    if (rows(value) == 0 | cols(value) == 0) return(0)
    return(sqrt(sum(vec(value):^2)))
}

real scalar kssbc__max_column_relres(
    real matrix residual,
    real matrix right_hand_side)
{
    real scalar column, maximum, one

    if (rows(residual) != rows(right_hand_side) |
        cols(residual) != cols(right_hand_side) |
        rows(residual) == 0 | cols(residual) == 0 |
        hasmissing(residual) | hasmissing(right_hand_side)) return(.)
    maximum = 0
    for (column=1; column<=cols(residual); column++) {
        one = kssbc__norm2(residual[.,column]) /
            (1+kssbc__norm2(right_hand_side[.,column]))
        maximum = max((maximum,one))
    }
    return(maximum)
}

real scalar kssbc__timer_seconds(real scalar identifier)
{
    real matrix value

    value = timer_value(identifier)
    return(value[1,1])
}

struct kssbc_inverse_result
{
    string scalar status
    real matrix inverse
    real scalar rcond
    real scalar relres
}

struct kssbc_target_matrices
{
    real matrix worker
    real matrix firm
    real matrix covariance
}

struct kssbc_result
{
    string scalar status
    string scalar message
    real rowvector plugin
    real rowvector correction
    real rowvector corrected
    real rowvector numerical_mcse
    real scalar n_stored
    real scalar n_physical
    real scalar worker_levels
    real scalar firm_levels
    real scalar parameters
    real scalar deletion_units
    real scalar target_weight_sum
    real scalar max_leverage
    real scalar information_rcond
    real scalar preconditioner_ratio
    real scalar control_schur_rcond
    real scalar deletion_rank_gap
    real scalar inverse_relres
    real scalar weighted_rss
    real scalar fit_seconds
    real scalar leverage_seconds
    real scalar target_seconds
    real scalar correction_seconds
    real scalar preconditioner_seconds
    real scalar solver_iterations
    real scalar solver_max_residual
    real scalar probes
}

struct kssbc_result scalar kssbc__empty_result()
{
    struct kssbc_result scalar out

    out.status = "INVALID_INPUT"
    out.message = "invalid exact KSS input"
    out.plugin = J(1,4,.)
    out.correction = J(1,4,.)
    out.corrected = J(1,4,.)
    out.numerical_mcse = J(1,4,0)
    out.n_stored = .
    out.n_physical = .
    out.worker_levels = .
    out.firm_levels = .
    out.parameters = .
    out.deletion_units = .
    out.target_weight_sum = .
    out.max_leverage = .
    out.information_rcond = .
    out.preconditioner_ratio = .
    out.control_schur_rcond = .
    out.deletion_rank_gap = .
    out.inverse_relres = .
    out.weighted_rss = .
    out.fit_seconds = .
    out.leverage_seconds = .
    out.target_seconds = .
    out.correction_seconds = .
    out.preconditioner_seconds = .
    out.solver_iterations = .
    out.solver_max_residual = .
    out.probes = .
    return(out)
}

struct kssbc_result scalar kssbc__failure(
    string scalar status,
    string scalar message)
{
    struct kssbc_result scalar out

    out = kssbc__empty_result()
    out.status = status
    out.message = message
    return(out)
}

struct kssbc_inverse_result scalar kssbc__inverse(
    real matrix information,
    real scalar rank_tolerance)
{
    struct kssbc_inverse_result scalar out
    real matrix scaled, scaled_inverse, residual
    real colvector inverse_scale, eigen
    real scalar dimension, largest, smallest

    out.status = "SINGULAR_INFORMATION"
    out.inverse = J(0,0,.)
    out.rcond = .
    out.relres = .
    dimension = rows(information)
    if (dimension == 0 | cols(information) != dimension |
        hasmissing(information)) return(out)
    information = 0.5 :* (information + information')
    if (min(diagonal(information)) <= 0) return(out)
    inverse_scale = 1 :/ sqrt(diagonal(information))
    scaled = (inverse_scale * inverse_scale') :* information
    eigen = Re(eigenvalues(0.5 :* (scaled + scaled')))
    if (hasmissing(eigen)) return(out)
    largest = max(eigen)
    smallest = min(eigen)
    if (largest <= 0 | smallest <= rank_tolerance * largest) {
        out.rcond = max((0, smallest / largest))
        return(out)
    }
    out.rcond = smallest / largest
    scaled_inverse = invsym(scaled)
    residual = scaled * scaled_inverse - I(dimension)
    out.relres = kssbc__max_column_relres(residual,I(dimension))
    if (hasmissing(scaled_inverse) | hasmissing(out.relres) |
        out.relres > max((1e-10, 100 * rank_tolerance))) {
        out.status = "INVERSE_RESIDUAL_FAILED"
        return(out)
    }
    out.inverse = (inverse_scale * inverse_scale') :* scaled_inverse
    if (hasmissing(out.inverse)) {
        out.status = "INVERSE_RESIDUAL_FAILED"
        return(out)
    }
    out.status = "CONVERGED"
    return(out)
}

real matrix kssbc__design(
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real scalar worker_levels,
    real scalar firm_levels)
{
    real scalar n, p, control_count, row
    real matrix design

    n = rows(worker)
    control_count = cols(controls)
    p = worker_levels + firm_levels - 1 + control_count
    design = J(n,p,0)
    for (row=1; row<=n; row++) {
        design[row,worker[row]] = 1
        if (firm[row] < firm_levels) {
            design[row,worker_levels+firm[row]] = 1
        }
    }
    if (control_count > 0) {
        design[.,(worker_levels+firm_levels)..p] = controls
    }
    return(design)
}

struct kssbc_target_matrices scalar kssbc__targets(
    real colvector worker,
    real colvector firm,
    real colvector target_weight,
    real scalar worker_levels,
    real scalar firm_levels,
    real scalar parameter_count)
{
    struct kssbc_target_matrices scalar out
    real scalar row, n, firm_parameters, firm_start, total
    real colvector worker_share, firm_share
    real matrix cell_share, cross

    n = rows(worker)
    total = sum(target_weight)
    firm_parameters = firm_levels - 1
    worker_share = J(worker_levels,1,0)
    firm_share = J(firm_parameters,1,0)
    cell_share = J(worker_levels,firm_parameters,0)
    for (row=1; row<=n; row++) {
        worker_share[worker[row]] = worker_share[worker[row]] +
            target_weight[row] / total
        if (firm[row] < firm_levels) {
            firm_share[firm[row]] = firm_share[firm[row]] +
                target_weight[row] / total
            cell_share[worker[row],firm[row]] =
                cell_share[worker[row],firm[row]] + target_weight[row] / total
        }
    }

    out.worker = J(parameter_count,parameter_count,0)
    out.firm = J(parameter_count,parameter_count,0)
    out.covariance = J(parameter_count,parameter_count,0)
    out.worker[|1,1 \ worker_levels,worker_levels|] =
        diag(worker_share) - worker_share * worker_share'
    if (firm_parameters > 0) {
        firm_start = worker_levels + 1
        out.firm[|firm_start,firm_start \
            worker_levels+firm_parameters,worker_levels+firm_parameters|] =
            diag(firm_share) - firm_share * firm_share'
        cross = cell_share - worker_share * firm_share'
        out.covariance[|1,firm_start \
            worker_levels,worker_levels+firm_parameters|] = 0.5 :* cross
        out.covariance[|firm_start,1 \
            worker_levels+firm_parameters,worker_levels|] = 0.5 :* cross'
    }
    return(out)
}

real scalar kssbc__quadratic(
    real colvector coefficient,
    real matrix target)
{
    return((coefficient' * target * coefficient)[1,1])
}

real colvector kssbc__target_diagonal(
    real matrix design_inverse,
    real matrix target)
{
    return(rowsum((design_inverse * target) :* design_inverse))
}

struct kssbc_result scalar kssbc__exact(
    real colvector y,
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector target_weight,
    real colvector deletion_id,
    string scalar deletion,
    string scalar nuisance,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar exact_limit,
    real scalar blocksize_limit)
{
    struct kssbc_result scalar out
    struct kssbc_inverse_result scalar full_inverse, working_inverse
    struct kssbc_inverse_result scalar deleted_information_inverse
    struct kssbc_target_matrices scalar targets
    real scalar n, worker_levels, firm_levels, controls_count
    real scalar full_parameters, parameters, group, groups, begin, finish
    real scalar max_leverage, eigmax, minimum_maker
    real scalar inverse_forward_bound, rank_verification_margin, row
    real matrix full_design, design, information, A, design_inverse
    real matrix deleted_information
    real matrix sorted_delete, panel, block_design, block_inverse, projection
    real matrix maker, bias_block
    real colvector full_beta, beta, working_y, residual, leverage_diagonal
    real colvector row_order, index, block_frequency, transformed_y
    real colvector transformed_residual, deleted_residual, eigen
    real rowvector plugin, correction

    out = kssbc__empty_result()
    n = rows(y)
    if (n == 0 | cols(y) != 1 | rows(worker) != n | cols(worker) != 1 |
        rows(firm) != n | cols(firm) != 1 | rows(frequency) != n |
        cols(frequency) != 1 | rows(target_weight) != n |
        cols(target_weight) != 1 | rows(deletion_id) != n |
        cols(deletion_id) != 1 | rows(controls) != n) {
        return(kssbc__failure("INVALID_INPUT", "KSS inputs have incompatible dimensions"))
    }
    if (hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(target_weight) | hasmissing(deletion_id)) {
        return(kssbc__failure("NONFINITE_INPUT", "KSS inputs must be finite"))
    }
    if (min(frequency) <= 0 |
        max(abs(frequency - floor(frequency))) != 0) {
        return(kssbc__failure("INVALID_FREQUENCY", "frequency weights must be positive integers"))
    }
    if (min(target_weight) < 0 | sum(target_weight) <= 0) {
        return(kssbc__failure("INVALID_TARGET_WEIGHT", "target weights must be nonnegative with positive mass"))
    }
    if (deletion != "observation" & deletion != "match") {
        return(kssbc__failure("UNSUPPORTED_DELETION", "deletion must be observation or match"))
    }
    if (nuisance != "joint" & nuisance != "fixedoffset") {
        return(kssbc__failure("INVALID_NUISANCE", "nuisance must be joint or fixedoffset"))
    }
    if (rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        block_tolerance <= 0 | block_tolerance >= 1) {
        return(kssbc__failure("INVALID_TOLERANCE", "invalid exact-solver tolerance"))
    }

    worker_levels = max(worker)
    firm_levels = max(firm)
    if (worker_levels < 1 | firm_levels < 2 |
        min(worker) != 1 | min(firm) != 1 |
        max(abs(worker-floor(worker))) != 0 |
        max(abs(firm-floor(firm))) != 0 |
        rows(uniqrows(sort(worker,1))) != worker_levels |
        rows(uniqrows(sort(firm,1))) != firm_levels) {
        return(kssbc__failure("INVALID_IDENTIFIER", "worker and firm IDs must be dense positive integers"))
    }
    controls_count = cols(controls)
    full_parameters = worker_levels + firm_levels - 1 + controls_count
    if (full_parameters > exact_limit) {
        return(kssbc__failure("EXACT_SIZE_LIMIT", "identified coefficient dimension exceeds exact_limit()"))
    }
    timer_clear(91)
    timer_clear(92)
    timer_on(91)

    if (deletion == "match") {
        row_order = order(deletion_id,1)
        sorted_delete = deletion_id[row_order]
        panel = panelsetup(sorted_delete,1)
        groups = rows(panel)
        for (group=1; group<=groups; group++) {
            begin = panel[group,1]
            finish = panel[group,2]
            index = row_order[|begin \ finish|]
            if (rows(index) > blocksize_limit) {
                return(kssbc__failure("BLOCK_SIZE_LIMIT", "a deletion block exceeds blocksize_limit()"))
            }
            if (min(worker[index]) != max(worker[index]) |
                min(firm[index]) != max(firm[index])) {
                return(kssbc__failure("CROSS_COORDINATE_MATCH", "each deletion ID must remain within one worker-firm coordinate"))
            }
        }
    }
    else {
        groups = sum(frequency)
    }

    full_design = kssbc__design(
        worker, firm, controls, worker_levels, firm_levels)
    information = full_design' * (frequency :* full_design)
    full_inverse = kssbc__inverse(information,rank_tolerance)
    if (full_inverse.status != "CONVERGED") {
        if (full_inverse.status == "SINGULAR_INFORMATION") {
            return(kssbc__failure("SINGULAR_INFORMATION", "full weighted design is unidentified or disconnected"))
        }
        return(kssbc__failure(full_inverse.status, "full weighted inverse failed its residual gate"))
    }
    full_beta = full_inverse.inverse * (full_design' * (frequency :* y))
    if (hasmissing(full_beta)) {
        return(kssbc__failure("NONFINITE_FIT", "full weighted least-squares fit is nonfinite"))
    }

    if (nuisance == "fixedoffset" & controls_count > 0) {
        working_y = y - controls *
            full_beta[(full_parameters-controls_count+1)..full_parameters]
        parameters = worker_levels + firm_levels - 1
        design = full_design[.,1..parameters]
        information = design' * (frequency :* design)
        working_inverse = kssbc__inverse(information,rank_tolerance)
        if (working_inverse.status != "CONVERGED") {
            return(kssbc__failure(working_inverse.status, "fixed-offset two-way inverse failed"))
        }
    }
    else {
        working_y = y
        design = full_design
        parameters = full_parameters
        working_inverse = full_inverse
    }
    A = working_inverse.inverse
    inverse_forward_bound = working_inverse.relres /
        max((working_inverse.rcond,rank_tolerance))
    if (hasmissing(inverse_forward_bound) | inverse_forward_bound >= 0.01) {
        return(kssbc__failure("INVERSE_FORWARD_ERROR_FAILED", "working information inverse is too ill-conditioned for a fail-closed deletion-rank gate"))
    }
    rank_verification_margin = max((block_tolerance,
        10*inverse_forward_bound))
    beta = A * (design' * (frequency :* working_y))
    residual = working_y - design * beta
    if (hasmissing(beta) | hasmissing(residual)) {
        return(kssbc__failure("NONFINITE_FIT", "working least-squares fit is nonfinite"))
    }

    targets = kssbc__targets(
        worker, firm, target_weight, worker_levels, firm_levels, parameters)
    plugin = J(1,4,0)
    plugin[1] = kssbc__quadratic(beta,targets.worker)
    plugin[2] = kssbc__quadratic(beta,targets.firm)
    plugin[3] = kssbc__quadratic(beta,targets.covariance)
    plugin[4] = plugin[1] + plugin[2] + 2 * plugin[3]
    correction = J(1,4,0)
    max_leverage = 0
    design_inverse = design * A
    timer_off(91)
    timer_on(92)

    if (deletion == "observation") {
        leverage_diagonal = rowsum(design_inverse :* design)
        if (min(leverage_diagonal) < -100 * rank_tolerance) {
            return(kssbc__failure("NONESTIMABLE_DELETION", "physical observation deletion has leverage at or above one"))
        }
        for (row=1; row<=n; row++) {
            minimum_maker = 1-leverage_diagonal[row]
            if (minimum_maker <= block_tolerance) {
                return(kssbc__failure("NONESTIMABLE_DELETION", "physical observation deletion has leverage at or above one"))
            }
            if (minimum_maker <= rank_verification_margin) {
                deleted_information = information -
                    design[row,.]'*design[row,.]
                deleted_information_inverse = kssbc__inverse(
                    deleted_information,rank_tolerance)
                if (deleted_information_inverse.status != "CONVERGED") {
                    return(kssbc__failure("NONESTIMABLE_DELETION", "a direct deleted-information factorization rejects physical observation deletion"))
                }
            }
        }
        max_leverage = max(leverage_diagonal)
        correction[1] = sum(frequency :* working_y :* residual :*
            kssbc__target_diagonal(design_inverse,targets.worker) :/
            (1 :- leverage_diagonal))
        correction[2] = sum(frequency :* working_y :* residual :*
            kssbc__target_diagonal(design_inverse,targets.firm) :/
            (1 :- leverage_diagonal))
        correction[3] = sum(frequency :* working_y :* residual :*
            kssbc__target_diagonal(design_inverse,targets.covariance) :/
            (1 :- leverage_diagonal))
    }
    else {
        row_order = order(deletion_id,1)
        sorted_delete = deletion_id[row_order]
        panel = panelsetup(sorted_delete,1)
        for (group=1; group<=rows(panel); group++) {
            begin = panel[group,1]
            finish = panel[group,2]
            index = row_order[|begin \ finish|]
            block_frequency = sqrt(frequency[index])
            block_design = block_frequency :* design[index,.]
            block_inverse = block_design * A
            projection = block_inverse * block_design'
            projection = 0.5 :* (projection + projection')
            eigen = Re(eigenvalues(projection))
            if (hasmissing(eigen)) {
                return(kssbc__failure("NONFINITE_LEVERAGE", "block projection eigenvalue calculation failed"))
            }
            eigmax = max(eigen)
            minimum_maker = 1 - eigmax
            if (minimum_maker <= block_tolerance) {
                return(kssbc__failure("NONESTIMABLE_DELETION", "a match deletion loses identified design rank"))
            }
            if (minimum_maker <= rank_verification_margin) {
                deleted_information = information -
                    block_design'*block_design
                deleted_information_inverse = kssbc__inverse(
                    deleted_information,rank_tolerance)
                if (deleted_information_inverse.status != "CONVERGED") {
                    return(kssbc__failure("NONESTIMABLE_DELETION", "a direct deleted-information factorization rejects match deletion"))
                }
            }
            max_leverage = max((max_leverage,eigmax))
            maker = I(rows(index)) - projection
            transformed_y = block_frequency :* working_y[index]
            transformed_residual = block_frequency :* residual[index]
            deleted_residual = invsym(maker) * transformed_residual
            if (hasmissing(deleted_residual) |
                kssbc__norm2(maker*deleted_residual-transformed_residual) >
                100 * rank_tolerance * (1+kssbc__norm2(transformed_residual))) {
                return(kssbc__failure("BLOCK_INVERSE_FAILED", "a block residual solve failed its residual gate"))
            }
            bias_block = block_inverse * targets.worker * block_inverse'
            correction[1] = correction[1] +
                (transformed_y' * bias_block * deleted_residual)[1,1]
            bias_block = block_inverse * targets.firm * block_inverse'
            correction[2] = correction[2] +
                (transformed_y' * bias_block * deleted_residual)[1,1]
            bias_block = block_inverse * targets.covariance * block_inverse'
            correction[3] = correction[3] +
                (transformed_y' * bias_block * deleted_residual)[1,1]
        }
    }
    correction[4] = correction[1] + correction[2] + 2 * correction[3]
    timer_off(92)
    if (hasmissing(plugin) | hasmissing(correction)) {
        return(kssbc__failure("NONFINITE_CORRECTION", "exact KSS correction is nonfinite"))
    }

    out.status = "CONVERGED"
    out.message = "exact KSS calculation converged"
    out.plugin = plugin
    out.correction = correction
    out.corrected = plugin - correction
    out.numerical_mcse = J(1,4,0)
    out.n_stored = n
    out.n_physical = sum(frequency)
    out.worker_levels = worker_levels
    out.firm_levels = firm_levels
    out.parameters = parameters
    out.deletion_units = groups
    out.target_weight_sum = sum(target_weight)
    out.max_leverage = max_leverage
    out.information_rcond = min((full_inverse.rcond,working_inverse.rcond))
    out.preconditioner_ratio = .
    out.control_schur_rcond = .
    out.inverse_relres = max((full_inverse.relres,working_inverse.relres))
    out.weighted_rss = sum(frequency:*residual:^2)
    out.fit_seconds = kssbc__timer_seconds(91)
    out.leverage_seconds = 0
    out.target_seconds = 0
    out.correction_seconds = kssbc__timer_seconds(92)
    out.preconditioner_seconds = 0
    out.solver_iterations = 0
    out.solver_max_residual = out.inverse_relres
    out.probes = 0
    return(out)
}

struct kssbc_fe_design
{
    string scalar status
    string scalar message
    real scalar n
    real scalar worker_levels
    real scalar firm_levels
    real colvector worker
    real colvector firm
    real colvector frequency
    real colvector worker_order
    real colvector firm_order
    real matrix worker_panel
    real matrix firm_panel
    real colvector worker_weight
    real colvector firm_weight
    real colvector schur_diagonal
    real scalar preconditioner_ratio
}

struct kssbc_solve_result
{
    string scalar status
    string scalar message
    real matrix coefficient
    real scalar iterations
    real scalar relres
}

struct kssbc_joint_design
{
    string scalar status
    string scalar message
    struct kssbc_fe_design scalar base
    real matrix controls
    real matrix cross
    real matrix base_cross_inverse
    real matrix residualized_controls
    real matrix schur_inverse
    real scalar schur_rcond
    real scalar preparation_relres
    real scalar preparation_iterations
}

real matrix kssbc__group_sum(
    real matrix values,
    real colvector row_order,
    real matrix panel)
{
    return(panelsum(values[row_order,.],panel))
}

struct kssbc_fe_design scalar kssbc__fe_prepare(
    real colvector worker,
    real colvector firm,
    real colvector frequency,
    real scalar rank_tolerance)
{
    struct kssbc_fe_design scalar out
    real colvector pair_order, pair_first, pair_worker, pair_firm, pair_code
    real colvector pair_weight, adjustment, firm_pair_order
    real matrix pair_panel, firm_pair_panel
    real scalar worker_levels, firm_levels, row

    out.status = "INVALID_INPUT"
    out.message = "invalid two-way design"
    out.n = rows(worker)
    out.worker_levels = .
    out.firm_levels = .
    out.worker = worker
    out.firm = firm
    out.frequency = frequency
    out.worker_order = J(0,1,.)
    out.firm_order = J(0,1,.)
    out.worker_panel = J(0,2,.)
    out.firm_panel = J(0,2,.)
    out.worker_weight = J(0,1,.)
    out.firm_weight = J(0,1,.)
    out.schur_diagonal = J(0,1,.)
    out.preconditioner_ratio = .

    if (rows(worker) == 0 | rows(firm) != rows(worker) |
        rows(frequency) != rows(worker) | hasmissing(worker) |
        hasmissing(firm) | hasmissing(frequency) | min(frequency) <= 0) {
        return(out)
    }
    worker_levels = max(worker)
    firm_levels = max(firm)
    if (worker_levels < 1 | firm_levels < 2 | min(worker) != 1 |
        min(firm) != 1 | rows(uniqrows(sort(worker,1))) != worker_levels |
        rows(uniqrows(sort(firm,1))) != firm_levels) {
        out.status = "INVALID_IDENTIFIER"
        out.message = "two-way identifiers must be dense positive integers"
        return(out)
    }
    out.worker_levels = worker_levels
    out.firm_levels = firm_levels
    out.worker_order = order(worker,1)
    out.firm_order = order(firm,1)
    out.worker_panel = panelsetup(worker[out.worker_order],1)
    out.firm_panel = panelsetup(firm[out.firm_order],1)
    out.worker_weight = kssbc__group_sum(
        frequency,out.worker_order,out.worker_panel)
    out.firm_weight = kssbc__group_sum(
        frequency,out.firm_order,out.firm_panel)

    pair_order = order((worker,firm),(1,2))
    pair_code = J(rows(worker),1,1)
    for (row=2; row<=rows(worker); row++) {
        pair_code[row] = pair_code[row-1] +
            (worker[pair_order[row]] != worker[pair_order[row-1]] |
            firm[pair_order[row]] != firm[pair_order[row-1]])
    }
    pair_panel = panelsetup(pair_code,1)
    pair_weight = panelsum(frequency[pair_order],pair_panel)
    pair_first = pair_order[pair_panel[.,1]]
    pair_worker = worker[pair_first]
    pair_firm = firm[pair_first]
    firm_pair_order = order(pair_firm,1)
    firm_pair_panel = panelsetup(pair_firm[firm_pair_order],1)
    adjustment = panelsum(
        ((pair_weight:^2) :/ out.worker_weight[pair_worker])[firm_pair_order],
        firm_pair_panel)
    out.schur_diagonal =
        out.firm_weight[1..(firm_levels-1)] - adjustment[1..(firm_levels-1)]
    if (hasmissing(out.schur_diagonal) | min(out.schur_diagonal) <= 0) {
        out.status = "SINGULAR_INFORMATION"
        out.message = "grounded firm mobility system has a nonpositive diagonal"
        return(out)
    }
    out.preconditioner_ratio = min(out.schur_diagonal) /
        max(out.schur_diagonal)
    out.status = "CONVERGED"
    out.message = "two-way matrix-free design prepared"
    return(out)
}

real matrix kssbc__fe_predict(
    struct kssbc_fe_design scalar design,
    real matrix coefficient)
{
    real matrix firm_coefficient, fitted
    real scalar columns

    columns = cols(coefficient)
    firm_coefficient = J(design.firm_levels,columns,0)
    firm_coefficient[1..(design.firm_levels-1),.] =
        coefficient[(design.worker_levels+1)..rows(coefficient),.]
    fitted = coefficient[1..design.worker_levels,.][design.worker,.] +
        firm_coefficient[design.firm,.]
    return(fitted)
}

real matrix kssbc__fe_transpose(
    struct kssbc_fe_design scalar design,
    real matrix values)
{
    real matrix worker_part, firm_part

    worker_part = kssbc__group_sum(
        values,design.worker_order,design.worker_panel)
    firm_part = kssbc__group_sum(
        values,design.firm_order,design.firm_panel)
    return(worker_part \ firm_part[1..(design.firm_levels-1),.])
}

real colvector kssbc__fe_schur_action(
    struct kssbc_fe_design scalar design,
    real colvector firm_coefficient)
{
    real colvector extended, fitted, worker_mean, residual, firm_sum

    extended = firm_coefficient \ 0
    fitted = extended[design.firm]
    worker_mean = kssbc__group_sum(
        design.frequency :* fitted,
        design.worker_order,design.worker_panel) :/ design.worker_weight
    residual = fitted - worker_mean[design.worker]
    firm_sum = kssbc__group_sum(
        design.frequency :* residual,
        design.firm_order,design.firm_panel)
    return(firm_sum[1..(design.firm_levels-1)])
}

struct kssbc_solve_result scalar kssbc__fe_solve(
    struct kssbc_fe_design scalar design,
    real colvector right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct kssbc_solve_result scalar out
    real scalar workers, firms, iteration, denominator, rz, rz_new
    real scalar reduced_scale, full_scale
    real colvector worker_rhs, firm_rhs, worker_base, reduced_rhs
    real colvector firm_coefficient, residual, preconditioned, direction
    real colvector action, worker_coefficient, fitted, full_residual

    out.status = "INVALID_INPUT"
    out.message = "invalid matrix-free right-hand side"
    out.coefficient = J(0,1,.)
    out.iterations = .
    out.relres = .
    workers = design.worker_levels
    firms = design.firm_levels
    if (design.status != "CONVERGED" |
        rows(right_hand_side) != workers+firms-1 |
        cols(right_hand_side) != 1 | hasmissing(right_hand_side)) return(out)

    worker_rhs = right_hand_side[1..workers]
    firm_rhs = right_hand_side[(workers+1)..rows(right_hand_side)]
    worker_base = worker_rhs :/ design.worker_weight
    reduced_rhs = firm_rhs - kssbc__group_sum(
        design.frequency :* worker_base[design.worker],
        design.firm_order,design.firm_panel)[1..(firms-1)]
    reduced_scale = kssbc__norm2(reduced_rhs)
    firm_coefficient = J(firms-1,1,0)
    if (reduced_scale == 0) {
        iteration = 0
    }
    else {
        residual = reduced_rhs
        preconditioned = residual :/ design.schur_diagonal
        direction = preconditioned
        rz = (residual' * preconditioned)[1,1]
        for (iteration=1; iteration<=maxiter; iteration++) {
            action = kssbc__fe_schur_action(design,direction)
            denominator = (direction' * action)[1,1]
            if (denominator <= 0 | denominator >= . | rz <= 0 | rz >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "firm mobility PCG lost positive curvature"
                return(out)
            }
            firm_coefficient = firm_coefficient + (rz/denominator) :* direction
            residual = residual - (rz/denominator) :* action
            if (kssbc__norm2(residual) <=
                tolerance * (1+reduced_scale)) break
            preconditioned = residual :/ design.schur_diagonal
            rz_new = (residual' * preconditioned)[1,1]
            if (rz_new <= 0 | rz_new >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "firm mobility PCG residual became nonpositive"
                return(out)
            }
            direction = preconditioned + (rz_new/rz) :* direction
            rz = rz_new
        }
        if (iteration > maxiter) {
            out.status = "PCG_NONCONVERGENCE"
            out.message = "firm mobility PCG exceeded maxiter()"
            return(out)
        }
    }
    fitted = (firm_coefficient \ 0)[design.firm]
    worker_coefficient = worker_base - kssbc__group_sum(
        design.frequency :* fitted,
        design.worker_order,design.worker_panel) :/ design.worker_weight
    out.coefficient = worker_coefficient \ firm_coefficient
    fitted = kssbc__fe_predict(design,out.coefficient)
    full_residual = kssbc__fe_transpose(
        design,design.frequency :* fitted) - right_hand_side
    full_scale = kssbc__norm2(right_hand_side)
    out.relres = kssbc__norm2(full_residual) / (1+full_scale)
    if (hasmissing(out.coefficient) | hasmissing(out.relres) |
        out.relres > max((1e-11,10*tolerance))) {
        out.status = "SOLVER_RESIDUAL_FAILED"
        out.message = "recomputed full two-way residual exceeds tolerance"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "matrix-free two-way solve converged"
    out.iterations = iteration
    return(out)
}

struct kssbc_solve_result scalar kssbc__fe_solve_matrix(
    struct kssbc_fe_design scalar design,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct kssbc_solve_result scalar out, one
    real scalar column

    out.status = "CONVERGED"
    out.message = "batched two-way solves converged"
    out.coefficient = J(rows(right_hand_side),cols(right_hand_side),.)
    out.iterations = 0
    out.relres = 0
    for (column=1; column<=cols(right_hand_side); column++) {
        one = kssbc__fe_solve(
            design,right_hand_side[.,column],tolerance,maxiter)
        if (one.status != "CONVERGED") return(one)
        out.coefficient[.,column] = one.coefficient
        out.iterations = max((out.iterations,one.iterations))
        out.relres = max((out.relres,one.relres))
    }
    return(out)
}

struct kssbc_joint_design scalar kssbc__joint_prepare(
    struct kssbc_fe_design scalar base,
    real matrix controls,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance)
{
    struct kssbc_joint_design scalar out
    struct kssbc_solve_result scalar solved
    struct kssbc_inverse_result scalar small_inverse
    real matrix weighted_controls, schur, checked_schur

    out.status = "INVALID_INPUT"
    out.message = "invalid joint-control design"
    out.base = base
    out.controls = controls
    out.cross = J(base.worker_levels+base.firm_levels-1,0,.)
    out.base_cross_inverse = J(base.worker_levels+base.firm_levels-1,0,.)
    out.residualized_controls = J(base.n,0,.)
    out.schur_inverse = J(0,0,.)
    out.schur_rcond = .
    out.preparation_relres = 0
    out.preparation_iterations = 0
    if (base.status != "CONVERGED" | rows(controls) != base.n |
        hasmissing(controls)) return(out)
    if (cols(controls) == 0) {
        out.status = "CONVERGED"
        out.message = "two-way design has no joint controls"
        return(out)
    }
    weighted_controls = base.frequency :* controls
    out.cross = kssbc__fe_transpose(base,weighted_controls)
    solved = kssbc__fe_solve_matrix(
        base,out.cross,tolerance,maxiter)
    if (solved.status != "CONVERGED") {
        out.status = solved.status
        out.message = solved.message
        return(out)
    }
    out.base_cross_inverse = solved.coefficient
    out.residualized_controls = controls -
        kssbc__fe_predict(base,out.base_cross_inverse)
    schur = controls' * weighted_controls -
        out.cross' * out.base_cross_inverse
    small_inverse = kssbc__inverse(schur,rank_tolerance)
    if (small_inverse.status != "CONVERGED") {
        out.status = "SINGULAR_NUISANCE_BLOCK"
        out.message = "residualized joint-control block is singular"
        return(out)
    }
    checked_schur = out.residualized_controls' *
        (base.frequency :* out.residualized_controls)
    out.preparation_relres = kssbc__max_column_relres(
        schur-checked_schur,checked_schur)
    if (out.preparation_relres > max((1e-11,10*tolerance))) {
        out.status = "CONTROL_SCHUR_RESIDUAL_FAILED"
        out.message = "joint-control Schur complement failed its residual gate"
        return(out)
    }
    out.schur_inverse = small_inverse.inverse
    out.schur_rcond = small_inverse.rcond
    out.preparation_relres = max((out.preparation_relres,small_inverse.relres,
        solved.relres))
    out.preparation_iterations = solved.iterations
    out.status = "CONVERGED"
    out.message = "joint-control Schur complement prepared"
    return(out)
}

real matrix kssbc__joint_predict(
    struct kssbc_joint_design scalar design,
    real matrix coefficient)
{
    real scalar base_parameters

    base_parameters = design.base.worker_levels + design.base.firm_levels - 1
    if (cols(design.controls) == 0) {
        return(kssbc__fe_predict(design.base,coefficient))
    }
    return(kssbc__fe_predict(
        design.base,coefficient[1..base_parameters,.]) +
        design.controls * coefficient[(base_parameters+1)..rows(coefficient),.])
}

real matrix kssbc__joint_transpose(
    struct kssbc_joint_design scalar design,
    real matrix values)
{
    if (cols(design.controls) == 0) {
        return(kssbc__fe_transpose(design.base,values))
    }
    return(kssbc__fe_transpose(design.base,values) \
        design.controls' * values)
}

struct kssbc_solve_result scalar kssbc__joint_solve(
    struct kssbc_joint_design scalar design,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct kssbc_solve_result scalar out, base_solved
    real scalar base_parameters, control_count, column
    real matrix base_rhs, control_rhs, gamma, base_coefficient
    real matrix fitted, residual

    out.status = "INVALID_INPUT"
    out.message = "invalid joint-system right-hand side"
    out.coefficient = J(0,0,.)
    out.iterations = .
    out.relres = .
    if (design.status != "CONVERGED") return(out)
    base_parameters = design.base.worker_levels + design.base.firm_levels - 1
    control_count = cols(design.controls)
    if (rows(right_hand_side) != base_parameters+control_count |
        hasmissing(right_hand_side)) return(out)
    base_rhs = right_hand_side[1..base_parameters,.]
    base_solved = kssbc__fe_solve_matrix(
        design.base,base_rhs,tolerance,maxiter)
    if (base_solved.status != "CONVERGED") return(base_solved)
    if (control_count == 0) return(base_solved)
    control_rhs = right_hand_side[(base_parameters+1)..rows(right_hand_side),.]
    gamma = design.schur_inverse *
        (control_rhs - design.cross' * base_solved.coefficient)
    base_coefficient = base_solved.coefficient -
        design.base_cross_inverse * gamma
    out.coefficient = base_coefficient \ gamma
    fitted = kssbc__joint_predict(design,out.coefficient)
    residual = kssbc__joint_transpose(
        design,design.base.frequency :* fitted) - right_hand_side
    out.relres = kssbc__max_column_relres(residual,right_hand_side)
    if (hasmissing(out.coefficient) | hasmissing(out.relres) |
        out.relres > max((1e-11,10*tolerance))) {
        out.status = "SOLVER_RESIDUAL_FAILED"
        out.message = "recomputed full joint-system residual exceeds tolerance"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "matrix-free joint-system solve converged"
    out.iterations = base_solved.iterations
    return(out)
}

real matrix kssbc__physical_panels(real colvector frequency)
{
    real matrix panel
    real scalar row, begin

    panel = J(rows(frequency),2,.)
    begin = 1
    for (row=1; row<=rows(frequency); row++) {
        panel[row,1] = begin
        panel[row,2] = begin+frequency[row]-1
        begin = panel[row,2]+1
    }
    return(panel)
}

real colvector kssbc__physical_rademacher_sum(real colvector frequency)
{
    real colvector out, physical_random
    real matrix physical_panel

    physical_panel = kssbc__physical_panels(frequency)
    physical_random = 2:*rbinomial(sum(frequency),1,1,0.5):-1
    out = panelsum(physical_random,physical_panel)
    return(out)
}

real colvector kssbc__target_direction(
    real colvector frequency,
    real colvector target_weight,
    real colvector rademacher_sum)
{
    real scalar total, centered_scalar
    real colvector share, square_share, first

    total = sum(target_weight)
    share = target_weight :/ total
    square_share = sqrt(target_weight :/ (frequency:*total))
    first = square_share :* rademacher_sum
    centered_scalar = sum(first)
    return(first - share :* centered_scalar)
}

real rowvector kssbc__effect_plugin(
    real colvector coefficient,
    struct kssbc_fe_design scalar base,
    real colvector target_weight)
{
    real colvector worker_effect, firm_coefficient, firm_effect, total_effect
    real scalar mass, worker_mean, firm_mean, worker_variance, firm_variance
    real scalar covariance

    mass = sum(target_weight)
    worker_effect = coefficient[1..base.worker_levels][base.worker]
    firm_coefficient = coefficient[(base.worker_levels+1)..
        (base.worker_levels+base.firm_levels-1)] \ 0
    firm_effect = firm_coefficient[base.firm]
    worker_mean = sum(target_weight:*worker_effect)/mass
    firm_mean = sum(target_weight:*firm_effect)/mass
    worker_variance = sum(target_weight:*(worker_effect:-worker_mean):^2)/mass
    firm_variance = sum(target_weight:*(firm_effect:-firm_mean):^2)/mass
    covariance = sum(target_weight:*(worker_effect:-worker_mean):*
        (firm_effect:-firm_mean))/mass
    total_effect = worker_effect + firm_effect
    return((worker_variance,firm_variance,covariance,
        worker_variance+firm_variance+2*covariance))
}

real rowvector kssbc__mcse(real matrix draws)
{
    real rowvector mean_draw

    if (rows(draws) < 2) return(J(1,cols(draws),.))
    mean_draw = colsum(draws) :/ rows(draws)
    return(sqrt(colsum((draws :- mean_draw):^2) :/
        (rows(draws)-1) :/ rows(draws)))
}

real scalar kssbc__union_find_root(
    real colvector parent,
    real scalar node)
{
    while (parent[node] != node) node = parent[node]
    return(node)
}

struct kssbc_component_result
{
    real colvector keep
    real scalar components
    real scalar edges
    real scalar firms
    real scalar physical_mass
    real scalar ambiguous
}

struct kssbc_articulation_result
{
    real colvector bad_worker
    real scalar count
}

struct kssbc_component_result scalar kssbc__largest_component(
    real colvector worker,
    real colvector firm,
    real colvector frequency,
    real colvector active)
{
    struct kssbc_component_result scalar out
    real colvector selected, edge_order, sorted_index, edge_code, edge_first
    real colvector edge_worker, edge_firm, parent, union_size, degree
    real colvector firm_count, component_mass
    real scalar n, workers, firms, nodes, row, edge, node, other
    real scalar root_node, root_other, best_root, best_firms, best_mass

    n = rows(worker)
    out.keep = J(n,1,0)
    out.components = 0
    out.edges = 0
    out.firms = 0
    out.physical_mass = 0
    out.ambiguous = 0
    selected = selectindex(active :== 1)
    if (rows(selected) == 0) return(out)
    workers = max(worker)
    firms = max(firm)
    nodes = workers+firms

    edge_order = order((worker[selected],firm[selected]),(1,2))
    sorted_index = selected[edge_order]
    edge_code = J(rows(selected),1,1)
    for (row=2; row<=rows(selected); row++) {
        edge_code[row] = edge_code[row-1] +
            (worker[sorted_index[row]] != worker[sorted_index[row-1]] |
            firm[sorted_index[row]] != firm[sorted_index[row-1]])
    }
    edge_first = sorted_index[panelsetup(edge_code,1)[.,1]]
    edge_worker = worker[edge_first]
    edge_firm = firm[edge_first]
    out.edges = rows(edge_first)

    parent = 1::nodes
    union_size = J(nodes,1,1)
    degree = J(nodes,1,0)
    for (edge=1; edge<=out.edges; edge++) {
        node = edge_worker[edge]
        other = workers+edge_firm[edge]
        degree[node] = degree[node]+1
        degree[other] = degree[other]+1
        root_node = kssbc__union_find_root(parent,node)
        root_other = kssbc__union_find_root(parent,other)
        if (root_node == root_other) continue
        if (union_size[root_node] < union_size[root_other]) {
            row = root_node
            root_node = root_other
            root_other = row
        }
        parent[root_other] = root_node
        union_size[root_node] = union_size[root_node]+union_size[root_other]
    }

    firm_count = J(nodes,1,0)
    component_mass = J(nodes,1,0)
    for (row=1; row<=firms; row++) {
        node = workers+row
        if (degree[node] == 0) continue
        root_node = kssbc__union_find_root(parent,node)
        firm_count[root_node] = firm_count[root_node]+1
    }
    for (row=1; row<=rows(selected); row++) {
        node = selected[row]
        root_node = kssbc__union_find_root(parent,worker[node])
        component_mass[root_node] = component_mass[root_node]+frequency[node]
    }

    best_root = 0
    best_firms = -1
    best_mass = -1
    for (node=1; node<=nodes; node++) {
        if (firm_count[node] <= 0) continue
        out.components = out.components+1
        if (firm_count[node] > best_firms |
            (firm_count[node] == best_firms &
            component_mass[node] > best_mass)) {
            best_root = node
            best_firms = firm_count[node]
            best_mass = component_mass[node]
            out.ambiguous = 0
        }
        else if (firm_count[node] == best_firms &
            component_mass[node] == best_mass) {
            out.ambiguous = 1
        }
    }
    if (best_root == 0) return(out)
    for (row=1; row<=rows(selected); row++) {
        node = selected[row]
        if (kssbc__union_find_root(parent,worker[node]) == best_root) {
            out.keep[node] = 1
        }
    }
    out.firms = best_firms
    out.physical_mass = best_mass
    return(out)
}

real colvector kssbc__worker_firm_counts(
    real colvector worker,
    real colvector firm,
    real colvector active)
{
    real colvector selected, pair_order, sorted_index, pair_code, pair_first
    real colvector counts
    real scalar row

    counts = J(max(worker),1,0)
    selected = selectindex(active :== 1)
    if (rows(selected) == 0) return(counts)
    pair_order = order((worker[selected],firm[selected]),(1,2))
    sorted_index = selected[pair_order]
    pair_code = J(rows(selected),1,1)
    for (row=2; row<=rows(selected); row++) {
        pair_code[row] = pair_code[row-1] +
            (worker[sorted_index[row]] != worker[sorted_index[row-1]] |
            firm[sorted_index[row]] != firm[sorted_index[row-1]])
    }
    pair_first = sorted_index[panelsetup(pair_code,1)[.,1]]
    for (row=1; row<=rows(pair_first); row++) {
        counts[worker[pair_first[row]]] =
            counts[worker[pair_first[row]]]+1
    }
    return(counts)
}

struct kssbc_articulation_result scalar kssbc__worker_articulations(
    real colvector worker,
    real colvector firm,
    real colvector active)
{
    struct kssbc_articulation_result scalar out
    real colvector selected, edge_order, sorted_index, edge_code, edge_first
    real colvector edge_worker, edge_firm, degree, adjacency_start
    real colvector adjacency_finish, cursor, neighbor, adjacency_edge
    real colvector discovery, low, dfs_parent, parent_edge, next_arc
    real colvector child_count, stack, articulation
    real scalar workers, firms, nodes, edges, row, edge, node, other
    real scalar arc, clock, top, root, parent

    workers = max(worker)
    firms = max(firm)
    nodes = workers+firms
    out.bad_worker = J(workers,1,0)
    out.count = 0
    selected = selectindex(active :== 1)
    if (rows(selected) == 0) return(out)

    edge_order = order((worker[selected],firm[selected]),(1,2))
    sorted_index = selected[edge_order]
    edge_code = J(rows(selected),1,1)
    for (row=2; row<=rows(selected); row++) {
        edge_code[row] = edge_code[row-1] +
            (worker[sorted_index[row]] != worker[sorted_index[row-1]] |
            firm[sorted_index[row]] != firm[sorted_index[row-1]])
    }
    edge_first = sorted_index[panelsetup(edge_code,1)[.,1]]
    edge_worker = worker[edge_first]
    edge_firm = firm[edge_first]
    edges = rows(edge_first)

    degree = J(nodes,1,0)
    for (edge=1; edge<=edges; edge++) {
        degree[edge_worker[edge]] = degree[edge_worker[edge]]+1
        node = workers+edge_firm[edge]
        degree[node] = degree[node]+1
    }
    adjacency_start = J(nodes,1,.)
    cursor = J(nodes,1,.)
    row = 1
    for (node=1; node<=nodes; node++) {
        adjacency_start[node] = row
        cursor[node] = row
        row = row+degree[node]
    }
    adjacency_finish = adjacency_start+degree:-1
    neighbor = J(2*edges,1,.)
    adjacency_edge = J(2*edges,1,.)
    for (edge=1; edge<=edges; edge++) {
        node = edge_worker[edge]
        other = workers+edge_firm[edge]
        arc = cursor[node]
        neighbor[arc] = other
        adjacency_edge[arc] = edge
        cursor[node] = cursor[node]+1
        arc = cursor[other]
        neighbor[arc] = node
        adjacency_edge[arc] = edge
        cursor[other] = cursor[other]+1
    }

    discovery = J(nodes,1,0)
    low = J(nodes,1,0)
    dfs_parent = J(nodes,1,0)
    parent_edge = J(nodes,1,0)
    next_arc = adjacency_start
    child_count = J(nodes,1,0)
    articulation = J(nodes,1,0)
    stack = J(nodes,1,0)
    clock = 0
    for (root=1; root<=nodes; root++) {
        if (degree[root] == 0 | discovery[root] != 0) continue
        top = 1
        stack[top] = root
        clock = clock+1
        discovery[root] = clock
        low[root] = clock
        while (top > 0) {
            node = stack[top]
            if (next_arc[node] <= adjacency_finish[node]) {
                arc = next_arc[node]
                next_arc[node] = next_arc[node]+1
                edge = adjacency_edge[arc]
                other = neighbor[arc]
                if (edge == parent_edge[node]) continue
                if (discovery[other] == 0) {
                    dfs_parent[other] = node
                    parent_edge[other] = edge
                    child_count[node] = child_count[node]+1
                    clock = clock+1
                    discovery[other] = clock
                    low[other] = clock
                    top = top+1
                    stack[top] = other
                }
                else low[node] = min((low[node],discovery[other]))
            }
            else {
                top = top-1
                parent = dfs_parent[node]
                if (parent == 0) {
                    if (child_count[node] > 1) articulation[node] = 1
                }
                else {
                    if (dfs_parent[parent] != 0 &
                        low[node] >= discovery[parent]) {
                        articulation[parent] = 1
                    }
                    low[parent] = min((low[parent],low[node]))
                }
            }
        }
    }
    out.bad_worker = articulation[1..workers]
    out.count = sum(out.bad_worker)
    return(out)
}

void kssbc__stata_prune_graph(
    string scalar worker_name,
    string scalar firm_name,
    string scalar frequency_name,
    string scalar deletion_name,
    string scalar sample_name,
    string scalar deletion,
    string scalar keep_name,
    string scalar diagnostics_name,
    string scalar status_local,
    string scalar message_local)
{
    struct kssbc_component_result scalar component
    struct kssbc_articulation_result scalar articulation
    real colvector worker, firm, frequency, deletion_id, sample, sample_index
    real colvector deletion_order, deletion_sorted, index, active
    real colvector worker_firms, worker_physical, diagnostics
    real matrix deletion_panel
    real scalar n, workers, group, begin, finish, row, removed
    real scalar initial_components, initial_component_rows, mover_input_rows
    real scalar graph_edges, articulation_removed, insufficient_removed
    real scalar iterations, maximum_components, retained_mass, retained_rows
    real scalar graph_seconds

    sample = st_data(.,sample_name)
    sample_index = selectindex(sample :== 1)
    worker = st_data(sample_index,worker_name)
    firm = st_data(sample_index,firm_name)
    frequency = st_data(sample_index,frequency_name)
    deletion_id = st_data(sample_index,deletion_name)
    n = rows(worker)
    st_local(status_local,"INVALID_GRAPH_INPUT")
    st_local(message_local,"graph-pruning inputs are invalid")
    if (n == 0 | rows(firm) != n | rows(frequency) != n |
        rows(deletion_id) != n | hasmissing(worker) | hasmissing(firm) |
        hasmissing(frequency) | hasmissing(deletion_id)) return
    timer_clear(95)
    timer_on(95)

    if (deletion == "match") {
        deletion_order = order(deletion_id,1)
        deletion_sorted = deletion_id[deletion_order]
        deletion_panel = panelsetup(deletion_sorted,1)
        for (group=1; group<=rows(deletion_panel); group++) {
            begin = deletion_panel[group,1]
            finish = deletion_panel[group,2]
            index = deletion_order[|begin\finish|]
            if (min(worker[index]) != max(worker[index]) |
                min(firm[index]) != max(firm[index])) {
                st_local(status_local,"CROSS_COORDINATE_MATCH")
                st_local(message_local,
                    "each deletion ID must remain within one worker-firm coordinate")
                return
            }
        }
    }
    else if (deletion != "observation") {
        st_local(status_local,"UNSUPPORTED_DELETION")
        st_local(message_local,"deletion must be observation or match")
        return
    }

    workers = max(worker)
    active = J(n,1,1)
    component = kssbc__largest_component(worker,firm,frequency,active)
    if (component.ambiguous) {
        st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
        st_local(message_local,
            "multiple connected components tie for largest-component selection")
        return
    }
    if (sum(component.keep) == 0) {
        st_local(status_local,"NO_LEAVEOUT_COMPONENT")
        st_local(message_local,"no connected worker-firm component remains")
        return
    }
    active = active:*component.keep
    initial_components = component.components
    initial_component_rows = sum(active)

    if (deletion == "match") {
        worker_firms = kssbc__worker_firm_counts(worker,firm,active)
        active = active :* (worker_firms[worker] :> 1)
        mover_input_rows = sum(active)
        if (mover_input_rows == 0) {
            st_local(status_local,"NO_MOVER_SAMPLE")
            st_local(message_local,
                "no observations remain in the mover target population")
            return
        }
    }
    else mover_input_rows = sum(active)

    component = kssbc__largest_component(worker,firm,frequency,active)
    if (component.ambiguous) {
        st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
        st_local(message_local,
            "multiple mover components tie for largest-component selection")
        return
    }
    active = active:*component.keep
    graph_edges = component.edges
    maximum_components = max((initial_components,component.components))
    articulation_removed = 0
    insufficient_removed = 0
    iterations = 0
    while (sum(active) > 0) {
        component = kssbc__largest_component(worker,firm,frequency,active)
        if (component.ambiguous) {
            st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
            st_local(message_local,
                "graph pruning creates tied largest components")
            return
        }
        maximum_components = max((maximum_components,component.components))
        active = active:*component.keep
        if (sum(active) == 0) break

        if (deletion == "match") {
            worker_firms = kssbc__worker_firm_counts(worker,firm,active)
            removed = sum(worker_firms :== 1)
            if (removed > 0) {
                active = active :* (worker_firms[worker] :> 1)
                insufficient_removed = insufficient_removed+removed
                iterations = iterations+1
                continue
            }
        }
        else {
            worker_physical = J(workers,1,0)
            for (row=1; row<=n; row++) {
                if (active[row]) worker_physical[worker[row]] =
                    worker_physical[worker[row]]+frequency[row]
            }
            removed = sum(worker_physical :== 1)
            if (removed > 0) {
                active = active :* (worker_physical[worker] :> 1)
                insufficient_removed = insufficient_removed+removed
                iterations = iterations+1
                continue
            }
        }

        articulation = kssbc__worker_articulations(worker,firm,active)
        if (articulation.count == 0) break
        active = active :* (articulation.bad_worker[worker] :== 0)
        articulation_removed = articulation_removed+articulation.count
        iterations = iterations+1
        if (iterations > workers+1) {
            st_local(status_local,"GRAPH_ITERATION_FAILED")
            st_local(message_local,
                "articulation-worker pruning exceeded its finite iteration bound")
            return
        }
    }

    if (sum(active) == 0) {
        st_local(status_local,"NO_LEAVEOUT_COMPONENT")
        st_local(message_local,
            "no component remains after articulation-worker pruning")
        return
    }
    component = kssbc__largest_component(worker,firm,frequency,active)
    if (component.ambiguous) {
        st_local(status_local,"AMBIGUOUS_LARGEST_COMPONENT")
        st_local(message_local,
            "final graph pruning creates tied largest components")
        return
    }
    active = active:*component.keep
    maximum_components = max((maximum_components,component.components))
    st_store(sample_index,keep_name,active)
    retained_rows = sum(active)
    retained_mass = sum(frequency:*active)
    timer_off(95)
    graph_seconds = kssbc__timer_seconds(95)
    diagnostics = (n,retained_rows,sum(frequency),retained_mass,
        graph_edges,articulation_removed,maximum_components,
        mover_input_rows,initial_component_rows,insufficient_removed,
        iterations,graph_seconds)
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,"CONVERGED")
    st_local(message_local,
        "MATLAB-compatible leave-one-worker-connected component selected")
}

struct kssbc_rank_certificate
{
    string scalar status
    string scalar message
    real scalar gap
    real scalar max_loss
}

struct kssbc_rank_certificate scalar kssbc__joint_rank_certificate(
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector deletion_id,
    string scalar deletion,
    real scalar rank_tolerance)
{
    struct kssbc_rank_certificate scalar out
    struct kssbc_inverse_result scalar within_inverse, deleted_inverse
    real scalar n, control_count, row, cell, group, groups, begin, finish
    real scalar remaining_frequency, loss, threshold, whitening_error
    real scalar minimum_deleted_eigen
    real colvector cell_order, cell_sorted_code, cell_of_row
    real colvector cell_frequency, deletion_order, block_frequency, block_cell
    real colvector index, eigen
    real matrix cell_panel, deletion_panel, weighted_controls, cell_sum
    real matrix cell_mean, centered, within, whitener, transformed
    real matrix weighted_transformed
    real matrix checked_within, block_sum, mean_gap, block_cross
    real matrix deleted_scatter, scatter_loss, deleted_within

    out.status = "UNVERIFIED_DELETION_RANK"
    out.message = "joint-control deletion rank lacks a deterministic certificate"
    out.gap = .
    out.max_loss = .
    n = rows(worker)
    control_count = cols(controls)
    if (n == 0 | rows(firm) != n | rows(controls) != n |
        rows(frequency) != n | rows(deletion_id) != n |
        control_count == 0 | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(deletion_id) | min(frequency) <= 0) return(out)
    if (deletion != "observation" & deletion != "match") return(out)

    cell_order = order((worker,firm),(1,2))
    cell_sorted_code = J(n,1,1)
    for (row=2; row<=n; row++) {
        cell_sorted_code[row] = cell_sorted_code[row-1] +
            (worker[cell_order[row]] != worker[cell_order[row-1]] |
            firm[cell_order[row]] != firm[cell_order[row-1]])
    }
    cell_panel = panelsetup(cell_sorted_code,1)
    cell_of_row = J(n,1,.)
    cell_of_row[cell_order] = cell_sorted_code
    cell_frequency = panelsum(frequency[cell_order],cell_panel)
    weighted_controls = frequency :* controls
    cell_sum = panelsum(weighted_controls[cell_order,.],cell_panel)
    cell_mean = cell_sum :/
        (cell_frequency*J(1,control_count,1))
    centered = controls - cell_mean[cell_of_row,.]
    weighted_controls = frequency :* centered
    cell_sum = panelsum(weighted_controls[cell_order,.],cell_panel)
    within = centered' * weighted_controls -
        cell_sum' * (cell_sum :/
        (cell_frequency*J(1,control_count,1)))
    within = 0.5 :* (within+within')
    within_inverse = kssbc__inverse(within,rank_tolerance)
    if (within_inverse.status != "CONVERGED") {
        out.message = "within-cell control variation cannot certify every deletion; use algorithm(exact) or revise the controls"
        return(out)
    }

    whitener = cholesky(within_inverse.inverse)
    if (hasmissing(whitener)) {
        out.message = "within-cell control whitening failed"
        return(out)
    }
    transformed = centered * whitener
    weighted_transformed = frequency :* transformed
    cell_sum = panelsum(weighted_transformed[cell_order,.],cell_panel)
    checked_within = transformed' * weighted_transformed -
        cell_sum' * (cell_sum :/
        (cell_frequency*J(1,control_count,1)))
    whitening_error = kssbc__norm2(checked_within-I(control_count))
    threshold = max((1e-10,1000*rank_tolerance))
    if (hasmissing(checked_within) | hasmissing(whitening_error) |
        whitening_error > threshold) {
        out.message = "within-cell control whitening failed its residual gate"
        return(out)
    }

    out.max_loss = 0
    minimum_deleted_eigen = .
    if (deletion == "match") {
        deletion_order = order(deletion_id,1)
        deletion_panel = panelsetup(deletion_id[deletion_order],1)
        groups = rows(deletion_panel)
        block_frequency = panelsum(
            frequency[deletion_order],deletion_panel)
        block_sum = panelsum(
            weighted_transformed[deletion_order,.],deletion_panel)
        block_cell = cell_of_row[
            deletion_order[deletion_panel[.,1]]]
        for (group=1; group<=groups; group++) {
            begin = deletion_panel[group,1]
            finish = deletion_panel[group,2]
            index = deletion_order[|begin\finish|]
            cell = block_cell[group]
            remaining_frequency =
                cell_frequency[cell]-block_frequency[group]
            block_cross = transformed[index,.]' *
                (frequency[index]:*transformed[index,.])
            deleted_scatter = block_cross -
                block_sum[group,.]'*block_sum[group,.] /
                block_frequency[group]
            deleted_scatter = 0.5:*(deleted_scatter+deleted_scatter')
            eigen = Re(eigenvalues(deleted_scatter))
            if (hasmissing(eigen) | min(eigen) < -threshold) {
                out.message = "match rank-certificate deleted scatter is numerically inconsistent"
                return(out)
            }
            if (remaining_frequency > 0) {
                mean_gap = block_sum[group,.] / block_frequency[group] -
                    (cell_sum[cell,.]-block_sum[group,.]) /
                    remaining_frequency
                scatter_loss = deleted_scatter +
                    block_frequency[group]*remaining_frequency /
                    cell_frequency[cell] * mean_gap'*mean_gap
            }
            else scatter_loss = deleted_scatter
            scatter_loss = 0.5:*(scatter_loss+scatter_loss')
            loss = sum(diagonal(scatter_loss))
            if (loss < -threshold) {
                out.message = "match rank-certificate accumulation is numerically inconsistent"
                return(out)
            }
            out.max_loss = max((out.max_loss,max((0,loss))))
            deleted_within = 0.5:*(checked_within-scatter_loss +
                (checked_within-scatter_loss)')
            eigen = Re(eigenvalues(deleted_within))
            if (hasmissing(eigen)) {
                out.message = "match deleted control scatter eigenvalue calculation failed"
                return(out)
            }
            if (missing(minimum_deleted_eigen)) {
                minimum_deleted_eigen = min(eigen)
            }
            else minimum_deleted_eigen = min((minimum_deleted_eigen,min(eigen)))
            deleted_inverse = kssbc__inverse(deleted_within,rank_tolerance)
            if (deleted_inverse.status != "CONVERGED") {
                out.message = "match deletion does not retain a directly factorable control scatter"
                return(out)
            }
        }
    }
    else {
        for (row=1; row<=n; row++) {
            cell = cell_of_row[row]
            remaining_frequency = cell_frequency[cell]-1
            if (remaining_frequency > 0) {
                mean_gap = transformed[row,.] -
                    (cell_sum[cell,.]-transformed[row,.]) /
                    remaining_frequency
                scatter_loss = remaining_frequency / cell_frequency[cell] *
                    mean_gap'*mean_gap
            }
            else scatter_loss = J(control_count,control_count,0)
            scatter_loss = 0.5:*(scatter_loss+scatter_loss')
            loss = sum(diagonal(scatter_loss))
            if (loss < -threshold) {
                out.message = "observation rank-certificate accumulation is numerically inconsistent"
                return(out)
            }
            out.max_loss = max((out.max_loss,max((0,loss))))
            deleted_within = 0.5:*(checked_within-scatter_loss +
                (checked_within-scatter_loss)')
            eigen = Re(eigenvalues(deleted_within))
            if (hasmissing(eigen)) {
                out.message = "observation deleted control scatter eigenvalue calculation failed"
                return(out)
            }
            if (missing(minimum_deleted_eigen)) {
                minimum_deleted_eigen = min(eigen)
            }
            else minimum_deleted_eigen = min((minimum_deleted_eigen,min(eigen)))
            deleted_inverse = kssbc__inverse(deleted_within,rank_tolerance)
            if (deleted_inverse.status != "CONVERGED") {
                out.message = "observation deletion does not retain a directly factorable control scatter"
                return(out)
            }
        }
    }

    out.gap = min((1-out.max_loss,minimum_deleted_eigen)) -
        whitening_error-threshold
    if (hasmissing(out.gap) | out.gap <= 0) {
        out.message = "within-cell variation does not certify joint-control rank after every deletion; use algorithm(exact) or revise the controls"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "joint-control deletion rank certified by a probe-independent within-cell scatter bound"
    return(out)
}

struct kssbc_result scalar kssbc__jla(
    real colvector y,
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector target_weight,
    real colvector deletion_id,
    string scalar deletion,
    string scalar nuisance,
    real scalar probes,
    real scalar batch,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit)
{
    struct kssbc_result scalar out
    struct kssbc_fe_design scalar base
    struct kssbc_joint_design scalar full_joint, working_joint
    struct kssbc_solve_result scalar solved, projection_solved, target_solved
    struct kssbc_inverse_result scalar maker_inverse
    struct kssbc_rank_certificate scalar rank_certificate
    real scalar n, controls_count, base_parameters, full_parameters
    real scalar probe, group, groups, begin, finish, eigmax, maximum_leverage
    real scalar batch_start, batch_finish, batch_columns, batch_column
    real scalar solver_iterations, solver_residual, target_mass
    real scalar deletion_rank_gap, physical_count, row
    real matrix deletion_panel, physical_panel, sorted_delete, rhs
    real matrix target_rhs, target_draws, physical_random_batch
    real matrix rademacher_batch, projected_batch, target_direction_batch
    real matrix block_control, projection_block, maker, inverse_maker
    real colvector row_order, index, working_y, coefficient, fitted, residual
    real colvector rademacher_sum, projected, physical_row, physical_random
    real colvector physical_projected, projection_square_sum
    real colvector projection_fourth_sum, copy_first_correlation
    real colvector copy_third_correlation, copy_control_leverage
    real colvector copy_inverse_weight
    real colvector p_first, m_first, p_second
    real colvector m_second, mixed_second, p_mean, m_mean, denominator
    real colvector p_constrained, m_constrained, finite_variance, finite_bias
    real colvector control_leverage, total_residual_leverage, inverse_weight
    real colvector deletion_frequency, deletion_projected, deletion_random
    real colvector transformed_residual, deleted_adjusted, block_frequency
    real colvector common_direction, inverse_common
    real matrix worker_target, firm_target
    real colvector worker_projection, firm_projection
    real colvector total_projection, correction_weight, group_first, group_second
    real colvector eigen, gamma
    real rowvector plugin, correction, numerical_mcse

    out = kssbc__empty_result()
    deletion_rank_gap = .
    n = rows(y)
    if (n == 0 | cols(y) != 1 | rows(worker) != n | rows(firm) != n |
        rows(controls) != n | rows(frequency) != n |
        rows(target_weight) != n | rows(deletion_id) != n |
        hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(target_weight) | hasmissing(deletion_id)) {
        return(kssbc__failure("INVALID_INPUT", "matrix-free KSS inputs are invalid"))
    }
    if (min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0) {
        return(kssbc__failure("INVALID_FREQUENCY", "frequency weights must be positive integers"))
    }
    if (min(target_weight) < 0 | sum(target_weight) <= 0) {
        return(kssbc__failure("INVALID_TARGET_WEIGHT", "target weights must have nonnegative positive mass"))
    }
    if (deletion != "observation" & deletion != "match") {
        return(kssbc__failure("UNSUPPORTED_DELETION", "deletion must be observation or match"))
    }
    if (nuisance != "joint" & nuisance != "fixedoffset") {
        return(kssbc__failure("INVALID_NUISANCE", "nuisance must be joint or fixedoffset"))
    }
    if (probes < 2 | probes != floor(probes) | batch < 1 |
        maxiter < 1 | tolerance <= 0 | tolerance >= 1) {
        return(kssbc__failure("INVALID_TUNING", "invalid JLA or solver tuning parameter"))
    }

    timer_clear(91)
    timer_clear(92)
    timer_clear(93)
    timer_clear(94)
    timer_on(91)

    timer_on(94)
    base = kssbc__fe_prepare(worker,firm,frequency,rank_tolerance)
    timer_off(94)
    if (base.status != "CONVERGED") {
        return(kssbc__failure(base.status,base.message))
    }
    base_parameters = base.worker_levels + base.firm_levels - 1
    controls_count = cols(controls)
    full_parameters = base_parameters + controls_count
    if (deletion == "match") {
        row_order = order(deletion_id,1)
        sorted_delete = deletion_id[row_order]
        deletion_panel = panelsetup(sorted_delete,1)
        groups = rows(deletion_panel)
        for (group=1; group<=groups; group++) {
            begin = deletion_panel[group,1]
            finish = deletion_panel[group,2]
            index = row_order[|begin \ finish|]
            if (rows(index) > blocksize_limit) {
                return(kssbc__failure("BLOCK_SIZE_LIMIT", "a match block exceeds blocksize_limit()"))
            }
            if (min(worker[index]) != max(worker[index]) |
                min(firm[index]) != max(firm[index])) {
                return(kssbc__failure("CROSS_COORDINATE_MATCH", "each deletion ID must remain within one worker-firm coordinate"))
            }
        }
    }
    else {
        groups = sum(frequency)
        row_order = J(0,1,.)
        deletion_panel = J(0,2,.)
    }

    full_joint = kssbc__joint_prepare(
        base,controls,tolerance,maxiter,rank_tolerance)
    if (full_joint.status != "CONVERGED") {
        return(kssbc__failure(full_joint.status,full_joint.message))
    }
    if (nuisance == "joint" & controls_count > 0) {
        rank_certificate = kssbc__joint_rank_certificate(
            worker,firm,controls,frequency,deletion_id,deletion,
            rank_tolerance)
        if (rank_certificate.status != "CONVERGED") {
            return(kssbc__failure(
                rank_certificate.status,rank_certificate.message))
        }
        deletion_rank_gap = rank_certificate.gap
    }
    rhs = kssbc__joint_transpose(full_joint,frequency:*y)
    solved = kssbc__joint_solve(full_joint,rhs,tolerance,maxiter)
    if (solved.status != "CONVERGED") {
        return(kssbc__failure(solved.status,solved.message))
    }
    solver_iterations = max((full_joint.preparation_iterations,solved.iterations))
    solver_residual = max((full_joint.preparation_relres,solved.relres))

    if (nuisance == "fixedoffset" & controls_count > 0) {
        gamma = solved.coefficient[(base_parameters+1)..full_parameters]
        working_y = y - controls*gamma
        working_joint = kssbc__joint_prepare(
            base,J(n,0,.),tolerance,maxiter,rank_tolerance)
        rhs = kssbc__joint_transpose(working_joint,frequency:*working_y)
        solved = kssbc__joint_solve(working_joint,rhs,tolerance,maxiter)
        if (solved.status != "CONVERGED") {
            return(kssbc__failure(solved.status,solved.message))
        }
        solver_iterations = max((solver_iterations,solved.iterations))
        solver_residual = max((solver_residual,solved.relres))
    }
    else {
        working_y = y
        working_joint = full_joint
    }
    coefficient = solved.coefficient
    fitted = kssbc__joint_predict(working_joint,coefficient)
    residual = working_y - fitted
    plugin = kssbc__effect_plugin(
        coefficient[1..base_parameters],base,target_weight)
    timer_off(91)
    timer_on(92)

    rseed(seed)
    physical_count = sum(frequency)
    physical_panel = kssbc__physical_panels(frequency)
    if (deletion == "observation") {
        physical_row = J(physical_count,1,.)
        for (row=1; row<=n; row++) {
            physical_row[|physical_panel[row,1]\physical_panel[row,2]|] =
                J(frequency[row],1,row)
        }
        projection_square_sum = J(n,1,0)
        projection_fourth_sum = J(n,1,0)
        copy_first_correlation = J(physical_count,1,0)
        copy_third_correlation = J(physical_count,1,0)
    }
    else {
        deletion_frequency = panelsum(frequency[row_order],deletion_panel)
        p_first = J(groups,1,0)
        m_first = J(groups,1,0)
        p_second = J(groups,1,0)
        m_second = J(groups,1,0)
        mixed_second = J(groups,1,0)
    }

    for (batch_start=1; batch_start<=probes; batch_start=batch_start+batch) {
        batch_finish = min((probes,batch_start+batch-1))
        batch_columns = batch_finish-batch_start+1
        rademacher_batch = J(n,batch_columns,.)
        if (deletion == "observation") {
            physical_random_batch = J(physical_count,batch_columns,.)
        }
        for (batch_column=1; batch_column<=batch_columns; batch_column++) {
            if (deletion == "observation") {
                physical_random_batch[.,batch_column] =
                    2:*rbinomial(physical_count,1,1,0.5):-1
                rademacher_batch[.,batch_column] = panelsum(
                    physical_random_batch[.,batch_column],physical_panel)
            }
            else rademacher_batch[.,batch_column] =
                    kssbc__physical_rademacher_sum(frequency)
        }
        rhs = kssbc__fe_transpose(base,rademacher_batch)
        projection_solved = kssbc__fe_solve_matrix(
            base,rhs,tolerance,maxiter)
        if (projection_solved.status != "CONVERGED") {
            return(kssbc__failure(
                projection_solved.status,projection_solved.message))
        }
        solver_iterations = max((solver_iterations,projection_solved.iterations))
        solver_residual = max((solver_residual,projection_solved.relres))
        projected_batch = kssbc__fe_predict(
            base,projection_solved.coefficient)
        for (batch_column=1; batch_column<=batch_columns; batch_column++) {
            projected = projected_batch[.,batch_column]
            rademacher_sum = rademacher_batch[.,batch_column]
            if (deletion == "observation") {
                physical_random = physical_random_batch[.,batch_column]
                physical_projected = projected[physical_row]
                projection_square_sum = projection_square_sum + projected:^2
                projection_fourth_sum = projection_fourth_sum + projected:^4
                copy_first_correlation = copy_first_correlation +
                    physical_random:*physical_projected
                copy_third_correlation = copy_third_correlation +
                    physical_random:*physical_projected:^3
            }
            else {
                deletion_projected = panelsum(
                    (frequency:*projected)[row_order],deletion_panel) :/
                    deletion_frequency
                deletion_projected = sqrt(deletion_frequency) :*
                    deletion_projected
                deletion_random = panelsum(
                    rademacher_sum[row_order],deletion_panel) :/
                    sqrt(deletion_frequency)
                deletion_random = deletion_random - deletion_projected
                p_first = p_first + deletion_projected:^2
                m_first = m_first + deletion_random:^2
                p_second = p_second + deletion_projected:^4
                m_second = m_second + deletion_random:^4
                mixed_second = mixed_second +
                    deletion_projected:^2 :* deletion_random:^2
            }
        }
    }

    if (deletion == "observation") {
        p_first = projection_square_sum[physical_row]
        p_second = projection_fourth_sum[physical_row]
        m_first = probes:+p_first:-2:*copy_first_correlation
        m_second = probes:+6:*p_first:+p_second:-
            4:*copy_first_correlation:-4:*copy_third_correlation
        mixed_second = p_first:+p_second:-2:*copy_third_correlation
    }
    p_mean = p_first :/ probes
    m_mean = m_first :/ probes
    denominator = p_mean + m_mean
    if (hasmissing(denominator) | min(denominator) <= block_tolerance) {
        return(kssbc__failure("JLA_CONSTRAINT_FAILED", "JLA projection and residual masses do not have positive sum"))
    }
    p_constrained = p_mean :/ denominator
    m_constrained = m_mean :/ denominator
    p_second = p_second :/ probes
    m_second = m_second :/ probes
    mixed_second = mixed_second :/ probes
    finite_variance = (m_constrained:^2:*p_second +
        p_constrained:^2:*m_second -
        2:*p_constrained:*m_constrained:*mixed_second) :/ probes
    finite_bias = (m_constrained:*p_second -
        p_constrained:*m_second +
        (m_constrained-p_constrained):*mixed_second) :/ probes
    if (min(finite_variance) < -100*rank_tolerance) {
        return(kssbc__failure("JLA_MOMENT_FAILED", "finite-projection variance estimate is negative"))
    }
    finite_variance = finite_variance :* (finite_variance :> 0)

    if (cols(working_joint.controls) > 0) {
        control_leverage = rowsum(
            (working_joint.residualized_controls*working_joint.schur_inverse) :*
            working_joint.residualized_controls)
    }
    else control_leverage = J(n,1,0)

    maximum_leverage = 0
    if (deletion == "observation") {
        copy_control_leverage = control_leverage[physical_row]
        total_residual_leverage = m_constrained - copy_control_leverage
        if (min(total_residual_leverage) <= block_tolerance) {
            return(kssbc__failure("NONESTIMABLE_DELETION", "estimated full observation residual leverage is nonpositive"))
        }
        copy_inverse_weight = 1:/total_residual_leverage +
            finite_bias:/total_residual_leverage:^2 -
            finite_variance:/total_residual_leverage:^3
        if (hasmissing(copy_inverse_weight) | min(copy_inverse_weight) <= 0) {
            return(kssbc__failure("JLA_INVERSE_FAILED", "finite-projection observation inverse is nonpositive"))
        }
        inverse_weight = panelsum(copy_inverse_weight,physical_panel) :/
            frequency
        deleted_adjusted = residual :* inverse_weight
        maximum_leverage = max(p_constrained+copy_control_leverage)
    }
    else {
        deleted_adjusted = J(n,1,.)
        for (group=1; group<=groups; group++) {
            begin = deletion_panel[group,1]
            finish = deletion_panel[group,2]
            index = row_order[|begin \ finish|]
            block_frequency = sqrt(frequency[index])
            common_direction = block_frequency :/ sqrt(deletion_frequency[group])
            projection_block = p_constrained[group] :*
                common_direction*common_direction'
            if (cols(working_joint.controls) > 0) {
                block_control = block_frequency :*
                    working_joint.residualized_controls[index,.]
                projection_block = projection_block +
                    block_control*working_joint.schur_inverse*block_control'
            }
            projection_block = 0.5:*(projection_block+projection_block')
            eigen = Re(eigenvalues(projection_block))
            eigmax = max(eigen)
            if (hasmissing(eigen) | 1-eigmax <= block_tolerance) {
                return(kssbc__failure("NONESTIMABLE_DELETION", "estimated full match residual block is singular"))
            }
            maximum_leverage = max((maximum_leverage,eigmax))
            maker = I(rows(index)) - projection_block
            maker_inverse = kssbc__inverse(maker,rank_tolerance)
            if (maker_inverse.status != "CONVERGED") {
                return(kssbc__failure("BLOCK_INVERSE_FAILED", "estimated match residual block inverse failed"))
            }
            solver_residual = max((solver_residual,maker_inverse.relres))
            inverse_maker = maker_inverse.inverse
            transformed_residual = block_frequency :* residual[index]
            inverse_common = inverse_maker*common_direction
            deleted_adjusted[index] = inverse_maker*transformed_residual +
                finite_bias[group] :* inverse_common :*
                (common_direction'*inverse_maker*transformed_residual)[1,1] -
                finite_variance[group] :* inverse_common :*
                (common_direction'*inverse_maker*common_direction)[1,1] :*
                (common_direction'*inverse_maker*transformed_residual)[1,1]
        }
    }

    timer_off(92)
    timer_on(93)

    target_draws = J(probes,4,.)
    target_mass = sum(target_weight)
    for (batch_start=1; batch_start<=probes; batch_start=batch_start+batch) {
        batch_finish = min((probes,batch_start+batch-1))
        batch_columns = batch_finish-batch_start+1
        target_direction_batch = J(n,batch_columns,.)
        for (batch_column=1; batch_column<=batch_columns; batch_column++) {
            rademacher_sum = kssbc__physical_rademacher_sum(frequency)
            target_direction_batch[.,batch_column] =
                kssbc__target_direction(
                    frequency,target_weight,rademacher_sum)
        }
        worker_target = kssbc__group_sum(
            target_direction_batch,base.worker_order,base.worker_panel)
        firm_target = kssbc__group_sum(
            target_direction_batch,base.firm_order,base.firm_panel)
        target_rhs = J(base_parameters+cols(working_joint.controls),
            2*batch_columns,0)
        for (batch_column=1; batch_column<=batch_columns; batch_column++) {
            target_rhs[1..base.worker_levels,2*batch_column-1] =
                worker_target[.,batch_column]
            target_rhs[(base.worker_levels+1)..base_parameters,
                2*batch_column] =
                firm_target[1..(base.firm_levels-1),batch_column]
        }
        target_solved = kssbc__joint_solve(
            working_joint,target_rhs,tolerance,maxiter)
        if (target_solved.status != "CONVERGED") {
            return(kssbc__failure(target_solved.status,target_solved.message))
        }
        solver_iterations = max((solver_iterations,target_solved.iterations))
        solver_residual = max((solver_residual,target_solved.relres))
        for (batch_column=1; batch_column<=batch_columns; batch_column++) {
            probe = batch_start+batch_column-1
            worker_projection = kssbc__joint_predict(
                working_joint,
                target_solved.coefficient[.,2*batch_column-1])
            firm_projection = kssbc__joint_predict(
                working_joint,target_solved.coefficient[.,2*batch_column])
            total_projection = worker_projection + firm_projection
            if (deletion == "observation") {
                correction_weight = frequency:*working_y:*deleted_adjusted
                target_draws[probe,1] = sum(
                    correction_weight:*worker_projection:^2)
                target_draws[probe,2] = sum(
                    correction_weight:*firm_projection:^2)
                target_draws[probe,4] = sum(
                    correction_weight:*total_projection:^2)
            }
            else {
                group_first = panelsum(
                    (frequency:*working_y:*worker_projection)[row_order],
                    deletion_panel)
                group_second = panelsum(
                    (sqrt(frequency):*worker_projection:*
                    deleted_adjusted)[row_order],deletion_panel)
                target_draws[probe,1] = sum(group_first:*group_second)
                group_first = panelsum(
                    (frequency:*working_y:*firm_projection)[row_order],
                    deletion_panel)
                group_second = panelsum(
                    (sqrt(frequency):*firm_projection:*
                    deleted_adjusted)[row_order],deletion_panel)
                target_draws[probe,2] = sum(group_first:*group_second)
                group_first = panelsum(
                    (frequency:*working_y:*total_projection)[row_order],
                    deletion_panel)
                group_second = panelsum(
                    (sqrt(frequency):*total_projection:*
                    deleted_adjusted)[row_order],deletion_panel)
                target_draws[probe,4] = sum(group_first:*group_second)
            }
            target_draws[probe,3] = 0.5 :*
                (target_draws[probe,4]-target_draws[probe,1]-
                target_draws[probe,2])
        }
    }
    correction = colsum(target_draws) :/ probes
    numerical_mcse = kssbc__mcse(target_draws)
    timer_off(93)
    if (hasmissing(plugin) | hasmissing(correction) |
        hasmissing(numerical_mcse)) {
        return(kssbc__failure("NONFINITE_CORRECTION", "JLA target correction is nonfinite"))
    }

    out.status = "CONVERGED"
    out.message = "matrix-free improved-JLA KSS calculation converged"
    out.plugin = plugin
    out.correction = correction
    out.corrected = plugin-correction
    out.numerical_mcse = numerical_mcse
    out.n_stored = n
    out.n_physical = sum(frequency)
    out.worker_levels = base.worker_levels
    out.firm_levels = base.firm_levels
    out.parameters = base_parameters + cols(working_joint.controls)
    out.deletion_units = groups
    out.target_weight_sum = target_mass
    out.max_leverage = maximum_leverage
    out.information_rcond = .
    out.preconditioner_ratio = base.preconditioner_ratio
    if (cols(full_joint.controls) > 0) {
        out.control_schur_rcond = full_joint.schur_rcond
    }
    else out.control_schur_rcond = .
    out.deletion_rank_gap = deletion_rank_gap
    out.inverse_relres = solver_residual
    out.weighted_rss = sum(frequency:*residual:^2)
    out.fit_seconds = kssbc__timer_seconds(91)
    out.leverage_seconds = kssbc__timer_seconds(92)
    out.target_seconds = kssbc__timer_seconds(93)
    out.correction_seconds = out.leverage_seconds+out.target_seconds
    out.preconditioner_seconds = kssbc__timer_seconds(94)
    out.solver_iterations = solver_iterations
    out.solver_max_residual = solver_residual
    out.probes = probes
    return(out)
}

void kssbc__stata_jla(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar sample_name,
    string scalar deletion,
    string scalar nuisance,
    real scalar probes,
    real scalar batch,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit,
    string scalar results_name,
    string scalar status_local,
    string scalar message_local,
    string scalar diagnostics_name)
{
    struct kssbc_result scalar out
    real colvector y, worker, firm, frequency, target, deletion_id
    real matrix controls, results, diagnostics

    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    out = kssbc__jla(
        y,worker,firm,controls,frequency,target,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit)
    results = out.plugin \ out.correction \ out.corrected \
        out.numerical_mcse
    diagnostics = (out.n_stored,out.n_physical,out.worker_levels,
        out.firm_levels,out.parameters,out.deletion_units,
        out.target_weight_sum,out.max_leverage,out.information_rcond,
        out.inverse_relres,out.solver_iterations,
        out.solver_max_residual,out.probes,out.weighted_rss,
        out.fit_seconds,out.leverage_seconds,out.target_seconds,
        out.correction_seconds,out.preconditioner_seconds,
        out.preconditioner_ratio,out.control_schur_rcond,
        out.deletion_rank_gap)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

void kssbc__stata_exact(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar sample_name,
    string scalar deletion,
    string scalar nuisance,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar exact_limit,
    real scalar blocksize_limit,
    string scalar results_name,
    string scalar status_local,
    string scalar message_local,
    string scalar diagnostics_name)
{
    struct kssbc_result scalar out
    real colvector y, worker, firm, frequency, target, deletion_id
    real matrix controls, results, diagnostics

    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    out = kssbc__exact(
        y, worker, firm, controls, frequency, target, deletion_id,
        deletion, nuisance, rank_tolerance, block_tolerance,
        exact_limit, blocksize_limit)

    results = out.plugin \ out.correction \ out.corrected \
        out.numerical_mcse
    diagnostics = (out.n_stored, out.n_physical, out.worker_levels,
        out.firm_levels, out.parameters, out.deletion_units,
        out.target_weight_sum, out.max_leverage,
        out.information_rcond, out.inverse_relres,
        out.solver_iterations,out.solver_max_residual,out.probes,
        out.weighted_rss,out.fit_seconds,out.leverage_seconds,
        out.target_seconds,out.correction_seconds,
        out.preconditioner_seconds,out.preconditioner_ratio,
        out.control_schur_rcond,out.deletion_rank_gap)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

end
