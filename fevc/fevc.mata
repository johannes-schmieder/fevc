*! fevc Mata runtime 0.5.0-rc.1 05sep2026

version 18.0

mata:
mata set matastrict on
mata set matalnum off

string scalar vckss__version()
{
    return("0.5.0-rc.1")
}

real scalar vckss__api_level()
{
    return(21)
}

string scalar vckss__build_id()
{
    return("vckss-api21-stayer-hybrid")
}

real scalar vckss__norm2(real matrix value)
{
    if (rows(value) == 0 | cols(value) == 0) return(0)
    return(sqrt(quadcross(vec(value),vec(value))))
}

real scalar vckss__max_column_relres(
    real matrix residual,
    real matrix right_hand_side)
{
    real scalar column, maximum, one, scale

    if (rows(residual) != rows(right_hand_side) |
        cols(residual) != cols(right_hand_side) |
        rows(residual) == 0 | cols(residual) == 0 |
        hasmissing(residual) | hasmissing(right_hand_side)) return(.)
    maximum = 0
    for (column=1; column<=cols(residual); column++) {
        scale = vckss__norm2(right_hand_side[.,column])
        if (scale == 0) one = vckss__norm2(residual[.,column])
        else one = vckss__norm2(residual[.,column]) / scale
        maximum = max((maximum,one))
    }
    return(maximum)
}

real rowvector vckss__column_relres(
    real matrix residual,
    real matrix right_hand_side)
{
    real scalar column, scale
    real rowvector out

    if (rows(residual) != rows(right_hand_side) |
        cols(residual) != cols(right_hand_side) |
        rows(residual) == 0 | cols(residual) == 0 |
        hasmissing(residual) | hasmissing(right_hand_side)) return(J(1,0,.))
    out = J(1,cols(residual),.)
    for (column=1; column<=cols(residual); column++) {
        scale = vckss__norm2(right_hand_side[.,column])
        if (scale == 0) out[column] = vckss__norm2(residual[.,column])
        else out[column] = vckss__norm2(residual[.,column]) / scale
    }
    return(out)
}

/* Stable column totals used when an algebraic zero-sum restriction must be
   represented exactly in binary64.  The input order is already the public
   canonical semantic order established by the ado layer. */
real rowvector vckss__compensated_column_sum(real matrix values)
{
    if (rows(values) == 0 | cols(values) == 0 |
        hasmissing(values)) return(J(1,0,.))
    return(quadcolsum(values))
}

/* The separately solved worker and firm target-score blocks are zero sum in
   exact arithmetic.  Form the final canonical coordinate from all preceding
   coordinates so the supplied full RHS is compatible by construction.  A
   repair larger than registered regrouping roundoff is a typed failure, not
   a projection onto a nearby estimand. */
real matrix vckss__balance_zero_sum_score(
    real matrix score,
    real rowvector reference_scale)
{
    real scalar column, original_last, roundoff_gate
    real matrix out
    real rowvector preceding

    if (rows(score) == 0 | cols(score) == 0 |
        cols(reference_scale) != cols(score) |
        hasmissing(score) | hasmissing(reference_scale) |
        min(reference_scale) < 0) return(J(0,0,.))
    roundoff_gate = 4096*2.2204460492503131e-16
    out = score
    if (rows(out) > 1) preceding =
        vckss__compensated_column_sum(out[1..(rows(out)-1),.])
    for (column=1; column<=cols(score); column++) {
        original_last = out[rows(out),column]
        if (rows(out) == 1) out[1,column] = 0
        else out[rows(out),column] = -preceding[column]
        if ((reference_scale[column] == 0 &
             out[rows(out),column] != original_last) |
            (reference_scale[column] > 0 &
             abs(out[rows(out),column]-original_last) >
                roundoff_gate*reference_scale[column])) return(J(0,0,.))
    }
    return(out)
}

real scalar vckss__rounding_gamma(real scalar operations)
{
    real scalar product

    if (missing(operations) | operations < 0) return(.)
    // IEEE binary64 machine epsilon.  Using epsilon rather than unit
    // roundoff makes the standard gamma bound conservative by a factor two.
    product = operations * 2.2204460492503131e-16
    if (missing(product) | product >= 0.5) return(.)
    return(product/(1-product))
}

real scalar vckss__exact_physical_total(real colvector frequency)
{
    real scalar row, total, exact_limit

    if (cols(frequency) != 1 | rows(frequency) == 0 |
        hasmissing(frequency)) return(.)
    // Every integer through 2^53 has an exact binary64 representation.  By
    // testing the remaining capacity before each nonnegative addition, every
    // accepted partial sum is also exact; an over-limit total can therefore
    // never round back down to the boundary.
    exact_limit = 9007199254740992
    total = 0
    for (row=1; row<=rows(frequency); row++) {
        if (frequency[row] <= 0 |
            frequency[row] != floor(frequency[row]) |
            frequency[row] > exact_limit-total) return(.)
        total = total+frequency[row]
    }
    return(total)
}

real scalar vckss__inverse_forward_error(
    real scalar max_column_relres,
    real scalar reciprocal_condition,
    real scalar dimension)
{
    real scalar operator_residual, denominator

    if (missing(max_column_relres) | missing(reciprocal_condition) |
        missing(dimension) | dimension < 1 |
        max_column_relres < 0 | reciprocal_condition <= 0) return(.)
    // ||R||_2 <= ||R||_F <= sqrt(k) max_j ||R_j||_2.  The denominator is
    // the fail-closed inverse-perturbation margin, rather than an unscaled
    // residual/condition-number heuristic.
    operator_residual = sqrt(dimension)*max_column_relres
    denominator = reciprocal_condition-operator_residual
    if (missing(operator_residual) | denominator <= 0) return(.)
    return(operator_residual/denominator)
}

real scalar vckss__control_forward_limit()
{
    // Registered relative propagation ceiling for the canonical-control
    // representation.  This is separate from the PCG stopping tolerance.
    return(1e-8)
}

real scalar vckss__propagate_error(
    real scalar forward_error,
    real scalar reciprocal_margin)
{
    if (missing(forward_error) | missing(reciprocal_margin) |
        forward_error < 0 | reciprocal_margin <= forward_error) return(.)
    return(forward_error/(reciprocal_margin-forward_error))
}

real scalar vckss__timer_seconds(real scalar identifier)
{
    real matrix value

    value = timer_value(identifier)
    return(value[1,1])
}

struct vckss_inverse_result
{
    string scalar status
    real matrix inverse
    real scalar rcond
    real scalar relres
}

struct vckss_maker_result
{
    string scalar status
    string scalar message
    real matrix actions
    real scalar eigmax
    real scalar relres
}

struct vckss_target_matrices
{
    real matrix worker
    real matrix firm
    real matrix covariance
}

struct vckss_result
{
    string scalar status
    string scalar message
    real rowvector plugin
    real rowvector correction
    real rowvector corrected
    real rowvector numerical_mcse
    real matrix correction_by_source
    real scalar n_stored
    real scalar n_physical
    real scalar worker_levels
    real scalar firm_levels
    real scalar parameters
    real scalar full_parameters
    real scalar correction_parameters
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
    real scalar schur_seconds
    real scalar preconditioner_apply_seconds
    real scalar pcg_seconds
    real scalar solver_backend_seconds
    real scalar solver_iterations
    real scalar solver_max_residual
    real scalar solver_schur_actions
    real scalar solver_schur_batches
    real scalar solver_precond_applications
    real scalar solver_precond_batches
    real matrix solver_rhs_diagnostics
    real scalar fe_workspace_applicable
    real scalar fe_workspace_builds
    real scalar fe_buffered_schur_batches
    real scalar fe_legacy_schur_batches
    real scalar fe_buffered_schur_columns
    real scalar fe_legacy_schur_columns
    real scalar fe_packed_fallback_batches
    real scalar fe_max_buffer_width
    real scalar fe_workspace_peak_bytes
    real scalar fe_cell_bytes_avoided
    real scalar probes
}

struct vckss_result scalar vckss__empty_result()
{
    struct vckss_result scalar out

    out.status = "INVALID_INPUT"
    out.message = "invalid exact KSS input"
    out.plugin = J(1,4,.)
    out.correction = J(1,4,.)
    out.corrected = J(1,4,.)
    out.numerical_mcse = J(1,4,0)
    out.correction_by_source = J(2,4,.)
    out.n_stored = .
    out.n_physical = .
    out.worker_levels = .
    out.firm_levels = .
    out.parameters = .
    out.full_parameters = .
    out.correction_parameters = .
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
    out.schur_seconds = .
    out.preconditioner_apply_seconds = .
    out.pcg_seconds = .
    out.solver_backend_seconds = .
    out.solver_iterations = .
    out.solver_max_residual = .
    out.solver_schur_actions = .
    out.solver_schur_batches = .
    out.solver_precond_applications = .
    out.solver_precond_batches = .
    out.solver_rhs_diagnostics = J(0,6,.)
    out.fe_workspace_applicable = 0
    out.fe_workspace_builds = 0
    out.fe_buffered_schur_batches = 0
    out.fe_legacy_schur_batches = 0
    out.fe_buffered_schur_columns = 0
    out.fe_legacy_schur_columns = 0
    out.fe_packed_fallback_batches = 0
    out.fe_max_buffer_width = 0
    out.fe_workspace_peak_bytes = 0
    out.fe_cell_bytes_avoided = 0
    out.probes = .
    return(out)
}

struct vckss_result scalar vckss__failure(
    string scalar status,
    string scalar message)
{
    struct vckss_result scalar out

    out = vckss__empty_result()
    out.status = status
    out.message = message
    return(out)
}

struct vckss_inverse_result scalar vckss__inverse(
    real matrix information,
    real scalar rank_tolerance)
{
    struct vckss_inverse_result scalar out
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
    out.relres = vckss__max_column_relres(residual,I(dimension))
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

struct vckss_maker_result scalar vckss__low_rank_maker(
    real matrix factor,
    real matrix right_hand_side,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_maker_result scalar out
    struct vckss_inverse_result scalar reduced_inverse
    real matrix gram, reduced_maker, complete_maker, residual
    real colvector eigen
    real scalar minimum_maker

    out.status = "INVALID_INPUT"
    out.message = "low-rank match-block inputs are invalid"
    out.actions = J(0,0,.)
    out.eigmax = .
    out.relres = .
    if (rows(factor) == 0 | cols(factor) == 0 |
        rows(right_hand_side) != rows(factor) |
        cols(right_hand_side) == 0 | hasmissing(factor) |
        hasmissing(right_hand_side) | rank_tolerance <= 0 |
        block_tolerance <= 0) return(out)

    if (rows(factor) <= cols(factor)) {
        complete_maker = I(rows(factor))-factor*factor'
        complete_maker = 0.5:*(complete_maker+complete_maker')
        eigen = Re(eigenvalues(complete_maker))
        if (hasmissing(eigen)) {
            out.status = "NONFINITE_LEVERAGE"
            out.message = "match residual-maker eigenvalue calculation failed"
            return(out)
        }
        minimum_maker = min(eigen)
        out.eigmax = 1-minimum_maker
        if (minimum_maker <= block_tolerance) {
            out.status = "NONESTIMABLE_DELETION"
            out.message = "a match residual block is singular"
            return(out)
        }
        reduced_inverse = vckss__inverse(
            complete_maker,rank_tolerance)
        if (reduced_inverse.status == "CONVERGED") {
            out.actions = reduced_inverse.inverse*right_hand_side
        }
    }
    else {
        gram = factor'*factor
        gram = 0.5:*(gram+gram')
        reduced_maker = I(cols(factor))-gram
        eigen = Re(eigenvalues(reduced_maker))
        if (hasmissing(eigen)) {
            out.status = "NONFINITE_LEVERAGE"
            out.message = "reduced match residual-maker eigenvalue calculation failed"
            return(out)
        }
        minimum_maker = min(eigen)
        out.eigmax = 1-minimum_maker
        if (minimum_maker <= block_tolerance) {
            out.status = "NONESTIMABLE_DELETION"
            out.message = "a reduced match residual block is singular"
            return(out)
        }
        reduced_inverse = vckss__inverse(
            reduced_maker,rank_tolerance)
        if (reduced_inverse.status == "CONVERGED") {
            out.actions = right_hand_side +
                factor*reduced_inverse.inverse *
                (factor'*right_hand_side)
        }
    }
    if (reduced_inverse.status != "CONVERGED") {
        out.status = "BLOCK_INVERSE_FAILED"
        out.message = "a match residual solve failed"
        return(out)
    }
    residual = out.actions-factor*(factor'*out.actions)-right_hand_side
    out.relres = max((reduced_inverse.relres,
        vckss__max_column_relres(residual,right_hand_side)))
    if (hasmissing(out.actions) | hasmissing(out.relres) |
        out.relres > max((1e-10,100*rank_tolerance))) {
        out.status = "BLOCK_INVERSE_FAILED"
        out.message = "a match residual solve failed its complete residual gate"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "dimension-reduced match residual solve converged"
    return(out)
}

struct vckss_control_basis_result
{
    string scalar status
    string scalar message
    real matrix controls
    real scalar relres
    real scalar forward_error
}

struct vckss_control_basis_result scalar vckss__canonical_controls(
    real matrix controls,
    real colvector frequency,
    real scalar rank_tolerance)
{
    struct vckss_control_basis_result scalar out
    struct vckss_inverse_result scalar gram_inverse, anchor_inverse
    real scalar n, control_count, pivot, chosen, maximum, margin
    real scalar whitening_error, anchor_error, cutoff, uncertainty
    real scalar numerical_error, summation_error, inverse_forward_error
    real scalar cholesky_error, basis_product_error, score_error
    real scalar anchor_forward_error, product_error, canonical_error
    real scalar projection_error, selected_zero_error, span_error
    real colvector selected, score, eligible, boundary
    real matrix gram, whitener, orthonormal, checked
    real matrix anchor, anchor_gram, residualized, projector

    out.status = "SINGULAR_NUISANCE_BLOCK"
    out.message = "requested control span has no stable canonical basis"
    out.controls = J(rows(controls),0,.)
    out.relres = .
    out.forward_error = .
    n = rows(controls)
    control_count = cols(controls)
    if (n == 0 | rows(frequency) != n | cols(frequency) != 1 |
        hasmissing(controls) | hasmissing(frequency) |
        min(frequency) <= 0) return(out)
    if (control_count == 0) {
        out.status = "CONVERGED"
        out.message = "no controls require canonicalization"
        out.controls = controls
        out.relres = 0
        out.forward_error = 0
        return(out)
    }
    if (control_count > 32) {
        out.status = "AMBIGUOUS_CONTROL_BASIS"
        out.message = "control count exceeds the certified canonical-basis range"
        return(out)
    }

    // First form an arbitrary weighted-orthonormal basis for the requested
    // control span.  Any invertible input-basis transformation changes this
    // basis only by an orthogonal rotation in exact arithmetic.
    gram = controls' * (frequency :* controls)
    gram_inverse = vckss__inverse(gram,rank_tolerance)
    if (gram_inverse.status != "CONVERGED") {
        out.message = "requested controls are singular before FE absorption"
        return(out)
    }
    whitener = cholesky(gram_inverse.inverse)
    if (hasmissing(whitener)) {
        out.message = "control-span whitening failed"
        return(out)
    }
    orthonormal = controls * whitener
    checked = orthonormal' * (frequency :* orthonormal)
    whitening_error = vckss__norm2(checked-I(control_count))
    margin = max((1e-10,1000*rank_tolerance))
    if (hasmissing(orthonormal) | hasmissing(whitening_error) |
        whitening_error > margin) {
        out.message = "control-span whitening failed its residual gate"
        return(out)
    }
    // Bound the error of a computed row score relative to the exact weighted
    // projector.  The Gram accumulation term is an operator-norm bound over
    // all k columns.  The inverse term first converts the recorded maximum
    // column residual to an operator residual and then uses the denominator
    // in the inverse-perturbation inequality.  The public k<=32 range keeps
    // these bounds finite and explicit.
    summation_error = control_count * vckss__rounding_gamma(2*n) /
        gram_inverse.rcond
    cholesky_error = control_count *
        vckss__rounding_gamma(2*control_count+1) /
        gram_inverse.rcond
    basis_product_error = vckss__rounding_gamma(2*control_count) *
        (1+1/gram_inverse.rcond)
    inverse_forward_error = vckss__inverse_forward_error(
        gram_inverse.relres,gram_inverse.rcond,control_count)
    numerical_error = whitening_error + summation_error + cholesky_error +
        basis_product_error + inverse_forward_error
    if (hasmissing(numerical_error) | numerical_error >= 0.25) {
        out.status = "AMBIGUOUS_CONTROL_BASIS"
        out.message = "control-span whitening error has no certified forward bound"
        return(out)
    }
    numerical_error = numerical_error/(1-numerical_error)
    score_error = 2*numerical_error +
        vckss__rounding_gamma(2*control_count) *
        (1+numerical_error)^2
    // Four score-error envelopes cover the candidate score in each of two
    // parameterizations, the computed maximum used by the cutoff, and the
    // final subtraction/comparison rounding.  The 1e-12 floor preserves the
    // registered exact-boundary fail-closed behavior.
    uncertainty = max((1e-12,4*score_error))
    if (uncertainty >= margin/4) {
        out.status = "AMBIGUOUS_CONTROL_BASIS"
        out.message = "control-span whitening error is too large to certify a canonical anchor"
        return(out)
    }

    // Pivoted row-space selection depends only on inner products between
    // rows of the orthonormal span.  Those inner products are unchanged by
    // the orthogonal rotation above.  The ado layer has already put every
    // controlled backend in one ID-free, control-coordinate-free semantic
    // order; equal-key rows that may permute are fully exchangeable.  Hence
    // the selected anchor and the RREF-style basis below are independent of
    // the user's control coordinates on every accepted exact and JLA path.
    selected = J(control_count,1,.)
    residualized = orthonormal
    for (pivot=1; pivot<=control_count; pivot++) {
        score = rowsum(residualized:^2)
        maximum = max(score)
        if (hasmissing(maximum) | maximum <= margin) {
            out.message = "control-span anchor selection lost rank"
            return(out)
        }
        cutoff = maximum-margin*max((1,maximum))
        boundary = selectindex(abs(score:-cutoff) :<= uncertainty)
        if (rows(boundary) > 0) {
            out.status = "AMBIGUOUS_CONTROL_BASIS"
            out.message = "control-span anchor score is numerically ambiguous at the canonical tie boundary"
            return(out)
        }
        eligible = selectindex(score :> cutoff)
        if (rows(eligible) == 0) {
            out.message = "control-span anchor selection failed"
            return(out)
        }
        chosen = eligible[1]
        selected[pivot] = chosen
        anchor = orthonormal[selected[1..pivot],.]
        anchor_gram = anchor * anchor'
        anchor_inverse = vckss__inverse(anchor_gram,rank_tolerance)
        if (anchor_inverse.status != "CONVERGED") {
            out.message = "control-span anchor is numerically singular"
            return(out)
        }
        anchor_forward_error = vckss__inverse_forward_error(
            anchor_inverse.relres,anchor_inverse.rcond,pivot)
        product_error = vckss__rounding_gamma(6*control_count) *
            (1 + pivot/max((anchor_inverse.rcond,rank_tolerance)))
        projector = anchor'*anchor_inverse.inverse*anchor
        projection_error = vckss__norm2(projector*projector-projector) +
            vckss__norm2(projector-projector')
        residualized = orthonormal-orthonormal*projector
        selected_zero_error = vckss__norm2(
            residualized[selected[1..pivot],.])
        numerical_error = numerical_error + anchor_forward_error +
            product_error + projection_error + selected_zero_error
        if (hasmissing(numerical_error) | numerical_error >= 0.25) {
            out.status = "AMBIGUOUS_CONTROL_BASIS"
            out.message = "control-span anchor update has no certified forward bound"
            return(out)
        }
        score_error = 2*numerical_error +
            vckss__rounding_gamma(2*control_count) *
            (1+numerical_error)^2
        uncertainty = max((1e-12,4*score_error))
        if (uncertainty >= margin/4) {
            out.status = "AMBIGUOUS_CONTROL_BASIS"
            out.message = "control-span anchor error is too large to certify a canonical basis"
            return(out)
        }
    }
    anchor = orthonormal[selected,.]
    anchor_gram = anchor * anchor'
    anchor_inverse = vckss__inverse(anchor_gram,rank_tolerance)
    if (anchor_inverse.status != "CONVERGED") {
        out.message = "complete control-span anchor is numerically singular"
        return(out)
    }
    out.controls = orthonormal * anchor' * anchor_inverse.inverse
    anchor_error = vckss__norm2(out.controls[selected,.]-I(control_count))
    checked = orthonormal' * (frequency :* out.controls)
    span_error = vckss__norm2(out.controls-orthonormal*checked) /
        max((1,vckss__norm2(out.controls)))
    anchor_forward_error = vckss__inverse_forward_error(
        anchor_inverse.relres,anchor_inverse.rcond,control_count)
    product_error = vckss__rounding_gamma(8*control_count) *
        (1+control_count/max((anchor_inverse.rcond,rank_tolerance)))
    canonical_error = anchor_error + span_error + anchor_forward_error +
        product_error
    numerical_error = numerical_error+canonical_error
    if (hasmissing(out.controls) | hasmissing(anchor_error) |
        hasmissing(span_error) | hasmissing(numerical_error) |
        numerical_error >= 0.25 |
        anchor_error > margin) {
        out.message = "canonical control basis failed its anchor residual gate"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "control span mapped to an ID-free canonical basis"
    out.relres = max((gram_inverse.relres,whitening_error,
        anchor_inverse.relres,anchor_error))
    out.forward_error = numerical_error
    return(out)
}

real matrix vckss__design(
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

struct vckss_target_matrices scalar vckss__targets(
    real colvector worker,
    real colvector firm,
    real colvector target_weight,
    real scalar worker_levels,
    real scalar firm_levels,
    real scalar parameter_count)
{
    struct vckss_target_matrices scalar out
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

real scalar vckss__quadratic(
    real colvector coefficient,
    real matrix target)
{
    return((coefficient' * target * coefficient)[1,1])
}

real colvector vckss__target_diagonal(
    real matrix design_inverse,
    real matrix target)
{
    return(rowsum((design_inverse * target) :* design_inverse))
}

struct vckss_result scalar vckss__exact(
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
    struct vckss_result scalar out
    struct vckss_inverse_result scalar full_inverse, working_inverse
    struct vckss_inverse_result scalar deleted_information_inverse
    struct vckss_maker_result scalar reduced_maker
    struct vckss_control_basis_result scalar canonical_controls
    struct vckss_target_matrices scalar targets
    real scalar n, worker_levels, firm_levels, controls_count
    real scalar full_parameters, parameters, group, groups, begin, finish
    real scalar max_leverage, eigmax, minimum_maker
    real scalar inverse_forward_bound, rank_verification_margin, row
    real scalar control_downstream_bound, block_solver_residual
    real matrix full_design, design, information, A, design_inverse
    real matrix deleted_information
    real matrix sorted_delete, panel, block_design, block_inverse
    real matrix low_rank, inverse_factor
    real colvector full_beta, beta, working_y, residual, leverage_diagonal
    real colvector row_order, index, block_frequency, transformed_y
    real colvector transformed_residual, deleted_residual
    real colvector target_left, target_right
    real rowvector plugin, correction, corrected

    out = vckss__empty_result()
    n = rows(y)
    if (n == 0 | cols(y) != 1 | rows(worker) != n | cols(worker) != 1 |
        rows(firm) != n | cols(firm) != 1 | rows(frequency) != n |
        cols(frequency) != 1 | rows(target_weight) != n |
        cols(target_weight) != 1 | rows(deletion_id) != n |
        cols(deletion_id) != 1 | rows(controls) != n) {
        return(vckss__failure("INVALID_INPUT", "KSS inputs have incompatible dimensions"))
    }
    if (hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(target_weight) | hasmissing(deletion_id)) {
        return(vckss__failure("NONFINITE_INPUT", "KSS inputs must be finite"))
    }
    if (min(frequency) <= 0 |
        max(abs(frequency - floor(frequency))) != 0) {
        return(vckss__failure("INVALID_FREQUENCY", "frequency weights must be positive integers"))
    }
    if (missing(vckss__exact_physical_total(frequency))) {
        return(vckss__failure("PHYSICAL_TOTAL_LIMIT", "literal frequency total exceeds the exact binary64 integer range"))
    }
    if (min(target_weight) < 0 | sum(target_weight) <= 0) {
        return(vckss__failure("INVALID_TARGET_WEIGHT", "target weights must be nonnegative with positive mass"))
    }
    if (deletion != "observation" & deletion != "match") {
        return(vckss__failure("UNSUPPORTED_DELETION", "deletion must be observation or match"))
    }
    if (nuisance != "joint" & nuisance != "fixedoffset") {
        return(vckss__failure("INVALID_NUISANCE", "nuisance must be joint or fixedoffset"))
    }
    if (rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        block_tolerance <= 0 | block_tolerance >= 1) {
        return(vckss__failure("INVALID_TOLERANCE", "invalid exact-solver tolerance"))
    }

    worker_levels = max(worker)
    firm_levels = max(firm)
    if (worker_levels < 1 | firm_levels < 2 |
        min(worker) != 1 | min(firm) != 1 |
        max(abs(worker-floor(worker))) != 0 |
        max(abs(firm-floor(firm))) != 0 |
        rows(uniqrows(sort(worker,1))) != worker_levels |
        rows(uniqrows(sort(firm,1))) != firm_levels) {
        return(vckss__failure("INVALID_IDENTIFIER", "worker and firm IDs must be dense positive integers"))
    }
    controls_count = cols(controls)
    full_parameters = worker_levels + firm_levels - 1 + controls_count
    if (full_parameters > exact_limit) {
        return(vckss__failure("EXACT_SIZE_LIMIT", "identified coefficient dimension exceeds exact_limit()"))
    }
    if (controls_count > 0) {
        canonical_controls = vckss__canonical_controls(
            controls,frequency,rank_tolerance)
        if (canonical_controls.status != "CONVERGED") {
            if (canonical_controls.status == "SINGULAR_NUISANCE_BLOCK") {
                return(vckss__failure("SINGULAR_INFORMATION",
                    canonical_controls.message))
            }
            return(vckss__failure(canonical_controls.status,
                canonical_controls.message))
        }
        controls = canonical_controls.controls
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
                return(vckss__failure("BLOCK_SIZE_LIMIT", "a deletion block exceeds blocksize_limit()"))
            }
            if (min(worker[index]) != max(worker[index]) |
                min(firm[index]) != max(firm[index])) {
                return(vckss__failure("CROSS_COORDINATE_MATCH", "each deletion ID must remain within one worker-firm coordinate"))
            }
        }
    }
    else {
        groups = sum(frequency)
    }

    full_design = vckss__design(
        worker, firm, controls, worker_levels, firm_levels)
    information = full_design' * (frequency :* full_design)
    full_inverse = vckss__inverse(information,rank_tolerance)
    if (full_inverse.status != "CONVERGED") {
        if (full_inverse.status == "SINGULAR_INFORMATION") {
            return(vckss__failure("SINGULAR_INFORMATION", "full weighted design is unidentified or disconnected"))
        }
        return(vckss__failure(full_inverse.status, "full weighted inverse failed its residual gate"))
    }
    if (controls_count > 0) {
        if (full_inverse.rcond <= canonical_controls.forward_error) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "canonical-control error exhausts the full-design conditioning margin"))
        }
        control_downstream_bound = vckss__propagate_error(
            canonical_controls.forward_error,full_inverse.rcond)
        if (hasmissing(control_downstream_bound) |
            control_downstream_bound > vckss__control_forward_limit()) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "full-design conditioning cannot certify control-basis invariance at the registered tolerance"))
        }
    }
    full_beta = full_inverse.inverse * (full_design' * (frequency :* y))
    if (hasmissing(full_beta)) {
        return(vckss__failure("NONFINITE_FIT", "full weighted least-squares fit is nonfinite"))
    }

    if (nuisance == "fixedoffset" & controls_count > 0) {
        working_y = y - controls *
            full_beta[(full_parameters-controls_count+1)..full_parameters]
        parameters = worker_levels + firm_levels - 1
        design = full_design[.,1..parameters]
        information = design' * (frequency :* design)
        working_inverse = vckss__inverse(information,rank_tolerance)
        if (working_inverse.status != "CONVERGED") {
            return(vckss__failure(working_inverse.status, "fixed-offset two-way inverse failed"))
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
        return(vckss__failure("INVERSE_FORWARD_ERROR_FAILED", "working information inverse is too ill-conditioned for a fail-closed deletion-rank gate"))
    }
    rank_verification_margin = max((block_tolerance,
        10*inverse_forward_bound))
    beta = A * (design' * (frequency :* working_y))
    residual = working_y - design * beta
    if (hasmissing(beta) | hasmissing(residual)) {
        return(vckss__failure("NONFINITE_FIT", "working least-squares fit is nonfinite"))
    }

    targets = vckss__targets(
        worker, firm, target_weight, worker_levels, firm_levels, parameters)
    plugin = J(1,4,0)
    plugin[1] = vckss__quadratic(beta,targets.worker)
    plugin[2] = vckss__quadratic(beta,targets.firm)
    plugin[3] = vckss__quadratic(beta,targets.covariance)
    plugin[4] = plugin[1] + plugin[2] + 2 * plugin[3]
    correction = J(1,4,0)
    max_leverage = 0
    block_solver_residual = 0
    design_inverse = design * A
    if (deletion == "match") {
        inverse_factor = cholesky(A)
        if (hasmissing(inverse_factor) |
            vckss__norm2(inverse_factor*inverse_factor'-A) >
            100*rank_tolerance*(1+vckss__norm2(A))) {
            return(vckss__failure("INVERSE_RESIDUAL_FAILED", "working inverse square root failed its residual gate"))
        }
    }
    timer_off(91)
    timer_on(92)

    if (deletion == "observation") {
        leverage_diagonal = rowsum(design_inverse :* design)
        if (min(leverage_diagonal) < -100 * rank_tolerance) {
            return(vckss__failure("NONESTIMABLE_DELETION", "physical observation deletion has leverage at or above one"))
        }
        for (row=1; row<=n; row++) {
            minimum_maker = 1-leverage_diagonal[row]
            if (minimum_maker <= block_tolerance) {
                return(vckss__failure("NONESTIMABLE_DELETION", "physical observation deletion has leverage at or above one"))
            }
            if (controls_count > 0 & nuisance == "joint") {
                control_downstream_bound =
                    vckss__propagate_error(
                    canonical_controls.forward_error,minimum_maker)
                if (hasmissing(control_downstream_bound) |
                    control_downstream_bound >
                    vckss__control_forward_limit()) {
                    return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "observation-deletion conditioning cannot certify control-basis invariance"))
                }
            }
            if (minimum_maker <= rank_verification_margin) {
                deleted_information = information -
                    design[row,.]'*design[row,.]
                deleted_information_inverse = vckss__inverse(
                    deleted_information,rank_tolerance)
                if (deleted_information_inverse.status != "CONVERGED") {
                    return(vckss__failure("NONESTIMABLE_DELETION", "a direct deleted-information factorization rejects physical observation deletion"))
                }
            }
        }
        max_leverage = max(leverage_diagonal)
        correction[1] = sum(frequency :* working_y :* residual :*
            vckss__target_diagonal(design_inverse,targets.worker) :/
            (1 :- leverage_diagonal))
        correction[2] = sum(frequency :* working_y :* residual :*
            vckss__target_diagonal(design_inverse,targets.firm) :/
            (1 :- leverage_diagonal))
        correction[3] = sum(frequency :* working_y :* residual :*
            vckss__target_diagonal(design_inverse,targets.covariance) :/
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
            low_rank = block_design*inverse_factor
            reduced_maker = vckss__low_rank_maker(
                low_rank,block_frequency:*residual[index],
                rank_tolerance,block_tolerance)
            if (reduced_maker.status != "CONVERGED") {
                return(vckss__failure(
                    reduced_maker.status,reduced_maker.message))
            }
            block_solver_residual = max((
                block_solver_residual,reduced_maker.relres))
            eigmax = reduced_maker.eigmax
            minimum_maker = 1 - eigmax
            if (minimum_maker <= block_tolerance) {
                return(vckss__failure("NONESTIMABLE_DELETION", "a match deletion loses identified design rank"))
            }
            if (controls_count > 0 & nuisance == "joint") {
                control_downstream_bound =
                    vckss__propagate_error(
                    canonical_controls.forward_error,minimum_maker)
                if (hasmissing(control_downstream_bound) |
                    control_downstream_bound >
                    vckss__control_forward_limit()) {
                    return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "match-deletion conditioning cannot certify control-basis invariance"))
                }
            }
            if (minimum_maker <= rank_verification_margin) {
                deleted_information = information -
                    block_design'*block_design
                deleted_information_inverse = vckss__inverse(
                    deleted_information,rank_tolerance)
                if (deleted_information_inverse.status != "CONVERGED") {
                    return(vckss__failure("NONESTIMABLE_DELETION", "a direct deleted-information factorization rejects match deletion"))
                }
            }
            max_leverage = max((max_leverage,eigmax))
            transformed_y = block_frequency :* working_y[index]
            transformed_residual = block_frequency :* residual[index]
            deleted_residual = reduced_maker.actions
            target_left = block_inverse'*transformed_y
            target_right = block_inverse'*deleted_residual
            correction[1] = correction[1] +
                (target_left' * targets.worker * target_right)[1,1]
            correction[2] = correction[2] +
                (target_left' * targets.firm * target_right)[1,1]
            correction[3] = correction[3] +
                (target_left' * targets.covariance * target_right)[1,1]
        }
    }
    correction[4] = correction[1] + correction[2] + 2 * correction[3]
    timer_off(92)
    if (hasmissing(plugin) | hasmissing(correction)) {
        return(vckss__failure("NONFINITE_CORRECTION", "exact KSS correction is nonfinite"))
    }
    corrected = plugin-correction
    if (hasmissing(corrected)) {
        return(vckss__failure("NONFINITE_CORRECTED_TARGET", "exact corrected target is nonfinite"))
    }

    out.status = "CONVERGED"
    out.message = "exact KSS calculation converged"
    out.plugin = plugin
    out.correction = correction
    out.corrected = corrected
    out.numerical_mcse = J(1,4,0)
    out.n_stored = n
    out.n_physical = sum(frequency)
    out.worker_levels = worker_levels
    out.firm_levels = firm_levels
    out.parameters = parameters
    out.full_parameters = full_parameters
    out.correction_parameters = parameters
    out.deletion_units = groups
    out.target_weight_sum = sum(target_weight)
    out.max_leverage = max_leverage
    out.information_rcond = min((full_inverse.rcond,working_inverse.rcond))
    out.preconditioner_ratio = .
    out.control_schur_rcond = .
    if (controls_count > 0) {
        out.inverse_relres = max((full_inverse.relres,working_inverse.relres,
            canonical_controls.relres,block_solver_residual))
    }
    else out.inverse_relres = max((full_inverse.relres,working_inverse.relres,
        block_solver_residual))
    out.weighted_rss = sum(frequency:*residual:^2)
    out.fit_seconds = vckss__timer_seconds(91)
    out.leverage_seconds = 0
    out.target_seconds = 0
    out.correction_seconds = vckss__timer_seconds(92)
    out.preconditioner_seconds = 0
    out.schur_seconds = 0
    out.preconditioner_apply_seconds = 0
    out.pcg_seconds = 0
    out.solver_backend_seconds = 0
    out.solver_iterations = 0
    out.solver_max_residual = out.inverse_relres
    out.solver_schur_actions = 0
    out.solver_schur_batches = 0
    out.solver_precond_applications = 0
    out.solver_precond_batches = 0
    out.solver_rhs_diagnostics = J(0,6,.)
    out.probes = 0
    return(out)
}

/* The separately labelled stayer hybrid uses one combined full-sample fit
   and one pooled target normalization, but two scientifically distinct
   deletion conventions.  Retained mover matches are deleted as blocks;
   eligible one-firm stayers are deleted one literal physical copy at a time.
   The ado layer constructs and labels those populations.  This kernel never
   infers stayer status from the post-pruning graph. */
struct vckss_result scalar vckss__exact_stayer_hybrid(
    real colvector y,
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector target_weight,
    real colvector deletion_id,
    real colvector stayer,
    string scalar nuisance,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar exact_limit,
    real scalar blocksize_limit)
{
    struct vckss_result scalar out
    struct vckss_inverse_result scalar full_inverse, working_inverse
    struct vckss_inverse_result scalar deleted_information_inverse
    struct vckss_maker_result scalar reduced_maker
    struct vckss_control_basis_result scalar canonical_controls
    struct vckss_target_matrices scalar targets
    real scalar n, worker_levels, firm_levels, controls_count
    real scalar full_parameters, parameters, group, mover_groups
    real scalar begin, finish, max_leverage, eigmax, minimum_maker
    real scalar inverse_forward_bound, rank_verification_margin, row
    real scalar control_downstream_bound, block_solver_residual
    real matrix full_design, design, information, A, design_inverse
    real matrix deleted_information, sorted_delete, panel
    real matrix block_design, block_inverse, low_rank, inverse_factor
    real colvector full_beta, beta, working_y, residual
    real colvector leverage_diagonal, row_order, index, mover_index
    real colvector stayer_index, block_frequency, transformed_y
    real colvector transformed_residual, deleted_residual
    real colvector target_left, target_right, target_diagonal_worker
    real colvector target_diagonal_firm, target_diagonal_covariance
    real rowvector plugin, correction, corrected
    real rowvector mover_correction, stayer_correction

    out = vckss__empty_result()
    n = rows(y)
    if (n == 0 | cols(y) != 1 | rows(worker) != n | cols(worker) != 1 |
        rows(firm) != n | cols(firm) != 1 | rows(frequency) != n |
        cols(frequency) != 1 | rows(target_weight) != n |
        cols(target_weight) != 1 | rows(deletion_id) != n |
        cols(deletion_id) != 1 | rows(stayer) != n | cols(stayer) != 1 |
        rows(controls) != n) {
        return(vckss__failure("INVALID_INPUT", "stayer-hybrid inputs have incompatible dimensions"))
    }
    if (hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(target_weight) | hasmissing(deletion_id) |
        hasmissing(stayer)) {
        return(vckss__failure("NONFINITE_INPUT", "stayer-hybrid inputs must be finite"))
    }
    if (min(stayer) < 0 | max(stayer) > 1 |
        max(abs(stayer-floor(stayer))) != 0) {
        return(vckss__failure("INVALID_INPUT", "stayer indicator must contain only zero and one"))
    }
    mover_index = selectindex(stayer :== 0)
    stayer_index = selectindex(stayer :== 1)
    if (rows(mover_index) == 0) {
        return(vckss__failure("NO_MOVER_SAMPLE", "stayer hybrid requires a retained mover sample"))
    }
    if (min(frequency) <= 0 |
        max(abs(frequency - floor(frequency))) != 0) {
        return(vckss__failure("INVALID_FREQUENCY", "frequency weights must be positive integers"))
    }
    if (missing(vckss__exact_physical_total(frequency))) {
        return(vckss__failure("PHYSICAL_TOTAL_LIMIT", "literal frequency total exceeds the exact binary64 integer range"))
    }
    if (min(target_weight) < 0 | sum(target_weight) <= 0) {
        return(vckss__failure("INVALID_TARGET_WEIGHT", "target weights must be nonnegative with positive mass"))
    }
    if (nuisance != "joint" & nuisance != "fixedoffset") {
        return(vckss__failure("INVALID_NUISANCE", "nuisance must be joint or fixedoffset"))
    }
    if (rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        block_tolerance <= 0 | block_tolerance >= 1) {
        return(vckss__failure("INVALID_TOLERANCE", "invalid exact-solver tolerance"))
    }

    worker_levels = max(worker)
    firm_levels = max(firm)
    if (worker_levels < 1 | firm_levels < 2 |
        min(worker) != 1 | min(firm) != 1 |
        max(abs(worker-floor(worker))) != 0 |
        max(abs(firm-floor(firm))) != 0 |
        rows(uniqrows(sort(worker,1))) != worker_levels |
        rows(uniqrows(sort(firm,1))) != firm_levels) {
        return(vckss__failure("INVALID_IDENTIFIER", "worker and firm IDs must be dense positive integers"))
    }
    controls_count = cols(controls)
    full_parameters = worker_levels + firm_levels - 1 + controls_count
    if (full_parameters > exact_limit) {
        return(vckss__failure("EXACT_SIZE_LIMIT", "stayer-hybrid identified coefficient dimension exceeds exact_limit()"))
    }
    if (controls_count > 0) {
        canonical_controls = vckss__canonical_controls(
            controls,frequency,rank_tolerance)
        if (canonical_controls.status != "CONVERGED") {
            if (canonical_controls.status == "SINGULAR_NUISANCE_BLOCK") {
                return(vckss__failure("SINGULAR_INFORMATION",
                    canonical_controls.message))
            }
            return(vckss__failure(canonical_controls.status,
                canonical_controls.message))
        }
        controls = canonical_controls.controls
    }

    row_order = mover_index[order(deletion_id[mover_index],1)]
    sorted_delete = deletion_id[row_order]
    panel = panelsetup(sorted_delete,1)
    mover_groups = rows(panel)
    for (group=1; group<=mover_groups; group++) {
        begin = panel[group,1]
        finish = panel[group,2]
        index = row_order[|begin \ finish|]
        if (rows(index) > blocksize_limit) {
            return(vckss__failure("BLOCK_SIZE_LIMIT", "a mover deletion block exceeds blocksize_limit()"))
        }
        if (min(worker[index]) != max(worker[index]) |
            min(firm[index]) != max(firm[index])) {
            return(vckss__failure("CROSS_COORDINATE_MATCH", "each mover deletion ID must remain within one worker-firm coordinate"))
        }
    }

    timer_clear(91)
    timer_clear(92)
    timer_on(91)
    full_design = vckss__design(
        worker, firm, controls, worker_levels, firm_levels)
    information = full_design' * (frequency :* full_design)
    full_inverse = vckss__inverse(information,rank_tolerance)
    if (full_inverse.status != "CONVERGED") {
        if (full_inverse.status == "SINGULAR_INFORMATION") {
            return(vckss__failure("SINGULAR_INFORMATION", "combined mover-stayer design is unidentified or disconnected"))
        }
        return(vckss__failure(full_inverse.status, "combined weighted inverse failed its residual gate"))
    }
    if (controls_count > 0) {
        if (full_inverse.rcond <= canonical_controls.forward_error) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "canonical-control error exhausts the combined-design conditioning margin"))
        }
        control_downstream_bound = vckss__propagate_error(
            canonical_controls.forward_error,full_inverse.rcond)
        if (hasmissing(control_downstream_bound) |
            control_downstream_bound > vckss__control_forward_limit()) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "combined-design conditioning cannot certify control-basis invariance at the registered tolerance"))
        }
    }
    full_beta = full_inverse.inverse *
        (full_design' * (frequency :* y))
    if (hasmissing(full_beta)) {
        return(vckss__failure("NONFINITE_FIT", "combined weighted least-squares fit is nonfinite"))
    }

    if (nuisance == "fixedoffset" & controls_count > 0) {
        working_y = y - controls *
            full_beta[(full_parameters-controls_count+1)..full_parameters]
        parameters = worker_levels + firm_levels - 1
        design = full_design[.,1..parameters]
        information = design' * (frequency :* design)
        working_inverse = vckss__inverse(information,rank_tolerance)
        if (working_inverse.status != "CONVERGED") {
            return(vckss__failure(working_inverse.status,
                "combined fixed-offset two-way inverse failed"))
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
        return(vckss__failure("INVERSE_FORWARD_ERROR_FAILED", "combined working inverse is too ill-conditioned for a fail-closed deletion-rank gate"))
    }
    rank_verification_margin = max((block_tolerance,
        10*inverse_forward_bound))
    beta = A * (design' * (frequency :* working_y))
    residual = working_y - design * beta
    if (hasmissing(beta) | hasmissing(residual)) {
        return(vckss__failure("NONFINITE_FIT", "combined working fit is nonfinite"))
    }

    targets = vckss__targets(
        worker, firm, target_weight, worker_levels, firm_levels, parameters)
    plugin = J(1,4,0)
    plugin[1] = vckss__quadratic(beta,targets.worker)
    plugin[2] = vckss__quadratic(beta,targets.firm)
    plugin[3] = vckss__quadratic(beta,targets.covariance)
    plugin[4] = plugin[1] + plugin[2] + 2*plugin[3]
    mover_correction = J(1,4,0)
    stayer_correction = J(1,4,0)
    max_leverage = 0
    block_solver_residual = 0
    design_inverse = design * A
    inverse_factor = cholesky(A)
    if (hasmissing(inverse_factor) |
        vckss__norm2(inverse_factor*inverse_factor'-A) >
        100*rank_tolerance*(1+vckss__norm2(A))) {
        return(vckss__failure("INVERSE_RESIDUAL_FAILED", "combined working inverse square root failed its residual gate"))
    }
    timer_off(91)
    timer_on(92)

    /* Mover contribution: delete every physical copy in the declared match. */
    for (group=1; group<=mover_groups; group++) {
        begin = panel[group,1]
        finish = panel[group,2]
        index = row_order[|begin \ finish|]
        block_frequency = sqrt(frequency[index])
        block_design = block_frequency :* design[index,.]
        block_inverse = block_design * A
        low_rank = block_design*inverse_factor
        reduced_maker = vckss__low_rank_maker(
            low_rank,block_frequency:*residual[index],
            rank_tolerance,block_tolerance)
        if (reduced_maker.status != "CONVERGED") {
            return(vckss__failure(
                reduced_maker.status,reduced_maker.message))
        }
        block_solver_residual = max((
            block_solver_residual,reduced_maker.relres))
        eigmax = reduced_maker.eigmax
        minimum_maker = 1-eigmax
        if (minimum_maker <= block_tolerance) {
            return(vckss__failure("NONESTIMABLE_DELETION", "a mover match deletion loses combined-design rank"))
        }
        if (controls_count > 0 & nuisance == "joint") {
            control_downstream_bound = vckss__propagate_error(
                canonical_controls.forward_error,minimum_maker)
            if (hasmissing(control_downstream_bound) |
                control_downstream_bound > vckss__control_forward_limit()) {
                return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "mover-match deletion conditioning cannot certify control-basis invariance"))
            }
        }
        if (minimum_maker <= rank_verification_margin) {
            deleted_information = information-block_design'*block_design
            deleted_information_inverse = vckss__inverse(
                deleted_information,rank_tolerance)
            if (deleted_information_inverse.status != "CONVERGED") {
                return(vckss__failure("NONESTIMABLE_DELETION", "a direct deleted-information factorization rejects a mover match deletion"))
            }
        }
        max_leverage = max((max_leverage,eigmax))
        transformed_y = block_frequency :* working_y[index]
        transformed_residual = block_frequency :* residual[index]
        deleted_residual = reduced_maker.actions
        target_left = block_inverse'*transformed_y
        target_right = block_inverse'*deleted_residual
        mover_correction[1] = mover_correction[1] +
            (target_left'*targets.worker*target_right)[1,1]
        mover_correction[2] = mover_correction[2] +
            (target_left'*targets.firm*target_right)[1,1]
        mover_correction[3] = mover_correction[3] +
            (target_left'*targets.covariance*target_right)[1,1]
    }
    mover_correction[4] = mover_correction[1] +
        mover_correction[2] + 2*mover_correction[3]

    /* Stayer contribution: delete one literal copy.  Frequency multiplies
       the contribution because a stored row represents that many exchangeable
       physical deletions; the leverage in each denominator is per copy. */
    if (rows(stayer_index) > 0) {
        leverage_diagonal = rowsum(design_inverse :* design)
        target_diagonal_worker =
            vckss__target_diagonal(design_inverse,targets.worker)
        target_diagonal_firm =
            vckss__target_diagonal(design_inverse,targets.firm)
        target_diagonal_covariance =
            vckss__target_diagonal(design_inverse,targets.covariance)
        for (row=1; row<=rows(stayer_index); row++) {
            index = stayer_index[row]
            if (leverage_diagonal[index] < -100*rank_tolerance) {
                return(vckss__failure("NONESTIMABLE_DELETION", "a stayer physical-observation leverage is invalid"))
            }
            minimum_maker = 1-leverage_diagonal[index]
            if (minimum_maker <= block_tolerance) {
                return(vckss__failure("NONESTIMABLE_DELETION", "a stayer physical-observation deletion loses combined-design rank"))
            }
            if (controls_count > 0 & nuisance == "joint") {
                control_downstream_bound = vckss__propagate_error(
                    canonical_controls.forward_error,minimum_maker)
                if (hasmissing(control_downstream_bound) |
                    control_downstream_bound > vckss__control_forward_limit()) {
                    return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "stayer-observation deletion conditioning cannot certify control-basis invariance"))
                }
            }
            if (minimum_maker <= rank_verification_margin) {
                deleted_information = information-
                    design[index,.]'*design[index,.]
                deleted_information_inverse = vckss__inverse(
                    deleted_information,rank_tolerance)
                if (deleted_information_inverse.status != "CONVERGED") {
                    return(vckss__failure("NONESTIMABLE_DELETION", "a direct deleted-information factorization rejects a stayer physical-observation deletion"))
                }
            }
            max_leverage = max((max_leverage,leverage_diagonal[index]))
        }
        stayer_correction[1] = sum(frequency[stayer_index] :*
            working_y[stayer_index] :* residual[stayer_index] :*
            target_diagonal_worker[stayer_index] :/
            (1 :- leverage_diagonal[stayer_index]))
        stayer_correction[2] = sum(frequency[stayer_index] :*
            working_y[stayer_index] :* residual[stayer_index] :*
            target_diagonal_firm[stayer_index] :/
            (1 :- leverage_diagonal[stayer_index]))
        stayer_correction[3] = sum(frequency[stayer_index] :*
            working_y[stayer_index] :* residual[stayer_index] :*
            target_diagonal_covariance[stayer_index] :/
            (1 :- leverage_diagonal[stayer_index]))
        stayer_correction[4] = stayer_correction[1] +
            stayer_correction[2] + 2*stayer_correction[3]
    }

    correction = mover_correction+stayer_correction
    correction[4] = correction[1]+correction[2]+2*correction[3]
    timer_off(92)
    if (hasmissing(plugin) | hasmissing(correction)) {
        return(vckss__failure("NONFINITE_CORRECTION", "stayer-hybrid exact KSS correction is nonfinite"))
    }
    corrected = plugin-correction
    if (hasmissing(corrected)) {
        return(vckss__failure("NONFINITE_CORRECTED_TARGET", "stayer-hybrid corrected target is nonfinite"))
    }

    out.status = "CONVERGED"
    out.message = "mixed-deletion stayer-hybrid exact KSS calculation converged"
    out.plugin = plugin
    out.correction = correction
    out.corrected = corrected
    out.numerical_mcse = J(1,4,0)
    out.correction_by_source = mover_correction \ stayer_correction
    out.n_stored = n
    out.n_physical = sum(frequency)
    out.worker_levels = worker_levels
    out.firm_levels = firm_levels
    out.parameters = parameters
    out.full_parameters = full_parameters
    out.correction_parameters = parameters
    if (rows(stayer_index) > 0) {
        out.deletion_units = mover_groups+sum(frequency[stayer_index])
    }
    else out.deletion_units = mover_groups
    out.target_weight_sum = sum(target_weight)
    out.max_leverage = max_leverage
    out.information_rcond = min((full_inverse.rcond,working_inverse.rcond))
    out.preconditioner_ratio = .
    out.control_schur_rcond = .
    if (controls_count > 0) {
        out.inverse_relres = max((full_inverse.relres,working_inverse.relres,
            canonical_controls.relres,block_solver_residual))
    }
    else out.inverse_relres = max((full_inverse.relres,
        working_inverse.relres,block_solver_residual))
    out.weighted_rss = sum(frequency:*residual:^2)
    out.fit_seconds = vckss__timer_seconds(91)
    out.leverage_seconds = 0
    out.target_seconds = 0
    out.correction_seconds = vckss__timer_seconds(92)
    out.preconditioner_seconds = 0
    out.schur_seconds = 0
    out.preconditioner_apply_seconds = 0
    out.pcg_seconds = 0
    out.solver_backend_seconds = 0
    out.solver_iterations = 0
    out.solver_max_residual = out.inverse_relres
    out.solver_schur_actions = 0
    out.solver_schur_batches = 0
    out.solver_precond_applications = 0
    out.solver_precond_batches = 0
    out.solver_rhs_diagnostics = J(0,6,.)
    out.probes = 0
    return(out)
}

struct vckss_fe_design
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
    real scalar external_operator
    real scalar persistent_bytes
    pointer scalar operator_context
    pointer scalar operator_transpose_full
    pointer scalar operator_predict
    pointer scalar operator_schur_action
    pointer scalar operator_schur_into
    pointer scalar operator_worker_base
    pointer scalar operator_worker_to_firm
    pointer scalar operator_firm_to_worker
    pointer scalar operator_wtranspose
    pointer scalar operator_diagonal_apply
}

struct vckss_fe_workspace
{
    real matrix cell_buffer
    real matrix worker_buffer
    real scalar width
    real scalar modeled_bytes
}

struct vckss_solve_result
{
    string scalar status
    string scalar message
    string rowvector rhs_status
    real matrix coefficient
    real matrix prediction
    real scalar iterations
    real scalar relres
    real rowvector rhs_iterations
    real rowvector rhs_relres
    real scalar schur_actions
    real scalar schur_batches
    real scalar preconditioner_applications
    real scalar preconditioner_batches
    real scalar schur_seconds
    real scalar preconditioner_seconds
    real scalar pcg_seconds
    real scalar workspace_builds
    real scalar buffered_schur_batches
    real scalar legacy_schur_batches
    real scalar buffered_schur_columns
    real scalar legacy_schur_columns
    real scalar packed_fallback_batches
    real scalar max_buffer_width
    real scalar workspace_peak_bytes
    real scalar cell_bytes_avoided
}

real rowvector vckss__fe_profile_row(
    struct vckss_solve_result scalar solved)
{
    return((solved.workspace_builds,solved.buffered_schur_batches,
        solved.legacy_schur_batches,solved.buffered_schur_columns,
        solved.legacy_schur_columns,solved.packed_fallback_batches,
        solved.max_buffer_width,solved.workspace_peak_bytes,
        solved.cell_bytes_avoided))
}

real rowvector vckss__fe_profile_merge(
    real rowvector left,
    real rowvector right)
{
    real rowvector out

    out = left+right
    out[7] = max((left[7],right[7]))
    out[8] = max((left[8],right[8]))
    return(out)
}

struct vckss_preconditioner_result
{
    string scalar status
    string scalar message
    real matrix value
}

struct vckss_solver_backend
{
    string scalar route
    pointer scalar context
    pointer scalar apply
    real scalar exact_inverse
}

struct vckss_preconditioner_result scalar vckss__diagonal_apply(
    pointer scalar context,
    struct vckss_fe_design scalar design,
    real matrix residual)
{
    struct vckss_preconditioner_result scalar out

    context = context
    out.status = "INVALID_INPUT"
    out.message = "invalid diagonal-preconditioner input"
    out.value = J(0,0,.)
    if (design.status != "CONVERGED" |
        rows(residual) != design.firm_levels |
        cols(residual) < 1 | hasmissing(residual)) return(out)
    if (design.external_operator) {
        if (design.operator_diagonal_apply == NULL |
            design.operator_context == NULL) return(out)
        return((*design.operator_diagonal_apply)(
            design.operator_context,design,residual))
    }
    out.value = residual :/ design.schur_diagonal
    out.value = out.value -
        J(design.firm_levels,1,1)*(colsum(out.value):/design.firm_levels)
    if (hasmissing(out.value)) return(out)
    out.status = "CONVERGED"
    out.message = "diagonal preconditioner applied"
    return(out)
}

struct vckss_solver_backend scalar vckss__diagonal_backend()
{
    struct vckss_solver_backend scalar out

    out.route = "DIAGONAL"
    out.context = NULL
    out.apply = &vckss__diagonal_apply()
    out.exact_inverse = 0
    return(out)
}

struct vckss_joint_design
{
    string scalar status
    string scalar message
    struct vckss_fe_design scalar base
    struct vckss_solver_backend scalar backend
    real matrix controls
    real matrix cross
    real matrix base_cross_inverse
    real matrix residualized_controls
    real matrix schur_inverse
    real scalar schur_rcond
    real scalar preparation_relres
    real scalar preparation_iterations
    real rowvector preparation_rhs_iterations
    real rowvector preparation_rhs_relres
    real scalar preparation_schur_actions
    real scalar preparation_schur_batches
    real scalar preparation_precond_applications
    real scalar preparation_precond_batches
    real scalar preparation_schur_seconds
    real scalar preparation_precond_seconds
    real scalar preparation_pcg_seconds
    real rowvector preparation_fe_profile
}

real matrix vckss__solver_trace_rows(
    real scalar stage,
    real scalar batch_id,
    real rowvector iterations,
    real rowvector relres)
{
    real scalar column, columns, logical_rhs
    real matrix out

    columns = cols(iterations)
    if (columns == 0 | cols(relres) != columns) return(J(0,6,.))
    out = J(columns,6,.)
    for (column=1; column<=columns; column++) {
        if (stage == 4) logical_rhs = batch_id+column-1
        else if (stage == 5) logical_rhs = 2*(batch_id-1)+column
        else logical_rhs = column
        out[column,.] =
            (stage,batch_id,logical_rhs,iterations[column],relres[column],1)
    }
    return(out)
}

real matrix vckss__group_sum(
    real matrix values,
    real colvector row_order,
    real matrix panel)
{
    return(panelsum(values[row_order,.],panel))
}

struct vckss_fe_design scalar vckss__fe_prepare(
    real colvector worker,
    real colvector firm,
    real colvector frequency,
    real scalar rank_tolerance)
{
    struct vckss_fe_design scalar out
    real colvector pair_order, pair_first, pair_worker, pair_firm, pair_code
    real colvector pair_weight, adjustment, firm_pair_order
    real matrix pair_panel, firm_pair_panel
    real scalar worker_levels, firm_levels

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
    out.external_operator = 0
    out.persistent_bytes = .
    out.operator_context = NULL
    out.operator_transpose_full = NULL
    out.operator_predict = NULL
    out.operator_schur_action = NULL
    out.operator_schur_into = NULL
    out.operator_worker_base = NULL
    out.operator_worker_to_firm = NULL
    out.operator_firm_to_worker = NULL
    out.operator_wtranspose = NULL
    out.operator_diagonal_apply = NULL

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
    out.worker_weight = vckss__group_sum(
        frequency,out.worker_order,out.worker_panel)
    out.firm_weight = vckss__group_sum(
        frequency,out.firm_order,out.firm_panel)

    pair_order = order((worker,firm),(1,2))
    pair_code = J(rows(worker),1,1)
    if (rows(worker) > 1) {
        pair_code[2..rows(worker)] = 1 :+ runningsum(rowsum(
            (worker[pair_order[2..rows(worker)]],
             firm[pair_order[2..rows(worker)]]) :!=
            (worker[pair_order[1..(rows(worker)-1)]],
             firm[pair_order[1..(rows(worker)-1)]])) :> 0)
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
    out.schur_diagonal = out.firm_weight - adjustment
    if (hasmissing(out.schur_diagonal) | min(out.schur_diagonal) <= 0) {
        out.status = "SINGULAR_INFORMATION"
        out.message = "firm mobility quotient system has a nonpositive diagonal"
        return(out)
    }
    out.preconditioner_ratio = min(out.schur_diagonal) /
        max(out.schur_diagonal)
    out.persistent_bytes = 8*(8*out.n+
        5*(out.worker_levels+out.firm_levels))
    out.status = "CONVERGED"
    out.message = "two-way matrix-free design prepared"
    return(out)
}

real matrix vckss__fe_predict(
    struct vckss_fe_design scalar design,
    real matrix coefficient)
{
    real matrix firm_coefficient, fitted
    real scalar columns

    if (design.external_operator) {
        if (design.operator_predict == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_predict)(
            design.operator_context,design,coefficient))
    }
    columns = cols(coefficient)
    firm_coefficient = J(design.firm_levels,columns,0)
    firm_coefficient[1..(design.firm_levels-1),.] =
        coefficient[(design.worker_levels+1)..rows(coefficient),.]
    fitted = coefficient[1..design.worker_levels,.][design.worker,.] +
        firm_coefficient[design.firm,.]
    return(fitted)
}

real matrix vckss__fe_transpose_full(
    struct vckss_fe_design scalar design,
    real matrix values)
{
    real matrix worker_part, firm_part

    if (design.external_operator) {
        if (design.operator_transpose_full == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_transpose_full)(
            design.operator_context,design,values))
    }
    worker_part = vckss__group_sum(
        values,design.worker_order,design.worker_panel)
    firm_part = vckss__group_sum(
        values,design.firm_order,design.firm_panel)
    return(worker_part \ firm_part)
}

real matrix vckss__fe_transpose(
    struct vckss_fe_design scalar design,
    real matrix values)
{
    real matrix full

    full = vckss__fe_transpose_full(design,values)
    if (rows(full) != design.worker_levels+design.firm_levels) {
        return(J(0,0,.))
    }
    return(full[1..(design.worker_levels+design.firm_levels-1),.])
}

real matrix vckss__fe_schur_action(
    struct vckss_fe_design scalar design,
    real matrix firm_coefficient)
{
    real matrix fitted, worker_mean, residual, firm_sum

    if (design.external_operator) {
        if (design.operator_schur_action == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_schur_action)(
            design.operator_context,design,firm_coefficient))
    }
    fitted = firm_coefficient[design.firm,.]
    worker_mean = vckss__group_sum(
        design.frequency :* fitted,
        design.worker_order,design.worker_panel) :/ design.worker_weight
    residual = fitted - worker_mean[design.worker,.]
    firm_sum = vckss__group_sum(
        design.frequency :* residual,
        design.firm_order,design.firm_panel)
    return(firm_sum)
}

struct vckss_fe_workspace scalar vckss__fe_workspace_init(
    struct vckss_fe_design scalar design,
    real scalar width)
{
    struct vckss_fe_workspace scalar out

    out.width = width
    out.modeled_bytes = 8*width*(design.n+design.worker_levels+
        design.firm_levels)
    out.cell_buffer = J(design.n,width,0)
    out.worker_buffer = J(design.worker_levels,width,0)
    return(out)
}

void vckss__fe_schur_into(
    struct vckss_fe_design scalar design,
    real matrix firm_coefficient,
    pointer(struct vckss_fe_workspace scalar) scalar workspace,
    pointer(real matrix) scalar destination)
{
    if (design.external_operator) {
        (*design.operator_schur_into)(design.operator_context,design,
            firm_coefficient,workspace,destination)
        return
    }
    (*workspace).cell_buffer = firm_coefficient[design.firm,.]
    (*workspace).worker_buffer = vckss__group_sum(
        design.frequency:*(*workspace).cell_buffer,
        design.worker_order,design.worker_panel):/design.worker_weight
    (*workspace).cell_buffer = (*workspace).cell_buffer -
        (*workspace).worker_buffer[design.worker,.]
    (*destination) = vckss__group_sum(
        design.frequency:*(*workspace).cell_buffer,
        design.firm_order,design.firm_panel)
}

real matrix vckss__fe_worker_base(
    struct vckss_fe_design scalar design,
    real matrix worker_rhs)
{
    if (design.external_operator) {
        if (design.operator_worker_base == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_worker_base)(
            design.operator_context,design,worker_rhs))
    }
    return(worker_rhs:/design.worker_weight)
}

real matrix vckss__fe_worker_to_firm(
    struct vckss_fe_design scalar design,
    real matrix worker_value)
{
    if (design.external_operator) {
        if (design.operator_worker_to_firm == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_worker_to_firm)(
            design.operator_context,design,worker_value))
    }
    return(vckss__group_sum(
        design.frequency:*worker_value[design.worker,.],
        design.firm_order,design.firm_panel))
}

real matrix vckss__fe_firm_to_worker(
    struct vckss_fe_design scalar design,
    real matrix firm_value)
{
    if (design.external_operator) {
        if (design.operator_firm_to_worker == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_firm_to_worker)(
            design.operator_context,design,firm_value))
    }
    return(vckss__group_sum(
        design.frequency:*firm_value[design.firm,.],
        design.worker_order,design.worker_panel):/design.worker_weight)
}

real matrix vckss__fe_wtranspose_full(
    struct vckss_fe_design scalar design,
    real matrix fitted)
{
    if (design.external_operator) {
        if (design.operator_wtranspose == NULL |
            design.operator_context == NULL) return(J(0,0,.))
        return((*design.operator_wtranspose)(
            design.operator_context,design,fitted))
    }
    return(vckss__fe_transpose_full(design,design.frequency:*fitted))
}

struct vckss_solve_result scalar vckss__fe_solve_b0(
    struct vckss_fe_design scalar design,
    real colvector right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct vckss_solve_result scalar out
    real scalar workers, firms, iteration, denominator, rz, rz_new
    real scalar reduced_scale, full_scale, normalization
    real colvector worker_rhs, firm_rhs, full_firm_rhs, full_rhs
    real colvector worker_base, reduced_rhs
    real colvector firm_coefficient, residual, preconditioned, direction
    real colvector action, worker_coefficient, fitted, full_residual
    real colvector worker_lhs, firm_lhs

    out.status = "INVALID_INPUT"
    out.message = "invalid matrix-free right-hand side"
    out.rhs_status = "INVALID_INPUT"
    out.coefficient = J(0,1,.)
    out.prediction = J(0,1,.)
    out.iterations = .
    out.relres = .
    out.rhs_iterations = .
    out.rhs_relres = .
    out.schur_actions = 0
    out.schur_batches = 0
    out.preconditioner_applications = 0
    out.preconditioner_batches = 0
    out.schur_seconds = 0
    out.preconditioner_seconds = 0
    out.pcg_seconds = 0
    out.workspace_builds = 0
    out.buffered_schur_batches = 0
    out.legacy_schur_batches = 0
    out.buffered_schur_columns = 0
    out.legacy_schur_columns = 0
    out.packed_fallback_batches = 0
    out.max_buffer_width = 0
    out.workspace_peak_bytes = 0
    out.cell_bytes_avoided = 0
    workers = design.worker_levels
    firms = design.firm_levels
    if (design.status != "CONVERGED" |
        !(rows(right_hand_side) == workers+firms-1 |
          rows(right_hand_side) == workers+firms) |
        cols(right_hand_side) != 1 | hasmissing(right_hand_side)) return(out)

    worker_rhs = right_hand_side[1..workers]
    if (rows(right_hand_side) == workers+firms) {
        full_firm_rhs = right_hand_side[(workers+1)..rows(right_hand_side)]
        firm_rhs = full_firm_rhs[1..(firms-1)]
        full_rhs = right_hand_side
    }
    else {
        firm_rhs = right_hand_side[(workers+1)..rows(right_hand_side)]
        full_firm_rhs = firm_rhs \ (sum(worker_rhs)-sum(firm_rhs))
        full_rhs = worker_rhs \ full_firm_rhs
    }
    worker_base = vckss__fe_worker_base(design,worker_rhs)
    reduced_rhs = full_firm_rhs -
        vckss__fe_worker_to_firm(design,worker_base)
    // Solve the singular mobility Laplacian directly on the quotient.  Every
    // operation is equivariant to a permutation of firm labels; grounding is
    // imposed only after the fitted quotient element has been obtained.
    reduced_rhs = reduced_rhs :- mean(reduced_rhs)
    reduced_scale = vckss__norm2(reduced_rhs)
    firm_coefficient = J(firms,1,0)
    if (reduced_scale == 0) {
        iteration = 0
    }
    else {
        residual = reduced_rhs
        preconditioned = residual :/ design.schur_diagonal
        preconditioned = preconditioned :- mean(preconditioned)
        direction = preconditioned
        rz = (residual' * preconditioned)[1,1]
        for (iteration=1; iteration<=maxiter; iteration++) {
            action = vckss__fe_schur_action(design,direction)
            out.schur_actions = out.schur_actions+1
            out.schur_batches = out.schur_batches+1
            denominator = (direction' * action)[1,1]
            if (denominator <= 0 | denominator >= . | rz <= 0 | rz >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "firm mobility PCG lost positive curvature"
                return(out)
            }
            firm_coefficient = firm_coefficient + (rz/denominator) :* direction
            residual = residual - (rz/denominator) :* action
            residual = residual :- mean(residual)
            if (vckss__norm2(residual) <= tolerance*reduced_scale) break
            preconditioned = residual :/ design.schur_diagonal
            out.preconditioner_applications =
                out.preconditioner_applications+1
            out.preconditioner_batches = out.preconditioner_batches+1
            preconditioned = preconditioned :- mean(preconditioned)
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
    normalization = firm_coefficient[firms]
    firm_coefficient = firm_coefficient :- normalization
    worker_coefficient = worker_base -
        vckss__fe_firm_to_worker(design,firm_coefficient)
    out.coefficient = worker_coefficient \ firm_coefficient[1..(firms-1)]
    fitted = vckss__fe_predict(design,out.coefficient)
    worker_lhs = vckss__fe_wtranspose_full(design,fitted)
    firm_lhs = worker_lhs[(workers+1)..rows(worker_lhs)]
    worker_lhs = worker_lhs[1..workers]
    full_residual = (worker_lhs-worker_rhs) \ (firm_lhs-full_firm_rhs)
    full_scale = vckss__norm2(full_rhs)
    if (full_scale == 0) out.relres = vckss__norm2(full_residual)
    else out.relres = vckss__norm2(full_residual) / full_scale
    if (hasmissing(out.coefficient) | hasmissing(out.relres) |
        out.relres > max((1e-11,10*tolerance))) {
        out.status = "SOLVER_RESIDUAL_FAILED"
        out.message = "recomputed full two-way residual exceeds tolerance"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "matrix-free two-way solve converged"
    out.prediction = fitted
    out.iterations = iteration
    out.rhs_status = "CONVERGED"
    out.rhs_iterations = iteration
    out.rhs_relres = out.relres
    return(out)
}

struct vckss_solve_result scalar vckss__fe_solve_matrix_backend(
    struct vckss_fe_design scalar design,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter,
    struct vckss_solver_backend scalar backend)
{
    struct vckss_solve_result scalar out
    struct vckss_preconditioner_result scalar applied
    real scalar workers, firms, columns, column, iteration, active_count
    real scalar denominator, alpha, rz_new, normalization
    real matrix worker_rhs, firm_rhs, full_firm_rhs, full_rhs, worker_base
    real matrix reduced_rhs, firm_coefficient, residual, preconditioned
    real matrix direction, action, replacement_argument, explicit_residual
    real matrix fitted, worker_coefficient, worker_lhs, firm_lhs
    real matrix full_residual
    struct vckss_fe_workspace scalar workspace
    real colvector active_index
    real rowvector reduced_scale, rz, active, restart

    out.status = "INVALID_INPUT"
    out.message = "invalid batched matrix-free right-hand side"
    out.rhs_status = J(1,cols(right_hand_side),"INVALID_INPUT")
    out.coefficient = J(0,0,.)
    out.prediction = J(0,0,.)
    out.iterations = .
    out.relres = .
    out.rhs_iterations = J(1,cols(right_hand_side),.)
    out.rhs_relres = J(1,cols(right_hand_side),.)
    out.schur_actions = 0
    out.schur_batches = 0
    out.preconditioner_applications = 0
    out.preconditioner_batches = 0
    out.schur_seconds = 0
    out.preconditioner_seconds = 0
    out.pcg_seconds = 0
    out.workspace_builds = 0
    out.buffered_schur_batches = 0
    out.legacy_schur_batches = 0
    out.buffered_schur_columns = 0
    out.legacy_schur_columns = 0
    out.packed_fallback_batches = 0
    out.max_buffer_width = 0
    out.workspace_peak_bytes = 0
    out.cell_bytes_avoided = 0
    workers = design.worker_levels
    firms = design.firm_levels
    columns = cols(right_hand_side)
    if (design.status != "CONVERGED" | backend.apply == NULL |
        (design.external_operator & design.operator_schur_into == NULL) |
        columns < 1 |
        !(rows(right_hand_side) == workers+firms-1 |
          rows(right_hand_side) == workers+firms) |
        hasmissing(right_hand_side)) return(out)

    workspace = vckss__fe_workspace_init(design,columns)
    out.workspace_builds = 1
    out.max_buffer_width = columns
    out.workspace_peak_bytes = workspace.modeled_bytes

    timer_clear(96)
    timer_clear(97)
    timer_clear(98)
    timer_on(96)
    worker_rhs = right_hand_side[1..workers,.]
    if (rows(right_hand_side) == workers+firms) {
        full_firm_rhs =
            right_hand_side[(workers+1)..rows(right_hand_side),.]
        firm_rhs = full_firm_rhs[1..(firms-1),.]
        full_rhs = right_hand_side
    }
    else {
        firm_rhs = right_hand_side[(workers+1)..rows(right_hand_side),.]
        full_firm_rhs = firm_rhs \
            (colsum(worker_rhs)-colsum(firm_rhs))
        full_rhs = worker_rhs \
            full_firm_rhs
    }
    worker_base = vckss__fe_worker_base(design,worker_rhs)
    reduced_rhs = full_firm_rhs -
        vckss__fe_worker_to_firm(design,worker_base)
    // Work on the zero-sum quotient throughout.  Matrix operations share one
    // graph traversal across all active right-hand sides; all reductions and
    // convergence decisions remain column-specific.
    reduced_rhs = reduced_rhs -
        J(firms,1,1)*(colsum(reduced_rhs):/firms)
    reduced_scale = J(1,columns,.)
    firm_coefficient = J(firms,columns,0)
    residual = reduced_rhs
    preconditioned = J(firms,columns,0)
    direction = J(firms,columns,0)
    rz = J(1,columns,0)
    active = J(1,columns,0)
    out.rhs_status = J(1,columns,"PENDING")
    out.rhs_iterations = J(1,columns,0)
    for (column=1; column<=columns; column++) {
        reduced_scale[column] = vckss__norm2(reduced_rhs[.,column])
        if (reduced_scale[column] == 0) {
            out.rhs_status[column] = "CONVERGED"
        }
        else active[column] = 1
    }
    active_count = sum(active)
    // A backend may certify that one application is the exact inverse of the
    // reduced quotient operator (for example, a one-level full dense
    // terminal).  Use that action directly as a candidate solution, but never
    // as proof of convergence: coefficient reconstruction and the complete
    // residual against every original full-system RHS below remain mandatory.
    if (active_count > 0 & backend.exact_inverse == 1) {
        active_index = selectindex(active' :== 1)
        timer_on(98)
        if (active_count == columns) {
            applied = (*backend.apply)(backend.context,design,residual)
        }
        else applied = (*backend.apply)(
            backend.context,design,residual[.,active_index])
        timer_off(98)
        if (applied.status != "CONVERGED" |
            rows(applied.value) != firms |
            cols(applied.value) != active_count | hasmissing(applied.value)) {
            out.status = applied.status
            out.message = applied.message
            if (out.status == "CONVERGED") {
                out.status = "INVALID_PRECONDITIONER_ACTION"
                out.message = "exact-terminal backend returned an invalid inverse action"
            }
            timer_off(96)
            out.preconditioner_seconds = vckss__timer_seconds(98)
            out.pcg_seconds = vckss__timer_seconds(96)
            return(out)
        }
        firm_coefficient[.,active_index] = applied.value
        out.preconditioner_applications = active_count
        out.preconditioner_batches = 1
        for (column=1; column<=columns; column++) {
            if (active[column]) {
                out.rhs_status[column] = "CONVERGED"
                out.rhs_iterations[column] = 1
            }
            else firm_coefficient[.,column] = J(firms,1,0)
        }
        active = J(1,columns,0)
        active_count = 0
    }
    if (active_count > 0) {
        active_index = selectindex(active' :== 1)
        timer_on(98)
        if (active_count == columns) {
            applied = (*backend.apply)(backend.context,design,residual)
        }
        else applied = (*backend.apply)(
            backend.context,design,residual[.,active_index])
        timer_off(98)
        if (applied.status != "CONVERGED" |
            rows(applied.value) != firms |
            cols(applied.value) != active_count |
            hasmissing(applied.value)) {
            out.status = applied.status
            out.message = applied.message
            if (out.status == "CONVERGED") {
                out.status = "INVALID_PRECONDITIONER_ACTION"
                out.message = "backend returned an invalid packed action"
            }
            timer_off(96)
            out.preconditioner_seconds = vckss__timer_seconds(98)
            out.pcg_seconds = max((vckss__timer_seconds(96),
                out.schur_seconds+out.preconditioner_seconds))
            return(out)
        }
        preconditioned[.,active_index] = applied.value
        out.preconditioner_applications = active_count
        out.preconditioner_batches = 1
        for (column=1; column<=columns; column++) {
            if (!active[column]) {
                preconditioned[.,column] = J(firms,1,0)
                continue
            }
            direction[.,column] = preconditioned[.,column]
            rz[column] =
                (residual[.,column]'*preconditioned[.,column])[1,1]
            if (rz[column] <= 0 | rz[column] >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "firm mobility PCG residual became nonpositive"
                out.rhs_status[column] = out.status
                timer_off(96)
                out.schur_seconds = vckss__timer_seconds(97)
                out.preconditioner_seconds = vckss__timer_seconds(98)
                out.pcg_seconds = max((vckss__timer_seconds(96),
                    out.schur_seconds+out.preconditioner_seconds))
                return(out)
            }
        }
    }

    for (iteration=1; iteration<=maxiter & active_count>0; iteration++) {
        restart = J(1,columns,0)
        active_index = selectindex(active' :== 1)
        timer_on(97)
        if (active_count == columns) {
            action = J(firms,columns,0)
            vckss__fe_schur_into(design,direction,&workspace,&action)
            out.buffered_schur_batches = out.buffered_schur_batches+1
            out.buffered_schur_columns =
                out.buffered_schur_columns+active_count
            out.cell_bytes_avoided = out.cell_bytes_avoided+
                8*design.n*active_count
        }
        else {
            action = J(firms,columns,0)
            action[.,active_index] = vckss__fe_schur_action(
                design,direction[.,active_index])
            out.legacy_schur_batches = out.legacy_schur_batches+1
            out.legacy_schur_columns =
                out.legacy_schur_columns+active_count
            out.packed_fallback_batches = out.packed_fallback_batches+1
        }
        timer_off(97)
        out.schur_actions = out.schur_actions+active_count
        out.schur_batches = out.schur_batches+1
        for (column=1; column<=columns; column++) {
            if (!active[column]) continue
            denominator =
                (direction[.,column]'*action[.,column])[1,1]
            if (denominator <= 0 | denominator >= . |
                rz[column] <= 0 | rz[column] >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "firm mobility PCG lost positive curvature"
                out.rhs_status[column] = out.status
                timer_off(96)
                out.schur_seconds = vckss__timer_seconds(97)
                out.preconditioner_seconds = vckss__timer_seconds(98)
                out.pcg_seconds = max((vckss__timer_seconds(96),
                    out.schur_seconds+out.preconditioner_seconds))
                return(out)
            }
            alpha = rz[column]/denominator
            firm_coefficient[.,column] =
                firm_coefficient[.,column] + alpha:*direction[.,column]
            residual[.,column] = residual[.,column] - alpha:*action[.,column]
            residual[.,column] = residual[.,column] :-
                mean(residual[.,column])
        }
        // Periodically recompute the true quotient residual.  It governs the
        // convergence decision immediately.  A recurrence restart occurs only
        // when measured drift is material at the registered tolerance; an
        // unconditional restart destroys useful conjugacy on weak graphs.
        if (mod(iteration,100) == 0) {
            if (active_count == columns) {
                replacement_argument = firm_coefficient
            }
            else replacement_argument = firm_coefficient[.,active_index]
            timer_on(97)
            if (active_count == columns) {
                action = J(firms,columns,0)
                vckss__fe_schur_into(design,replacement_argument,
                    &workspace,&action)
                out.buffered_schur_batches =
                    out.buffered_schur_batches+1
                out.buffered_schur_columns =
                    out.buffered_schur_columns+active_count
                out.cell_bytes_avoided = out.cell_bytes_avoided+
                    8*design.n*active_count
            }
            else {
                action = J(firms,columns,0)
                action[.,active_index] = vckss__fe_schur_action(
                    design,replacement_argument)
                out.legacy_schur_batches = out.legacy_schur_batches+1
                out.legacy_schur_columns =
                    out.legacy_schur_columns+active_count
                out.packed_fallback_batches =
                    out.packed_fallback_batches+1
            }
            timer_off(97)
            out.schur_actions = out.schur_actions+active_count
            out.schur_batches = out.schur_batches+1
            explicit_residual = reduced_rhs-action
            for (column=1; column<=columns; column++) {
                if (!active[column]) continue
                explicit_residual[.,column] =
                    explicit_residual[.,column] :-
                    mean(explicit_residual[.,column])
                if (vckss__norm2(explicit_residual[.,column]) <=
                    tolerance*reduced_scale[column] |
                    vckss__norm2(explicit_residual[.,column]-
                        residual[.,column]) >
                    max((1e-14*reduced_scale[column],
                        0.1*tolerance*reduced_scale[column]))) {
                    residual[.,column] = explicit_residual[.,column]
                    restart[column] = 1
                }
            }
        }
        for (column=1; column<=columns; column++) {
            if (!active[column]) continue
            if (vckss__norm2(residual[.,column]) <=
                tolerance*reduced_scale[column]) {
                active[column] = 0
                out.rhs_status[column] = "CONVERGED"
                out.rhs_iterations[column] = iteration
                residual[.,column] = J(firms,1,0)
                direction[.,column] = J(firms,1,0)
            }
        }
        active_count = sum(active)
        if (active_count == 0) break
        active_index = selectindex(active' :== 1)
        timer_on(98)
        if (active_count == columns) {
            applied = (*backend.apply)(backend.context,design,residual)
        }
        else applied = (*backend.apply)(
            backend.context,design,residual[.,active_index])
        timer_off(98)
        if (applied.status != "CONVERGED" |
            rows(applied.value) != firms |
            cols(applied.value) != active_count |
            hasmissing(applied.value)) {
            out.status = applied.status
            out.message = applied.message
            if (out.status == "CONVERGED") {
                out.status = "INVALID_PRECONDITIONER_ACTION"
                out.message = "backend returned an invalid packed action"
            }
            timer_off(96)
            out.schur_seconds = vckss__timer_seconds(97)
            out.preconditioner_seconds = vckss__timer_seconds(98)
            out.pcg_seconds = max((vckss__timer_seconds(96),
                out.schur_seconds+out.preconditioner_seconds))
            return(out)
        }
        preconditioned = J(firms,columns,0)
        preconditioned[.,active_index] = applied.value
        out.preconditioner_applications =
            out.preconditioner_applications+active_count
        out.preconditioner_batches = out.preconditioner_batches+1
        for (column=1; column<=columns; column++) {
            if (!active[column]) {
                preconditioned[.,column] = J(firms,1,0)
                continue
            }
            rz_new =
                (residual[.,column]'*preconditioned[.,column])[1,1]
            if (rz_new <= 0 | rz_new >= .) {
                out.status = "PCG_BREAKDOWN"
                out.message = "firm mobility PCG residual became nonpositive"
                out.rhs_status[column] = out.status
                timer_off(96)
                out.schur_seconds = vckss__timer_seconds(97)
                out.preconditioner_seconds = vckss__timer_seconds(98)
                out.pcg_seconds = max((vckss__timer_seconds(96),
                    out.schur_seconds+out.preconditioner_seconds))
                return(out)
            }
            if (restart[column]) {
                direction[.,column] = preconditioned[.,column]
            }
            else {
                direction[.,column] = preconditioned[.,column] +
                    (rz_new/rz[column]):*direction[.,column]
            }
            rz[column] = rz_new
        }
    }
    if (active_count > 0) {
        out.status = "PCG_NONCONVERGENCE"
        out.message = "firm mobility PCG exceeded maxiter()"
        for (column=1; column<=columns; column++) {
            if (active[column]) {
                out.rhs_status[column] = out.status
                out.rhs_iterations[column] = maxiter
            }
        }
        timer_off(96)
        out.schur_seconds = vckss__timer_seconds(97)
        out.preconditioner_seconds = vckss__timer_seconds(98)
        out.pcg_seconds = max((vckss__timer_seconds(96),
            out.schur_seconds+out.preconditioner_seconds))
        return(out)
    }

    for (column=1; column<=columns; column++) {
        normalization = firm_coefficient[firms,column]
        firm_coefficient[.,column] =
            firm_coefficient[.,column] :- normalization
    }
    worker_coefficient = worker_base -
        vckss__fe_firm_to_worker(design,firm_coefficient)
    out.coefficient = worker_coefficient \
        firm_coefficient[1..(firms-1),.]
    fitted = vckss__fe_predict(design,out.coefficient)
    worker_lhs = vckss__fe_wtranspose_full(design,fitted)
    firm_lhs = worker_lhs[(workers+1)..rows(worker_lhs),.]
    worker_lhs = worker_lhs[1..workers,.]
    full_residual = (worker_lhs-worker_rhs) \
        (firm_lhs-full_firm_rhs)
    out.rhs_relres = vckss__column_relres(full_residual,full_rhs)
    if (cols(out.rhs_relres) != columns | hasmissing(out.coefficient) |
        hasmissing(out.rhs_relres)) {
        out.status = "SOLVER_RESIDUAL_FAILED"
        out.message = "recomputed full two-way residual is nonfinite"
    }
    else {
        for (column=1; column<=columns; column++) {
            if (out.rhs_relres[column] > max((1e-11,10*tolerance))) {
                out.status = "SOLVER_RESIDUAL_FAILED"
                out.message =
                    "recomputed full two-way residual exceeds tolerance"
                out.rhs_status[column] = out.status
                break
            }
        }
    }
    timer_off(96)
    out.schur_seconds = vckss__timer_seconds(97)
    out.preconditioner_seconds = vckss__timer_seconds(98)
    out.pcg_seconds = max((vckss__timer_seconds(96),
        out.schur_seconds+out.preconditioner_seconds))
    if (out.status == "SOLVER_RESIDUAL_FAILED") return(out)
    out.status = "CONVERGED"
    out.message = "lockstep batched two-way solves converged"
    // This prediction is reusable only after the complete residual against
    // the original full right-hand side has passed above.
    out.prediction = fitted
    out.iterations = max(out.rhs_iterations)
    out.relres = max(out.rhs_relres)
    return(out)
}

struct vckss_solve_result scalar vckss__fe_solve_matrix(
    struct vckss_fe_design scalar design,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct vckss_solver_backend scalar backend

    backend = vckss__diagonal_backend()
    return(vckss__fe_solve_matrix_backend(
        design,right_hand_side,tolerance,maxiter,backend))
}

struct vckss_solve_result scalar vckss__fe_solve(
    struct vckss_fe_design scalar design,
    real colvector right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    return(vckss__fe_solve_matrix(
        design,right_hand_side,tolerance,maxiter))
}

struct vckss_solve_result scalar vckss__fe_solve_matrix_b0(
    struct vckss_fe_design scalar design,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct vckss_solve_result scalar out, one
    real scalar column

    out.status = "CONVERGED"
    out.message = "scalar-reference two-way solves converged"
    out.rhs_status = J(1,cols(right_hand_side),"CONVERGED")
    out.coefficient = J(
        design.worker_levels+design.firm_levels-1,
        cols(right_hand_side),.)
    out.prediction = J(design.n,cols(right_hand_side),.)
    out.iterations = 0
    out.relres = 0
    out.rhs_iterations = J(1,cols(right_hand_side),0)
    out.rhs_relres = J(1,cols(right_hand_side),0)
    out.schur_actions = 0
    out.schur_batches = 0
    out.preconditioner_applications = 0
    out.preconditioner_batches = 0
    out.schur_seconds = 0
    out.preconditioner_seconds = 0
    out.pcg_seconds = 0
    out.workspace_builds = 0
    out.buffered_schur_batches = 0
    out.legacy_schur_batches = 0
    out.buffered_schur_columns = 0
    out.legacy_schur_columns = 0
    out.packed_fallback_batches = 0
    out.max_buffer_width = 0
    out.workspace_peak_bytes = 0
    out.cell_bytes_avoided = 0
    for (column=1; column<=cols(right_hand_side); column++) {
        one = vckss__fe_solve_b0(
            design,right_hand_side[.,column],tolerance,maxiter)
        if (one.status != "CONVERGED") return(one)
        out.coefficient[.,column] = one.coefficient
        out.prediction[.,column] = one.prediction
        out.rhs_iterations[column] = one.iterations
        out.rhs_relres[column] = one.relres
        out.iterations = max((out.iterations,one.iterations))
        out.relres = max((out.relres,one.relres))
        out.schur_actions = out.schur_actions+one.schur_actions
        out.schur_batches = out.schur_batches+one.schur_batches
        out.preconditioner_applications =
            out.preconditioner_applications+one.preconditioner_applications
        out.preconditioner_batches =
            out.preconditioner_batches+one.preconditioner_batches
        out.workspace_builds = out.workspace_builds+one.workspace_builds
        out.buffered_schur_batches = out.buffered_schur_batches+
            one.buffered_schur_batches
        out.legacy_schur_batches = out.legacy_schur_batches+
            one.legacy_schur_batches
        out.buffered_schur_columns = out.buffered_schur_columns+
            one.buffered_schur_columns
        out.legacy_schur_columns = out.legacy_schur_columns+
            one.legacy_schur_columns
        out.packed_fallback_batches = out.packed_fallback_batches+
            one.packed_fallback_batches
        out.max_buffer_width = max((out.max_buffer_width,
            one.max_buffer_width))
        out.workspace_peak_bytes = max((out.workspace_peak_bytes,
            one.workspace_peak_bytes))
        out.cell_bytes_avoided = out.cell_bytes_avoided+
            one.cell_bytes_avoided
    }
    return(out)
}

struct vckss_joint_design scalar vckss__joint_prepare(
    struct vckss_fe_design scalar base,
    real matrix controls,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    struct vckss_solver_backend scalar backend)
{
    struct vckss_joint_design scalar out
    struct vckss_solve_result scalar solved
    struct vckss_inverse_result scalar small_inverse
    real matrix weighted_controls, full_cross, schur, checked_schur

    out.status = "INVALID_INPUT"
    out.message = "invalid joint-control design"
    out.base = base
    out.backend = backend
    out.controls = controls
    out.cross = J(base.worker_levels+base.firm_levels-1,0,.)
    out.base_cross_inverse = J(base.worker_levels+base.firm_levels-1,0,.)
    out.residualized_controls = J(base.n,0,.)
    out.schur_inverse = J(0,0,.)
    out.schur_rcond = .
    out.preparation_relres = 0
    out.preparation_iterations = 0
    out.preparation_rhs_iterations = J(1,0,.)
    out.preparation_rhs_relres = J(1,0,.)
    out.preparation_schur_actions = 0
    out.preparation_schur_batches = 0
    out.preparation_precond_applications = 0
    out.preparation_precond_batches = 0
    out.preparation_schur_seconds = 0
    out.preparation_precond_seconds = 0
    out.preparation_pcg_seconds = 0
    out.preparation_fe_profile = J(1,9,0)
    if (base.status != "CONVERGED" | rows(controls) != base.n |
        hasmissing(controls)) return(out)
    if (cols(controls) == 0) {
        out.status = "CONVERGED"
        out.message = "two-way design has no joint controls"
        return(out)
    }
    weighted_controls = base.frequency :* controls
    full_cross = vckss__fe_transpose_full(base,weighted_controls)
    out.cross = full_cross[
        1..(base.worker_levels+base.firm_levels-1),.]
    solved = vckss__fe_solve_matrix_backend(
        base,full_cross,tolerance,maxiter,backend)
    if (solved.status != "CONVERGED") {
        out.status = solved.status
        out.message = solved.message
        return(out)
    }
    out.base_cross_inverse = solved.coefficient
    out.residualized_controls = controls-solved.prediction
    schur = controls' * weighted_controls -
        out.cross' * out.base_cross_inverse
    small_inverse = vckss__inverse(schur,rank_tolerance)
    if (small_inverse.status != "CONVERGED") {
        out.status = "SINGULAR_NUISANCE_BLOCK"
        out.message = "residualized joint-control block is singular"
        return(out)
    }
    checked_schur = out.residualized_controls' *
        (base.frequency :* out.residualized_controls)
    out.preparation_relres = vckss__max_column_relres(
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
    out.preparation_rhs_iterations = solved.rhs_iterations
    out.preparation_rhs_relres = solved.rhs_relres
    out.preparation_schur_actions = solved.schur_actions
    out.preparation_schur_batches = solved.schur_batches
    out.preparation_precond_applications =
        solved.preconditioner_applications
    out.preparation_precond_batches = solved.preconditioner_batches
    out.preparation_schur_seconds = solved.schur_seconds
    out.preparation_precond_seconds = solved.preconditioner_seconds
    out.preparation_pcg_seconds = solved.pcg_seconds
    out.preparation_fe_profile = vckss__fe_profile_row(solved)
    out.status = "CONVERGED"
    out.message = "joint-control Schur complement prepared"
    return(out)
}

real matrix vckss__joint_predict(
    struct vckss_joint_design scalar design,
    real matrix coefficient)
{
    real scalar base_parameters

    base_parameters = design.base.worker_levels + design.base.firm_levels - 1
    if (cols(design.controls) == 0) {
        return(vckss__fe_predict(design.base,coefficient))
    }
    return(vckss__fe_predict(
        design.base,coefficient[1..base_parameters,.]) +
        design.controls * coefficient[(base_parameters+1)..rows(coefficient),.])
}

real matrix vckss__joint_transpose(
    struct vckss_joint_design scalar design,
    real matrix values)
{
    if (cols(design.controls) == 0) {
        return(vckss__fe_transpose(design.base,values))
    }
    return(vckss__fe_transpose(design.base,values) \
        design.controls' * values)
}

real matrix vckss__joint_transpose_full(
    struct vckss_joint_design scalar design,
    real matrix values)
{
    if (cols(design.controls) == 0) {
        return(vckss__fe_transpose_full(design.base,values))
    }
    return(vckss__fe_transpose_full(design.base,values) \
        design.controls' * values)
}

struct vckss_solve_result scalar vckss__joint_solve(
    struct vckss_joint_design scalar design,
    real matrix right_hand_side,
    real scalar tolerance,
    real scalar maxiter)
{
    struct vckss_solve_result scalar out, base_solved
    real scalar base_parameters, full_base_parameters, control_count, column
    real scalar supplied_full_base
    real matrix base_rhs, control_rhs, gamma, base_coefficient
    real matrix fitted, residual

    out.status = "INVALID_INPUT"
    out.message = "invalid joint-system right-hand side"
    out.rhs_status = J(1,cols(right_hand_side),"INVALID_INPUT")
    out.coefficient = J(0,0,.)
    out.prediction = J(0,0,.)
    out.iterations = .
    out.relres = .
    out.rhs_iterations = J(1,cols(right_hand_side),.)
    out.rhs_relres = J(1,cols(right_hand_side),.)
    out.schur_actions = 0
    out.schur_batches = 0
    out.preconditioner_applications = 0
    out.preconditioner_batches = 0
    out.schur_seconds = 0
    out.preconditioner_seconds = 0
    out.pcg_seconds = 0
    out.workspace_builds = 0
    out.buffered_schur_batches = 0
    out.legacy_schur_batches = 0
    out.buffered_schur_columns = 0
    out.legacy_schur_columns = 0
    out.packed_fallback_batches = 0
    out.max_buffer_width = 0
    out.workspace_peak_bytes = 0
    out.cell_bytes_avoided = 0
    if (design.status != "CONVERGED") return(out)
    base_parameters = design.base.worker_levels + design.base.firm_levels - 1
    full_base_parameters = base_parameters+1
    control_count = cols(design.controls)
    supplied_full_base =
        (rows(right_hand_side) == full_base_parameters+control_count)
    if (!(supplied_full_base |
          rows(right_hand_side) == base_parameters+control_count) |
        hasmissing(right_hand_side)) return(out)
    if (supplied_full_base) {
        base_rhs = right_hand_side[1..full_base_parameters,.]
    }
    else base_rhs = right_hand_side[1..base_parameters,.]
    base_solved = vckss__fe_solve_matrix_backend(
        design.base,base_rhs,tolerance,maxiter,design.backend)
    if (base_solved.status != "CONVERGED") return(base_solved)
    if (control_count == 0) return(base_solved)
    out.rhs_status = base_solved.rhs_status
    out.rhs_iterations = base_solved.rhs_iterations
    out.schur_actions = base_solved.schur_actions
    out.schur_batches = base_solved.schur_batches
    out.preconditioner_applications =
        base_solved.preconditioner_applications
    out.preconditioner_batches = base_solved.preconditioner_batches
    out.schur_seconds = base_solved.schur_seconds
    out.preconditioner_seconds = base_solved.preconditioner_seconds
    out.pcg_seconds = base_solved.pcg_seconds
    out.workspace_builds = base_solved.workspace_builds
    out.buffered_schur_batches = base_solved.buffered_schur_batches
    out.legacy_schur_batches = base_solved.legacy_schur_batches
    out.buffered_schur_columns = base_solved.buffered_schur_columns
    out.legacy_schur_columns = base_solved.legacy_schur_columns
    out.packed_fallback_batches = base_solved.packed_fallback_batches
    out.max_buffer_width = base_solved.max_buffer_width
    out.workspace_peak_bytes = base_solved.workspace_peak_bytes
    out.cell_bytes_avoided = base_solved.cell_bytes_avoided
    if (supplied_full_base) {
        control_rhs = right_hand_side[
            (full_base_parameters+1)..rows(right_hand_side),.]
    }
    else control_rhs = right_hand_side[
        (base_parameters+1)..rows(right_hand_side),.]
    gamma = design.schur_inverse *
        (control_rhs - design.cross' * base_solved.coefficient)
    base_coefficient = base_solved.coefficient -
        design.base_cross_inverse * gamma
    out.coefficient = base_coefficient \ gamma
    fitted = vckss__joint_predict(design,out.coefficient)
    if (supplied_full_base) {
        residual = vckss__joint_transpose_full(
            design,design.base.frequency :* fitted) - right_hand_side
    }
    else residual = vckss__joint_transpose(
        design,design.base.frequency :* fitted) - right_hand_side
    out.rhs_relres = vckss__column_relres(residual,right_hand_side)
    if (cols(out.rhs_relres) == cols(right_hand_side)) {
        out.relres = max(out.rhs_relres)
    }
    else out.relres = .
    if (hasmissing(out.coefficient) | hasmissing(out.relres) |
        out.relres > max((1e-11,10*tolerance))) {
        out.status = "SOLVER_RESIDUAL_FAILED"
        out.message = "recomputed full joint-system residual exceeds tolerance"
        for (column=1; column<=cols(right_hand_side); column++) {
            if (missing(out.rhs_relres[column]) |
                out.rhs_relres[column] > max((1e-11,10*tolerance))) {
                out.rhs_status[column] = out.status
            }
        }
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "matrix-free joint-system solve converged"
    // As in the FE solve, retain the batch prediction only after its complete
    // original joint-system residual has passed.
    out.prediction = fitted
    out.iterations = base_solved.iterations
    return(out)
}

real matrix vckss__physical_panels(real colvector frequency)
{
    real matrix panel
    real colvector finish

    panel = J(rows(frequency),2,.)
    if (rows(frequency) == 0) return(panel)
    finish = runningsum(frequency)
    panel[.,2] = finish
    panel[.,1] = finish:-frequency:+1
    return(panel)
}

real matrix vckss__exact_key_panel(real matrix sorted_key)
{
    real scalar group, groups, row
    real matrix out

    if (rows(sorted_key) < 1 | cols(sorted_key) < 1 |
        hasmissing(sorted_key)) return(J(0,2,.))
    groups = 1
    for (row=2; row<=rows(sorted_key); row++) {
        groups = groups+any(sorted_key[row,.] :!= sorted_key[row-1,.])
    }
    out = J(groups,2,.)
    group = 1
    out[1,1] = 1
    for (row=2; row<=rows(sorted_key); row++) {
        if (any(sorted_key[row,.] :!= sorted_key[row-1,.])) {
            out[group,2] = row-1
            group++
            out[group,1] = row
        }
    }
    out[group,2] = rows(sorted_key)
    return(out)
}

real colvector vckss__semantic_group_ranks(
    real colvector semantic_rank,
    real colvector row_order,
    real matrix panel)
{
    real scalar group
    real colvector group_rank

    if (rows(semantic_rank) != rows(row_order) |
        cols(semantic_rank) != 1 | hasmissing(semantic_rank) |
        min(semantic_rank) < 1 | any(semantic_rank :!= floor(semantic_rank)) |
        max(semantic_rank) > vckss_rng__maximum_exact_integer() |
        rows(panel) < 1) return(J(0,1,.))
    group_rank = J(rows(panel),1,.)
    for (group=1; group<=rows(panel); group++) {
        group_rank[group] = min(semantic_rank[row_order[|
            panel[group,1] \ panel[group,2]|]])
    }
    if (rows(vckss_rng__canonical_order(group_rank)) != rows(group_rank)) {
        return(J(0,1,.))
    }
    return(group_rank)
}

real matrix vckss__draw_semantic_atoms(
    real colvector semantic_rank,
    real colvector trials,
    real scalar probe_count)
{
    real scalar probe
    real colvector canonical_order
    real matrix generated, out

    if (rows(semantic_rank) < 1 | rows(semantic_rank) != rows(trials) |
        !vckss_rng__trials_ok(trials) |
        missing(probe_count) | probe_count < 1 |
        probe_count != floor(probe_count)) return(J(0,0,.))
    canonical_order = vckss_rng__canonical_order(semantic_rank)
    if (rows(canonical_order) != rows(semantic_rank)) return(J(0,0,.))
    out = J(rows(trials),probe_count,.)
    for (probe=1; probe<=probe_count; probe++) {
        generated = vckss_rng__draw_probe_registered(
            trials[canonical_order])
        if (rows(generated) != rows(trials) | cols(generated) != 2 |
            hasmissing(generated)) return(J(0,0,.))
        out[canonical_order,probe] = generated[.,1]
    }
    return(out)
}

real colvector vckss__physical_rademacher_sum(real colvector frequency)
{
    real colvector out, physical_random
    real matrix physical_panel

    physical_panel = vckss__physical_panels(frequency)
    physical_random = 2:*rbinomial(sum(frequency),1,1,0.5):-1
    out = panelsum(physical_random,physical_panel)
    return(out)
}

real colvector vckss__rademacher_sum_prepared(
    real scalar physical_count,
    real matrix physical_panel)
{
    real colvector physical_random

    physical_random = 2:*rbinomial(physical_count,1,1,0.5):-1
    return(panelsum(physical_random,physical_panel))
}

real colvector vckss__target_direction(
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

real rowvector vckss__effect_plugin(
    real colvector coefficient,
    struct vckss_fe_design scalar base,
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

real rowvector vckss__mcse(real matrix draws)
{
    real rowvector mean_draw

    if (rows(draws) < 2) return(J(1,cols(draws),.))
    mean_draw = colsum(draws) :/ rows(draws)
    return(sqrt(colsum((draws :- mean_draw):^2) :/
        (rows(draws)-1) :/ rows(draws)))
}

real scalar vckss__union_find_root(
    real colvector parent,
    real scalar node)
{
    while (parent[node] != node) node = parent[node]
    return(node)
}

struct vckss_component_result
{
    real colvector keep
    real scalar components
    real scalar edges
    real scalar firms
    real scalar physical_mass
    real scalar ambiguous
}

struct vckss_articulation_result
{
    real colvector bad_worker
    real scalar count
}

struct vckss_component_result scalar vckss__largest_component(
    real colvector worker,
    real colvector firm,
    real colvector frequency,
    real colvector active)
{
    struct vckss_component_result scalar out
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
        root_node = vckss__union_find_root(parent,node)
        root_other = vckss__union_find_root(parent,other)
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
        root_node = vckss__union_find_root(parent,node)
        firm_count[root_node] = firm_count[root_node]+1
    }
    for (row=1; row<=rows(selected); row++) {
        node = selected[row]
        root_node = vckss__union_find_root(parent,worker[node])
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
        if (vckss__union_find_root(parent,worker[node]) == best_root) {
            out.keep[node] = 1
        }
    }
    out.firms = best_firms
    out.physical_mass = best_mass
    return(out)
}

real colvector vckss__worker_firm_counts(
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

struct vckss_articulation_result scalar vckss__worker_articulations(
    real colvector worker,
    real colvector firm,
    real colvector active)
{
    struct vckss_articulation_result scalar out
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

void vckss__stata_prune_graph(
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
    struct vckss_component_result scalar component
    struct vckss_articulation_result scalar articulation
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
    if (min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0) return
    if (missing(vckss__exact_physical_total(frequency))) {
        st_local(status_local,"PHYSICAL_TOTAL_LIMIT")
        st_local(message_local,
            "literal frequency total exceeds the exact binary64 integer range")
        return
    }
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
    component = vckss__largest_component(worker,firm,frequency,active)
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
        worker_firms = vckss__worker_firm_counts(worker,firm,active)
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

    component = vckss__largest_component(worker,firm,frequency,active)
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
        component = vckss__largest_component(worker,firm,frequency,active)
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
            worker_firms = vckss__worker_firm_counts(worker,firm,active)
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

        articulation = vckss__worker_articulations(worker,firm,active)
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
    component = vckss__largest_component(worker,firm,frequency,active)
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
    graph_seconds = vckss__timer_seconds(95)
    diagnostics = (n,retained_rows,sum(frequency),retained_mass,
        graph_edges,articulation_removed,maximum_components,
        mover_input_rows,initial_component_rows,insufficient_removed,
        iterations,graph_seconds)
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,"CONVERGED")
    st_local(message_local,
        "MATLAB-compatible leave-one-worker-connected component selected")
}

struct vckss_rank_certificate
{
    string scalar status
    string scalar message
    real scalar gap
    real scalar max_loss
}

struct vckss_rank_certificate scalar vckss__joint_rank_certificate(
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector deletion_id,
    string scalar deletion,
    real scalar rank_tolerance,
    | real colvector stayer_mask)
{
    struct vckss_rank_certificate scalar out
    struct vckss_inverse_result scalar within_inverse, deleted_inverse
    real scalar n, control_count, row, cell, group, groups, begin, finish
    real scalar remaining_frequency, loss, threshold, whitening_error
    real scalar minimum_deleted_eigen, hybrid
    real colvector cell_order, cell_sorted_code, cell_of_row
    real colvector cell_frequency, deletion_order, block_frequency, block_cell
    real colvector index, eigen, mover_index
    real matrix cell_panel, deletion_panel, weighted_controls, cell_sum
    real matrix cell_mean, centered, within, whitener, transformed
    real matrix weighted_transformed
    real matrix checked_within, block_sum, mean_gap, block_cross
    real matrix deleted_scatter, scatter_loss, deleted_within

    out.status = "UNVERIFIED_DELETION_RANK"
    out.message = "joint-control deletion rank lacks a deterministic certificate"
    out.gap = .
    out.max_loss = .
    hybrid = (args() >= 8)
    n = rows(worker)
    control_count = cols(controls)
    if (n == 0 | rows(firm) != n | rows(controls) != n |
        rows(frequency) != n | rows(deletion_id) != n |
        control_count == 0 | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(deletion_id) | min(frequency) <= 0) return(out)
    if (deletion != "observation" & deletion != "match") return(out)
    if (hybrid) {
        if (deletion != "match" | rows(stayer_mask) != n |
            cols(stayer_mask) != 1 | hasmissing(stayer_mask) |
            any((stayer_mask :!= 0) :& (stayer_mask :!= 1)) |
            sum(stayer_mask) == 0 | sum(stayer_mask) == n) return(out)
    }

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
    within_inverse = vckss__inverse(within,rank_tolerance)
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
    whitening_error = vckss__norm2(checked_within-I(control_count))
    threshold = max((1e-10,1000*rank_tolerance))
    if (hasmissing(checked_within) | hasmissing(whitening_error) |
        whitening_error > threshold) {
        out.message = "within-cell control whitening failed its residual gate"
        return(out)
    }

    out.max_loss = 0
    minimum_deleted_eigen = .
    if (deletion == "match") {
        if (hybrid) {
            mover_index = selectindex(stayer_mask :== 0)
            deletion_order = mover_index[order(deletion_id[mover_index],1)]
        }
        else deletion_order = order(deletion_id,1)
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
            deleted_inverse = vckss__inverse(deleted_within,rank_tolerance)
            if (deleted_inverse.status != "CONVERGED") {
                out.message = "match deletion does not retain a directly factorable control scatter"
                return(out)
            }
        }
    }
    if (deletion == "observation" | hybrid) {
        for (row=1; row<=n; row++) {
            if (hybrid) {
                if (!stayer_mask[row]) continue
            }
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
            deleted_inverse = vckss__inverse(deleted_within,rank_tolerance)
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

struct vckss_result scalar vckss__jla_backend(
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
    real scalar blocksize_limit,
    struct vckss_fe_design scalar base,
    struct vckss_solver_backend scalar backend,
    real scalar setup_seconds,
    | real colvector semantic_rank,
    real scalar semantic_atom_mode,
    real colvector stayer_mask)
{
    struct vckss_result scalar out
    struct vckss_joint_design scalar full_joint, working_joint
    struct vckss_solve_result scalar solved, projection_solved, target_solved
    struct vckss_maker_result scalar reduced_maker
    struct vckss_rank_certificate scalar rank_certificate
    struct vckss_control_basis_result scalar canonical_controls
    real scalar n, controls_count, base_parameters, full_parameters
    real scalar probe, group, groups, begin, finish, eigmax, maximum_leverage
    real scalar batch_start, batch_finish, batch_columns, batch_column
    real scalar solver_iterations, solver_residual, target_mass
    real scalar deletion_rank_gap, physical_count, row, control_basis_relres
    real scalar control_downstream_bound
    real scalar solver_schur_actions, solver_schur_batches
    real scalar solver_precond_applications
    real scalar solver_precond_batches
    real scalar solver_schur_seconds, solver_precond_seconds
    real scalar solver_pcg_seconds, use_semantic_atoms, hybrid
    real scalar mover_physical_count, stayer_physical_count
    real matrix deletion_panel, physical_panel, sorted_delete, rhs
    real matrix mover_physical_panel, stayer_physical_panel
    real matrix target_semantic_panel, target_semantic_key
    real matrix target_rhs, target_draws, physical_random_batch
    real matrix stayer_physical_random_batch
    real matrix rademacher_batch, projected_batch, target_direction_batch
    real matrix semantic_atom_batch
    real matrix deletion_projected_batch, deletion_random_batch
    real matrix target_prediction_batch, worker_projection_batch
    real matrix firm_projection_batch, total_projection_batch
    real matrix group_first_batch, group_second_batch
    real matrix block_control, control_factor, low_rank, maker_rhs
    real matrix solver_rhs_diagnostics
    real rowvector fe_profile
    real colvector row_order, index, working_y, coefficient, fitted, residual
    real colvector mover_index, stayer_index, stayer_physical_row
    real colvector unit_representative, target_semantic_order
    real colvector target_representative, target_semantic_trials
    real colvector rademacher_sum, projected, physical_row, physical_random
    real colvector physical_projected, projection_square_sum
    real colvector projection_fourth_sum, copy_first_correlation
    real colvector copy_third_correlation, copy_control_leverage
    real colvector copy_inverse_weight
    real colvector p_first, m_first, p_second
    real colvector m_second, mixed_second, p_mean, m_mean, denominator
    real colvector p_constrained, m_constrained, finite_variance, finite_bias
    real colvector stayer_p_first, stayer_m_first, stayer_p_second
    real colvector stayer_m_second, stayer_mixed_second
    real colvector stayer_p_mean, stayer_m_mean, stayer_denominator
    real colvector stayer_p_constrained, stayer_m_constrained
    real colvector stayer_finite_variance, stayer_finite_bias
    real colvector control_leverage, total_residual_leverage, inverse_weight
    real colvector deletion_frequency, deletion_projected, deletion_random
    real colvector transformed_residual, deleted_adjusted, block_frequency
    real colvector common_direction, inverse_common
    real colvector sqrt_frequency, target_share, target_sqrt_share
    real colvector weighted_y, weighted_deleted
    real matrix worker_target, firm_target
    real colvector worker_projection, firm_projection
    real colvector total_projection, correction_weight, group_first, group_second
    real colvector gamma
    real colvector unit_semantic_rank, target_semantic_atom_rank
    real rowvector plugin, correction, corrected, numerical_mcse
    real rowvector target_reference_scale

    out = vckss__empty_result()
    deletion_rank_gap = .
    use_semantic_atoms = 0
    hybrid = (args() >= 23)
    if (args() >= 22) {
        if (!(semantic_atom_mode == 0 | semantic_atom_mode == 1)) {
            return(vckss__failure(
                "RNG_SEMANTIC_MODE_INVALID",
                "semantic atom mode must be zero or one"))
        }
        use_semantic_atoms = semantic_atom_mode
    }
    n = rows(y)
    if (n == 0 | cols(y) != 1 | rows(worker) != n | rows(firm) != n |
        rows(controls) != n | rows(frequency) != n |
        rows(target_weight) != n | rows(deletion_id) != n |
        hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(controls) | hasmissing(frequency) |
        hasmissing(target_weight) | hasmissing(deletion_id)) {
        return(vckss__failure("INVALID_INPUT", "matrix-free KSS inputs are invalid"))
    }
    if (min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0) {
        return(vckss__failure("INVALID_FREQUENCY", "frequency weights must be positive integers"))
    }
    if (missing(vckss__exact_physical_total(frequency))) {
        return(vckss__failure("PHYSICAL_TOTAL_LIMIT", "literal frequency total exceeds the exact binary64 integer range"))
    }
    if (min(target_weight) < 0 | sum(target_weight) <= 0) {
        return(vckss__failure("INVALID_TARGET_WEIGHT", "target weights must have nonnegative positive mass"))
    }
    if (deletion != "observation" & deletion != "match") {
        return(vckss__failure("UNSUPPORTED_DELETION", "deletion must be observation or match"))
    }
    if (hybrid) {
        if (deletion != "match" | rows(stayer_mask) != n |
            cols(stayer_mask) != 1 | hasmissing(stayer_mask) |
            any((stayer_mask :!= 0) :& (stayer_mask :!= 1)) |
            sum(stayer_mask) == 0 | sum(stayer_mask) == n) {
            return(vckss__failure("INVALID_STAYER_PARTITION",
                "the mixed mover/stayer deletion partition is invalid"))
        }
    }
    if (nuisance != "joint" & nuisance != "fixedoffset") {
        return(vckss__failure("INVALID_NUISANCE", "nuisance must be joint or fixedoffset"))
    }
    if (probes < 2 | probes != floor(probes) | batch < 1 |
        maxiter < 1 | tolerance <= 0 | tolerance >= 1) {
        return(vckss__failure("INVALID_TUNING", "invalid JLA or solver tuning parameter"))
    }
    if (base.status != "CONVERGED" | base.n != n |
        rows(base.worker) != n | rows(base.firm) != n |
        rows(base.frequency) != n | backend.apply == NULL |
        missing(setup_seconds) | setup_seconds < 0) {
        return(vckss__failure("INVALID_SOLVER_BACKEND", "prepared JLA solver backend is invalid"))
    }

    timer_clear(91)
    timer_clear(92)
    timer_clear(93)
    timer_on(91)

    solver_schur_actions = 0
    solver_schur_batches = 0
    solver_precond_applications = 0
    solver_precond_batches = 0
    solver_schur_seconds = 0
    solver_precond_seconds = 0
    solver_pcg_seconds = 0
    solver_rhs_diagnostics = J(0,6,.)
    fe_profile = J(1,9,0)

    base_parameters = base.worker_levels + base.firm_levels - 1
    controls_count = cols(controls)
    full_parameters = base_parameters + controls_count
    control_basis_relres = 0
    if (controls_count > 0) {
        canonical_controls = vckss__canonical_controls(
            controls,frequency,rank_tolerance)
        if (canonical_controls.status != "CONVERGED") {
            return(vckss__failure(
                canonical_controls.status,canonical_controls.message))
        }
        controls = canonical_controls.controls
        control_basis_relres = canonical_controls.relres
    }
    if (deletion == "match") {
        if (hybrid) {
            mover_index = selectindex(stayer_mask :== 0)
            stayer_index = selectindex(stayer_mask :== 1)
            row_order = mover_index[order(deletion_id[mover_index],1)]
        }
        else row_order = order(deletion_id,1)
        sorted_delete = deletion_id[row_order]
        deletion_panel = panelsetup(sorted_delete,1)
        groups = rows(deletion_panel)
        for (group=1; group<=groups; group++) {
            begin = deletion_panel[group,1]
            finish = deletion_panel[group,2]
            index = row_order[|begin \ finish|]
            if (rows(index) > blocksize_limit) {
                return(vckss__failure("BLOCK_SIZE_LIMIT", "a match block exceeds blocksize_limit()"))
            }
            if (min(worker[index]) != max(worker[index]) |
                min(firm[index]) != max(firm[index])) {
                return(vckss__failure("CROSS_COORDINATE_MATCH", "each deletion ID must remain within one worker-firm coordinate"))
            }
        }
    }
    else {
        groups = sum(frequency)
        row_order = J(0,1,.)
        deletion_panel = J(0,2,.)
    }

    if (use_semantic_atoms) {
        if (hybrid | deletion != "match" | controls_count != 0 |
            args() < 22 | cols(semantic_rank) != 1 |
            rows(semantic_rank) != n | hasmissing(semantic_rank) |
            min(semantic_rank) < 1 |
            any(semantic_rank :!= floor(semantic_rank))) {
            return(vckss__failure(
                "RNG_SEMANTIC_MODE_INVALID",
                "compressed semantic atoms require eligible no-control match inputs"))
        }
        unit_representative = row_order[deletion_panel[.,1]]
        deletion_frequency = panelsum(frequency[row_order],deletion_panel)
        unit_semantic_rank = vckss__semantic_group_ranks(
            semantic_rank,row_order,deletion_panel)

        target_semantic_order = order((worker,firm,
            target_weight:/frequency,semantic_rank),(1,2,3,4))
        target_semantic_key = (worker,firm,target_weight:/frequency)[
            target_semantic_order,.]
        target_semantic_panel = vckss__exact_key_panel(target_semantic_key)
        target_representative =
            target_semantic_order[target_semantic_panel[.,1]]
        target_semantic_trials = panelsum(
            frequency[target_semantic_order],target_semantic_panel)
        target_semantic_atom_rank = vckss__semantic_group_ranks(
            semantic_rank,target_semantic_order,target_semantic_panel)
        if (rows(unit_semantic_rank) != groups |
            rows(target_semantic_atom_rank) != rows(target_semantic_panel) |
            rows(vckss_rng__canonical_order(unit_semantic_rank)) != groups |
            rows(vckss_rng__canonical_order(target_semantic_atom_rank)) !=
                rows(target_semantic_panel) |
            !vckss_rng__trials_ok(deletion_frequency) |
            !vckss_rng__trials_ok(target_semantic_trials)) {
            return(vckss__failure(
                "RNG_SEMANTIC_KEY_INVALID",
                "canonical deletion-unit or target-stratum atoms are invalid"))
        }
    }

    full_joint = vckss__joint_prepare(
        base,controls,tolerance,maxiter,rank_tolerance,backend)
    if (full_joint.status != "CONVERGED") {
        return(vckss__failure(full_joint.status,full_joint.message))
    }
    solver_schur_actions = solver_schur_actions+
        full_joint.preparation_schur_actions
    solver_schur_batches = solver_schur_batches+
        full_joint.preparation_schur_batches
    solver_precond_applications = solver_precond_applications+
        full_joint.preparation_precond_applications
    solver_precond_batches = solver_precond_batches+
        full_joint.preparation_precond_batches
    solver_schur_seconds = solver_schur_seconds+
        full_joint.preparation_schur_seconds
    solver_precond_seconds = solver_precond_seconds+
        full_joint.preparation_precond_seconds
    solver_pcg_seconds = solver_pcg_seconds+
        full_joint.preparation_pcg_seconds
    fe_profile = vckss__fe_profile_merge(
        fe_profile,full_joint.preparation_fe_profile)
    solver_rhs_diagnostics = solver_rhs_diagnostics \
        vckss__solver_trace_rows(1,0,
            full_joint.preparation_rhs_iterations,
            full_joint.preparation_rhs_relres)
    if (controls_count > 0) {
        if (full_joint.schur_rcond <= canonical_controls.forward_error) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "canonical-control error exhausts the joint-control conditioning margin"))
        }
        control_downstream_bound = vckss__propagate_error(
            canonical_controls.forward_error,full_joint.schur_rcond)
        if (hasmissing(control_downstream_bound) |
            control_downstream_bound > vckss__control_forward_limit()) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "joint-control conditioning cannot certify basis invariance at the registered tolerance"))
        }
    }
    if (controls_count > 0) {
        if (hybrid) rank_certificate = vckss__joint_rank_certificate(
            worker,firm,controls,frequency,deletion_id,deletion,
            rank_tolerance,stayer_mask)
        else rank_certificate = vckss__joint_rank_certificate(
            worker,firm,controls,frequency,deletion_id,deletion,
            rank_tolerance)
        if (rank_certificate.status != "CONVERGED") {
            return(vckss__failure(
                rank_certificate.status,rank_certificate.message))
        }
        deletion_rank_gap = rank_certificate.gap
        control_downstream_bound = vckss__propagate_error(
            canonical_controls.forward_error,deletion_rank_gap)
        if (hasmissing(control_downstream_bound) |
            control_downstream_bound > vckss__control_forward_limit()) {
            return(vckss__failure("AMBIGUOUS_CONTROL_BASIS", "deletion-rank conditioning cannot certify control-basis invariance"))
        }
    }
    rhs = vckss__joint_transpose_full(full_joint,frequency:*y)
    solved = vckss__joint_solve(full_joint,rhs,tolerance,maxiter)
    if (solved.status != "CONVERGED") {
        return(vckss__failure(solved.status,solved.message))
    }
    solver_schur_actions = solver_schur_actions+solved.schur_actions
    solver_schur_batches = solver_schur_batches+solved.schur_batches
    solver_precond_applications =
        solver_precond_applications+
        solved.preconditioner_applications
    solver_precond_batches = solver_precond_batches+
        solved.preconditioner_batches
    solver_schur_seconds = solver_schur_seconds+solved.schur_seconds
    solver_precond_seconds = solver_precond_seconds+
        solved.preconditioner_seconds
    solver_pcg_seconds = solver_pcg_seconds+solved.pcg_seconds
    fe_profile = vckss__fe_profile_merge(
        fe_profile,vckss__fe_profile_row(solved))
    solver_rhs_diagnostics = solver_rhs_diagnostics \
        vckss__solver_trace_rows(2,0,
            solved.rhs_iterations,solved.rhs_relres)
    solver_iterations = max((full_joint.preparation_iterations,solved.iterations))
    solver_residual = max((control_basis_relres,
        full_joint.preparation_relres,solved.relres))

    if (nuisance == "fixedoffset" & controls_count > 0) {
        gamma = solved.coefficient[(base_parameters+1)..full_parameters]
        working_y = y - controls*gamma
        working_joint = vckss__joint_prepare(
            base,J(n,0,.),tolerance,maxiter,rank_tolerance,backend)
        rhs = vckss__joint_transpose_full(
            working_joint,frequency:*working_y)
        solved = vckss__joint_solve(working_joint,rhs,tolerance,maxiter)
        if (solved.status != "CONVERGED") {
            return(vckss__failure(solved.status,solved.message))
        }
        solver_schur_actions = solver_schur_actions+solved.schur_actions
        solver_schur_batches = solver_schur_batches+solved.schur_batches
        solver_precond_applications =
            solver_precond_applications+
            solved.preconditioner_applications
        solver_precond_batches = solver_precond_batches+
            solved.preconditioner_batches
        solver_schur_seconds = solver_schur_seconds+solved.schur_seconds
        solver_precond_seconds = solver_precond_seconds+
            solved.preconditioner_seconds
        solver_pcg_seconds = solver_pcg_seconds+solved.pcg_seconds
        fe_profile = vckss__fe_profile_merge(
            fe_profile,vckss__fe_profile_row(solved))
        solver_rhs_diagnostics = solver_rhs_diagnostics \
            vckss__solver_trace_rows(3,0,
                solved.rhs_iterations,solved.rhs_relres)
        solver_iterations = max((solver_iterations,solved.iterations))
        solver_residual = max((solver_residual,solved.relres))
    }
    else {
        working_y = y
        working_joint = full_joint
    }
    coefficient = solved.coefficient
    fitted = solved.prediction
    residual = working_y - fitted
    plugin = vckss__effect_plugin(
        coefficient[1..base_parameters],base,target_weight)
    timer_off(91)
    timer_on(92)

    if (vckss_rng__production_contract() == "") {
        return(vckss__failure(
            "RNG_RUNTIME_UNREGISTERED",
            "the current Stata runtime has no registered KSS probe contract"))
    }
    if (vckss_rng__set_stream_seed(
            vckss_rng__domain_stream("leverage"),seed)) {
        return(vckss__failure(
            "RNG_SETUP_FAILED",
            "the leverage-domain mt64s stream could not be initialized"))
    }
    physical_count = sum(frequency)
    physical_panel = vckss__physical_panels(frequency)
    sqrt_frequency = sqrt(frequency)
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
        if (hybrid) {
            mover_physical_count = sum(frequency[mover_index])
            stayer_physical_count = sum(frequency[stayer_index])
            mover_physical_panel = vckss__physical_panels(
                frequency[mover_index])
            stayer_physical_panel = vckss__physical_panels(
                frequency[stayer_index])
            stayer_physical_row = J(stayer_physical_count,1,.)
            for (row=1; row<=rows(stayer_index); row++) {
                stayer_physical_row[|stayer_physical_panel[row,1] \
                    stayer_physical_panel[row,2]|] =
                    J(frequency[stayer_index[row]],1,stayer_index[row])
            }
            projection_square_sum = J(n,1,0)
            projection_fourth_sum = J(n,1,0)
            copy_first_correlation = J(stayer_physical_count,1,0)
            copy_third_correlation = J(stayer_physical_count,1,0)
        }
    }

    for (batch_start=1; batch_start<=probes; batch_start=batch_start+batch) {
        batch_finish = min((probes,batch_start+batch-1))
        batch_columns = batch_finish-batch_start+1
        if (use_semantic_atoms) {
            rademacher_batch = J(n,batch_columns,0)
            semantic_atom_batch = vckss__draw_semantic_atoms(
                unit_semantic_rank,deletion_frequency,batch_columns)
            if (rows(semantic_atom_batch) != groups |
                cols(semantic_atom_batch) != batch_columns |
                hasmissing(semantic_atom_batch)) {
                return(vckss__failure(
                    "RNG_SEMANTIC_DRAW_FAILED",
                    "registered deletion-unit atoms could not be generated"))
            }
            rademacher_batch[unit_representative,.] = semantic_atom_batch
        }
        else if (hybrid) rademacher_batch = J(n,batch_columns,0)
        else rademacher_batch = J(n,batch_columns,.)
        if (deletion == "observation") {
            physical_random_batch = J(physical_count,batch_columns,.)
        }
        else if (hybrid) {
            stayer_physical_random_batch =
                J(stayer_physical_count,batch_columns,.)
        }
        for (batch_column=1; batch_column<=batch_columns &
            !use_semantic_atoms; batch_column++) {
            if (deletion == "observation") {
                physical_random_batch[.,batch_column] =
                    2:*rbinomial(physical_count,1,1,0.5):-1
            }
            else if (hybrid) {
                rademacher_batch[mover_index,batch_column] =
                    vckss__rademacher_sum_prepared(
                        mover_physical_count,mover_physical_panel)
                stayer_physical_random_batch[.,batch_column] =
                    2:*rbinomial(stayer_physical_count,1,1,0.5):-1
                rademacher_batch[stayer_index,batch_column] =
                    panelsum(stayer_physical_random_batch[.,batch_column],
                        stayer_physical_panel)
            }
            else rademacher_batch[.,batch_column] =
                    vckss__rademacher_sum_prepared(
                        physical_count,physical_panel)
        }
        if (deletion == "observation") {
            rademacher_batch = panelsum(
                physical_random_batch,physical_panel)
        }
        rhs = vckss__fe_transpose_full(base,rademacher_batch)
        projection_solved = vckss__fe_solve_matrix_backend(
            base,rhs,tolerance,maxiter,backend)
        if (projection_solved.status != "CONVERGED") {
            return(vckss__failure(
                projection_solved.status,projection_solved.message))
        }
        solver_iterations = max((solver_iterations,projection_solved.iterations))
        solver_residual = max((solver_residual,projection_solved.relres))
        solver_schur_actions = solver_schur_actions+
            projection_solved.schur_actions
        solver_schur_batches = solver_schur_batches+
            projection_solved.schur_batches
        solver_precond_applications =
            solver_precond_applications+
            projection_solved.preconditioner_applications
        solver_precond_batches = solver_precond_batches+
            projection_solved.preconditioner_batches
        solver_schur_seconds = solver_schur_seconds+
            projection_solved.schur_seconds
        solver_precond_seconds = solver_precond_seconds+
            projection_solved.preconditioner_seconds
        solver_pcg_seconds = solver_pcg_seconds+
            projection_solved.pcg_seconds
        fe_profile = vckss__fe_profile_merge(
            fe_profile,vckss__fe_profile_row(projection_solved))
        solver_rhs_diagnostics = solver_rhs_diagnostics \
            vckss__solver_trace_rows(4,batch_start,
                projection_solved.rhs_iterations,
                projection_solved.rhs_relres)
        projected_batch = projection_solved.prediction
        if (deletion == "match") {
            deletion_projected_batch = panelsum(
                (frequency:*projected_batch)[row_order,.],deletion_panel) :/
                deletion_frequency
            deletion_projected_batch =
                sqrt(deletion_frequency):*deletion_projected_batch
            deletion_random_batch = panelsum(
                rademacher_batch[row_order,.],deletion_panel) :/
                sqrt(deletion_frequency)
            deletion_random_batch =
                deletion_random_batch-deletion_projected_batch
        }
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
                deletion_projected =
                    deletion_projected_batch[.,batch_column]
                deletion_random = deletion_random_batch[.,batch_column]
                p_first = p_first + deletion_projected:^2
                m_first = m_first + deletion_random:^2
                p_second = p_second + deletion_projected:^4
                m_second = m_second + deletion_random:^4
                mixed_second = mixed_second +
                    deletion_projected:^2 :* deletion_random:^2
                if (hybrid) {
                    physical_random =
                        stayer_physical_random_batch[.,batch_column]
                    physical_projected = projected[stayer_physical_row]
                    projection_square_sum[stayer_index] =
                        projection_square_sum[stayer_index] +
                        projected[stayer_index]:^2
                    projection_fourth_sum[stayer_index] =
                        projection_fourth_sum[stayer_index] +
                        projected[stayer_index]:^4
                    copy_first_correlation = copy_first_correlation +
                        physical_random:*physical_projected
                    copy_third_correlation = copy_third_correlation +
                        physical_random:*physical_projected:^3
                }
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
    else if (hybrid) {
        stayer_p_first = projection_square_sum[stayer_physical_row]
        stayer_p_second = projection_fourth_sum[stayer_physical_row]
        stayer_m_first = probes:+stayer_p_first:-
            2:*copy_first_correlation
        stayer_m_second = probes:+6:*stayer_p_first:+stayer_p_second:-
            4:*copy_first_correlation:-4:*copy_third_correlation
        stayer_mixed_second = stayer_p_first:+stayer_p_second:-
            2:*copy_third_correlation
    }
    p_mean = p_first :/ probes
    m_mean = m_first :/ probes
    denominator = p_mean + m_mean
    if (hasmissing(denominator) | min(denominator) <= block_tolerance) {
        return(vckss__failure("JLA_CONSTRAINT_FAILED", "JLA projection and residual masses do not have positive sum"))
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
        return(vckss__failure("JLA_MOMENT_FAILED", "finite-projection variance estimate is negative"))
    }
    finite_variance = finite_variance :* (finite_variance :> 0)
    if (hybrid) {
        stayer_p_mean = stayer_p_first :/ probes
        stayer_m_mean = stayer_m_first :/ probes
        stayer_denominator = stayer_p_mean + stayer_m_mean
        if (hasmissing(stayer_denominator) |
            min(stayer_denominator) <= block_tolerance) {
            return(vckss__failure("JLA_CONSTRAINT_FAILED",
                "stayer JLA projection and residual masses do not have positive sum"))
        }
        stayer_p_constrained = stayer_p_mean :/ stayer_denominator
        stayer_m_constrained = stayer_m_mean :/ stayer_denominator
        stayer_p_second = stayer_p_second :/ probes
        stayer_m_second = stayer_m_second :/ probes
        stayer_mixed_second = stayer_mixed_second :/ probes
        stayer_finite_variance =
            (stayer_m_constrained:^2:*stayer_p_second +
            stayer_p_constrained:^2:*stayer_m_second -
            2:*stayer_p_constrained:*stayer_m_constrained:*
                stayer_mixed_second) :/ probes
        stayer_finite_bias =
            (stayer_m_constrained:*stayer_p_second -
            stayer_p_constrained:*stayer_m_second +
            (stayer_m_constrained-stayer_p_constrained):*
                stayer_mixed_second) :/ probes
        if (min(stayer_finite_variance) < -100*rank_tolerance) {
            return(vckss__failure("JLA_MOMENT_FAILED",
                "finite-projection stayer variance estimate is negative"))
        }
        stayer_finite_variance = stayer_finite_variance :*
            (stayer_finite_variance :> 0)
    }

    if (cols(working_joint.controls) > 0) {
        control_leverage = rowsum(
            (working_joint.residualized_controls*working_joint.schur_inverse) :*
            working_joint.residualized_controls)
        control_factor = cholesky(working_joint.schur_inverse)
        if (hasmissing(control_factor) |
            vckss__norm2(control_factor*control_factor' -
            working_joint.schur_inverse) >
            100*rank_tolerance*
            (1+vckss__norm2(working_joint.schur_inverse))) {
            return(vckss__failure("INVERSE_RESIDUAL_FAILED", "control inverse square root failed its residual gate"))
        }
    }
    else control_leverage = J(n,1,0)

    maximum_leverage = 0
    if (deletion == "observation") {
        copy_control_leverage = control_leverage[physical_row]
        total_residual_leverage = m_constrained - copy_control_leverage
        if (min(total_residual_leverage) <= block_tolerance) {
            return(vckss__failure("NONESTIMABLE_DELETION", "estimated full observation residual leverage is nonpositive"))
        }
        copy_inverse_weight = 1:/total_residual_leverage +
            finite_bias:/total_residual_leverage:^2 -
            finite_variance:/total_residual_leverage:^3
        if (hasmissing(copy_inverse_weight) | min(copy_inverse_weight) <= 0) {
            return(vckss__failure("JLA_INVERSE_FAILED", "finite-projection observation inverse is nonpositive"))
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
            block_frequency = sqrt_frequency[index]
            common_direction = block_frequency :/ sqrt(deletion_frequency[group])
            low_rank = sqrt(p_constrained[group]):*common_direction
            if (cols(working_joint.controls) > 0) {
                block_control = block_frequency :*
                    working_joint.residualized_controls[index,.]
                low_rank = low_rank, block_control*control_factor
            }
            transformed_residual = block_frequency :* residual[index]
            maker_rhs = transformed_residual,common_direction
            reduced_maker = vckss__low_rank_maker(
                low_rank,maker_rhs,rank_tolerance,block_tolerance)
            if (reduced_maker.status != "CONVERGED") {
                return(vckss__failure(
                    reduced_maker.status,reduced_maker.message))
            }
            eigmax = reduced_maker.eigmax
            maximum_leverage = max((maximum_leverage,eigmax))
            solver_residual = max((solver_residual,reduced_maker.relres))
            transformed_residual = reduced_maker.actions[.,1]
            inverse_common = reduced_maker.actions[.,2]
            deleted_adjusted[index] = transformed_residual +
                finite_bias[group] :* inverse_common :*
                (common_direction'*transformed_residual)[1,1] -
                finite_variance[group] :* inverse_common :*
                (common_direction'*inverse_common)[1,1] :*
                (common_direction'*transformed_residual)[1,1]
        }
        if (hybrid) {
            copy_control_leverage = control_leverage[stayer_physical_row]
            total_residual_leverage = stayer_m_constrained -
                copy_control_leverage
            if (min(total_residual_leverage) <= block_tolerance) {
                return(vckss__failure("NONESTIMABLE_DELETION",
                    "estimated full stayer-observation residual leverage is nonpositive"))
            }
            copy_inverse_weight = 1:/total_residual_leverage +
                stayer_finite_bias:/total_residual_leverage:^2 -
                stayer_finite_variance:/total_residual_leverage:^3
            if (hasmissing(copy_inverse_weight) |
                min(copy_inverse_weight) <= 0) {
                return(vckss__failure("JLA_INVERSE_FAILED",
                    "finite-projection stayer-observation inverse is nonpositive"))
            }
            inverse_weight = panelsum(copy_inverse_weight,
                stayer_physical_panel) :/ frequency[stayer_index]
            deleted_adjusted[stayer_index] =
                residual[stayer_index] :* inverse_weight
            maximum_leverage = max((maximum_leverage,
                max(stayer_p_constrained+copy_control_leverage)))
        }
    }

    timer_off(92)
    timer_on(93)

    // The target domain starts from its own registered mt64s stream.  Probe
    // atoms are generated in complete logical-probe order inside the loops
    // below, independent of solver batching.  The public ado guard restores
    // the caller's algorithm, selected stream, and complete state on every
    // return path.
    if (vckss_rng__set_stream_seed(
            vckss_rng__domain_stream("target"),seed)) {
        return(vckss__failure(
            "RNG_SETUP_FAILED",
            "the target-domain mt64s stream could not be initialized"))
    }

    target_draws = J(probes,4,.)
    target_mass = sum(target_weight)
    target_share = target_weight:/target_mass
    target_sqrt_share =
        sqrt(target_weight:/(frequency:*target_mass))
    weighted_y = frequency:*working_y
    weighted_deleted = sqrt_frequency:*deleted_adjusted
    for (batch_start=1; batch_start<=probes; batch_start=batch_start+batch) {
        batch_finish = min((probes,batch_start+batch-1))
        batch_columns = batch_finish-batch_start+1
        if (use_semantic_atoms) {
            rademacher_batch = J(n,batch_columns,0)
            semantic_atom_batch = vckss__draw_semantic_atoms(
                target_semantic_atom_rank,target_semantic_trials,
                batch_columns)
            if (rows(semantic_atom_batch) != rows(target_semantic_panel) |
                cols(semantic_atom_batch) != batch_columns |
                hasmissing(semantic_atom_batch)) {
                return(vckss__failure(
                    "RNG_SEMANTIC_DRAW_FAILED",
                    "registered target-stratum atoms could not be generated"))
            }
            rademacher_batch[target_representative,.] =
                semantic_atom_batch
        }
        else {
            rademacher_batch = J(n,batch_columns,.)
            for (batch_column=1; batch_column<=batch_columns; batch_column++) {
                rademacher_batch[.,batch_column] =
                    vckss__rademacher_sum_prepared(
                        physical_count,physical_panel)
            }
        }
        target_direction_batch = target_sqrt_share:*rademacher_batch
        target_reference_scale =
            vckss__compensated_column_sum(abs(target_direction_batch)) +
            abs(vckss__compensated_column_sum(target_direction_batch))
        target_direction_batch = target_direction_batch -
            target_share*
                vckss__compensated_column_sum(target_direction_batch)
        worker_target = vckss__group_sum(
            target_direction_batch,base.worker_order,base.worker_panel)
        firm_target = vckss__group_sum(
            target_direction_batch,base.firm_order,base.firm_panel)
        worker_target = vckss__balance_zero_sum_score(
            worker_target,target_reference_scale)
        firm_target = vckss__balance_zero_sum_score(
            firm_target,target_reference_scale)
        if (rows(worker_target) != base.worker_levels |
            rows(firm_target) != base.firm_levels |
            cols(worker_target) != batch_columns |
            cols(firm_target) != batch_columns |
            hasmissing(worker_target) | hasmissing(firm_target)) {
            return(vckss__failure(
                "TARGET_CENTERING_FAILED",
                "target score compatibility repair exceeded roundoff"))
        }
        target_rhs = J(base_parameters+1+cols(working_joint.controls),
            2*batch_columns,0)
        for (batch_column=1; batch_column<=batch_columns; batch_column++) {
            target_rhs[1..base.worker_levels,2*batch_column-1] =
                worker_target[.,batch_column]
            target_rhs[(base.worker_levels+1)..(base_parameters+1),
                2*batch_column] =
                firm_target[.,batch_column]
        }
        target_solved = vckss__joint_solve(
            working_joint,target_rhs,tolerance,maxiter)
        if (target_solved.status != "CONVERGED") {
            return(vckss__failure(target_solved.status,target_solved.message))
        }
        solver_iterations = max((solver_iterations,target_solved.iterations))
        solver_residual = max((solver_residual,target_solved.relres))
        solver_schur_actions = solver_schur_actions+
            target_solved.schur_actions
        solver_schur_batches = solver_schur_batches+
            target_solved.schur_batches
        solver_precond_applications =
            solver_precond_applications+
            target_solved.preconditioner_applications
        solver_precond_batches = solver_precond_batches+
            target_solved.preconditioner_batches
        solver_schur_seconds = solver_schur_seconds+
            target_solved.schur_seconds
        solver_precond_seconds = solver_precond_seconds+
            target_solved.preconditioner_seconds
        solver_pcg_seconds = solver_pcg_seconds+target_solved.pcg_seconds
        fe_profile = vckss__fe_profile_merge(
            fe_profile,vckss__fe_profile_row(target_solved))
        solver_rhs_diagnostics = solver_rhs_diagnostics \
            vckss__solver_trace_rows(5,batch_start,
                target_solved.rhs_iterations,target_solved.rhs_relres)
        target_prediction_batch = target_solved.prediction
        worker_projection_batch =
            target_prediction_batch[.,2:*(1..batch_columns):-1]
        firm_projection_batch =
            target_prediction_batch[.,2:*(1..batch_columns)]
        total_projection_batch =
            worker_projection_batch+firm_projection_batch
        if (deletion == "observation") {
            correction_weight = weighted_y:*deleted_adjusted
            target_draws[|batch_start,1\batch_finish,1|] =
                colsum(correction_weight:*worker_projection_batch:^2)'
            target_draws[|batch_start,2\batch_finish,2|] =
                colsum(correction_weight:*firm_projection_batch:^2)'
            target_draws[|batch_start,4\batch_finish,4|] =
                colsum(correction_weight:*total_projection_batch:^2)'
        }
        else {
            group_first_batch = panelsum(
                (weighted_y:*worker_projection_batch)[row_order,.],
                deletion_panel)
            group_second_batch = panelsum(
                (weighted_deleted:*worker_projection_batch)[row_order,.],
                deletion_panel)
            target_draws[|batch_start,1\batch_finish,1|] =
                colsum(group_first_batch:*group_second_batch)'
            group_first_batch = panelsum(
                (weighted_y:*firm_projection_batch)[row_order,.],
                deletion_panel)
            group_second_batch = panelsum(
                (weighted_deleted:*firm_projection_batch)[row_order,.],
                deletion_panel)
            target_draws[|batch_start,2\batch_finish,2|] =
                colsum(group_first_batch:*group_second_batch)'
            group_first_batch = panelsum(
                (weighted_y:*total_projection_batch)[row_order,.],
                deletion_panel)
            group_second_batch = panelsum(
                (weighted_deleted:*total_projection_batch)[row_order,.],
                deletion_panel)
            target_draws[|batch_start,4\batch_finish,4|] =
                colsum(group_first_batch:*group_second_batch)'
            if (hybrid) {
                correction_weight = weighted_y:*deleted_adjusted
                target_draws[|batch_start,1\batch_finish,1|] =
                    target_draws[|batch_start,1\batch_finish,1|] +
                    colsum((correction_weight:*
                        worker_projection_batch:^2)[stayer_index,.])'
                target_draws[|batch_start,2\batch_finish,2|] =
                    target_draws[|batch_start,2\batch_finish,2|] +
                    colsum((correction_weight:*
                        firm_projection_batch:^2)[stayer_index,.])'
                target_draws[|batch_start,4\batch_finish,4|] =
                    target_draws[|batch_start,4\batch_finish,4|] +
                    colsum((correction_weight:*
                        total_projection_batch:^2)[stayer_index,.])'
            }
        }
        target_draws[|batch_start,3\batch_finish,3|] = 0.5 :*
            (target_draws[|batch_start,4\batch_finish,4|]-
            target_draws[|batch_start,1\batch_finish,1|]-
            target_draws[|batch_start,2\batch_finish,2|])
    }
    correction = colsum(target_draws) :/ probes
    numerical_mcse = vckss__mcse(target_draws)
    timer_off(93)
    if (hasmissing(plugin) | hasmissing(correction) |
        hasmissing(numerical_mcse)) {
        return(vckss__failure("NONFINITE_CORRECTION", "JLA target correction is nonfinite"))
    }
    corrected = plugin-correction
    if (hasmissing(corrected)) {
        return(vckss__failure("NONFINITE_CORRECTED_TARGET", "JLA corrected target is nonfinite"))
    }

    out.status = "CONVERGED"
    out.message = "matrix-free improved-JLA KSS calculation converged"
    out.plugin = plugin
    out.correction = correction
    out.corrected = corrected
    out.numerical_mcse = numerical_mcse
    out.n_stored = n
    out.n_physical = sum(frequency)
    out.worker_levels = base.worker_levels
    out.firm_levels = base.firm_levels
    out.parameters = base_parameters + cols(working_joint.controls)
    out.full_parameters = full_parameters
    out.correction_parameters =
        base_parameters + cols(working_joint.controls)
    if (hybrid) out.deletion_units = groups + stayer_physical_count
    else out.deletion_units = groups
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
    // Public stage timers are disjoint: setup/preconditioner preparation is
    // reported separately from the full fit, leverage, and target phases.
    out.fit_seconds = vckss__timer_seconds(91)
    out.leverage_seconds = vckss__timer_seconds(92)
    out.target_seconds = vckss__timer_seconds(93)
    out.correction_seconds = out.leverage_seconds+out.target_seconds
    out.preconditioner_seconds = setup_seconds
    out.schur_seconds = solver_schur_seconds
    out.preconditioner_apply_seconds = solver_precond_seconds
    out.pcg_seconds = solver_pcg_seconds
    out.solver_backend_seconds = out.preconditioner_seconds+solver_pcg_seconds
    out.solver_iterations = solver_iterations
    out.solver_max_residual = solver_residual
    out.solver_schur_actions = solver_schur_actions
    out.solver_schur_batches = solver_schur_batches
    out.solver_precond_applications = solver_precond_applications
    out.solver_precond_batches = solver_precond_batches
    out.solver_rhs_diagnostics = solver_rhs_diagnostics
    /* FE-BUF-PERF-V1 measurement baseline.  The legacy implementation
       materializes every Schur batch; the buffered candidate replaces these
       counts without changing the scientific solver contract. */
    out.fe_workspace_applicable = 1
    out.fe_workspace_builds = fe_profile[1]
    out.fe_buffered_schur_batches = fe_profile[2]
    out.fe_legacy_schur_batches = fe_profile[3]
    out.fe_buffered_schur_columns = fe_profile[4]
    out.fe_legacy_schur_columns = fe_profile[5]
    out.fe_packed_fallback_batches = fe_profile[6]
    out.fe_max_buffer_width = fe_profile[7]
    out.fe_workspace_peak_bytes = fe_profile[8]
    out.fe_cell_bytes_avoided = fe_profile[9]
    out.probes = probes
    return(out)
}

struct vckss_result scalar vckss__jla(
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
    struct vckss_fe_design scalar base
    struct vckss_solver_backend scalar backend
    real scalar setup_seconds

    timer_clear(94)
    timer_on(94)
    base = vckss__fe_prepare(worker,firm,frequency,rank_tolerance)
    timer_off(94)
    if (base.status != "CONVERGED") {
        return(vckss__failure(base.status,base.message))
    }
    setup_seconds = vckss__timer_seconds(94)
    backend = vckss__diagonal_backend()
    return(vckss__jla_backend(
        y,worker,firm,controls,frequency,target_weight,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit,
        base,backend,setup_seconds))
}

void vckss__stata_jla(
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
    string scalar diagnostics_name,
    string scalar solver_diagnostics_name)
{
    struct vckss_result scalar out
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
    out = vckss__jla(
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
        out.deletion_rank_gap,out.full_parameters,
        out.correction_parameters,out.schur_seconds,
        out.preconditioner_apply_seconds,out.pcg_seconds,
        out.solver_backend_seconds,out.solver_schur_actions,
        out.solver_schur_batches,out.solver_precond_applications,
        out.solver_precond_batches,out.fe_workspace_applicable,
        out.fe_workspace_builds,out.fe_buffered_schur_batches,
        out.fe_legacy_schur_batches,out.fe_buffered_schur_columns,
        out.fe_legacy_schur_columns,out.fe_packed_fallback_batches,
        out.fe_max_buffer_width,out.fe_workspace_peak_bytes,
        out.fe_cell_bytes_avoided)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_matrix(solver_diagnostics_name,out.solver_rhs_diagnostics)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

void vckss__stata_exact(
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
    struct vckss_result scalar out
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
    out = vckss__exact(
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
        out.control_schur_rcond,out.deletion_rank_gap,
        out.full_parameters,out.correction_parameters)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

void vckss__stata_exact_stayer_hybrid(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar stayer_name,
    string scalar sample_name,
    string scalar nuisance,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar exact_limit,
    real scalar blocksize_limit,
    string scalar results_name,
    string scalar status_local,
    string scalar message_local,
    string scalar diagnostics_name,
    string scalar correction_source_name)
{
    struct vckss_result scalar out
    real colvector y, worker, firm, frequency, target, deletion_id
    real colvector stayer
    real matrix controls, results, diagnostics

    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    stayer = st_data(.,stayer_name,sample_name)
    out = vckss__exact_stayer_hybrid(
        y,worker,firm,controls,frequency,target,deletion_id,stayer,
        nuisance,rank_tolerance,block_tolerance,exact_limit,
        blocksize_limit)

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
        out.deletion_rank_gap,out.full_parameters,
        out.correction_parameters)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_matrix(correction_source_name,out.correction_by_source)
    st_local(status_local,out.status)
    st_local(message_local,out.message)
}

end
