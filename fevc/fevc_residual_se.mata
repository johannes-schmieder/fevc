*! Leave-out residual standard errors for the exact Mata route: collapsed
*! deletion units, exact kernels, pseudo-noise trace and calibration, and the
*! first-order term for estimated control coefficients.
*! See docs/standard_errors/standard_errors.tex for the method.

version 18.0

mata:
mata set matastrict on
mata set matalnum off

real scalar vckss_rse__api_level()
{
    return(1)
}

string scalar vckss_rse__build_id()
{
    return("vckss-residual-se-api2")
}

// Subtract the mass-weighted mean of every column in unit space. For one row
// per unit scaled by sqrt(mass), the constant outcome is the direction root.
real matrix vckss_rse__center(real matrix value, real colvector root)
{
    return(value-root*(quadcross(root,value):/quadcross(root,root)))
}

// Image of the columns of `value' under the leave-out kernel C_t, which is
// B_t - (diag(r_t) M + M diag(r_t))/2 with r_t the target diagonal over the
// residual-maker diagonal.
real matrix vckss_rse__kernel(
    real matrix value,
    real matrix design,
    real matrix inverse,
    real matrix target,
    real colvector ratio)
{
    real matrix fitted, scaled

    fitted = inverse*quadcross(design,value)
    scaled = ratio:*value
    return(design*(inverse*(target*fitted))-
        0.5:*(ratio:*(value-design*fitted)+
            scaled-design*(inverse*quadcross(design,scaled))))
}

// Outcome-free features of one unit type: intercept, leverage rank, mass
// rank, and their squares, in that order; types without mass skip its terms.
// A type keeps one coefficient per hundred units, at least the intercept.
real matrix vckss_rse__features(
    real colvector leverage,
    real colvector mass,
    real scalar with_mass)
{
    real matrix basis
    real colvector lever_rank, mass_rank
    real scalar permitted

    lever_rank = vckss_inf__feature_ranks(leverage)
    if (with_mass) {
        mass_rank = vckss_inf__feature_ranks(mass)
        basis = (J(rows(leverage),1,1),lever_rank,mass_rank,
            lever_rank:^2,mass_rank:^2)
    }
    else basis = (J(rows(leverage),1,1),lever_rank,lever_rank:^2)
    permitted = max((1,min((cols(basis),floor(rows(leverage)/100)))))
    return(basis[.,1..permitted])
}

// Working variances for the pseudo-noise: least squares of the leave-out
// products on the features, separately for movers and stayers, floored at a
// small multiple of the median squared residual over the maker diagonal.
// Returns the variances; `info' receives (terms, floor, floored, minimum).
real colvector vckss_rse__working(
    real colvector product,
    real colvector residual,
    real colvector leverage,
    real colvector mass,
    real scalar movers,
    real rowvector info)
{
    real colvector fitted, sorted, variance
    real matrix basis
    real scalar n, middle, scale, floor_value, terms

    n = rows(product)
    fitted = J(n,1,.)
    terms = 0
    if (movers > 0) {
        basis = vckss_rse__features(leverage[|1\movers|],mass[|1\movers|],1)
        terms = cols(basis)
        fitted[|1\movers|] = basis*(invsym(quadcross(basis,basis))*
            quadcross(basis,product[|1\movers|]))
    }
    if (movers < n) {
        basis = vckss_rse__features(leverage[|movers+1\n|],
            mass[|movers+1\n|],0)
        terms = terms+cols(basis)
        fitted[|movers+1\n|] = basis*(invsym(quadcross(basis,basis))*
            quadcross(basis,product[|movers+1\n|]))
    }
    sorted = sort(residual:^2:/(1:-leverage),1)
    middle = floor(n/2)
    scale = mod(n,2) ? sorted[middle+1] : (sorted[middle]+sorted[middle+1])/2
    floor_value = 1e-8*scale
    variance = rowmax((fitted,J(n,1,floor_value)))
    info = (terms,floor_value,sum(fitted:<floor_value),min(fitted))
    return(variance)
}

// Unit aggregates of physical columns in the order of vckss_inf__units():
// mover units by deletion identifier, then one row per stayer copy. With
// within = 0 the mover rows are frequency sums over sqrt(mass); with
// within = 1 they are within-unit cross-products with the deviations of
// `other', and stayer copies are zero.
real matrix vckss_rse__aggregate(
    real matrix value,
    real colvector other,
    real colvector frequency,
    real colvector deletion,
    real colvector stayer,
    real scalar within)
{
    real colvector ix, permutation, members, weight
    real matrix panel, output, deviation
    real scalar g, movers, total, row, copy, out, mass

    ix = selectindex(stayer:==0)
    movers = 0
    if (rows(ix) > 0) {
        permutation = ix[order(deletion[ix],1)]
        panel = panelsetup(deletion[permutation],1)
        movers = rows(panel)
    }
    total = movers+sum(select(frequency,stayer:==1))
    output = J(total,cols(value),0)
    for (g=1; g<=movers; g++) {
        members = permutation[|panel[g,1]\panel[g,2]|]
        weight = frequency[members]
        mass = sum(weight)
        if (within) {
            deviation = value[members,.]:-
                quadcross(weight,value[members,.]):/mass
            output[g,.] = quadcross(weight:*(other[members]:-
                quadcross(weight,other[members])/mass),deviation)
        }
        else output[g,.] = quadcross(weight,value[members,.]):/sqrt(mass)
    }
    if (within) return(output)
    out = movers
    for (row=1; row<=rows(value); row++) {
        if (!stayer[row]) continue
        for (copy=1; copy<=frequency[row]; copy++) {
            out++
            output[out,.] = value[row,.]
        }
    }
    return(output)
}

void vckss_residual_se__stata(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar stayer_name,
    string scalar sample_name,
    real scalar confidence_level,
    real scalar draws,
    real scalar seed,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar check_identity,
    string scalar corrected_name,
    string scalar V_primitive_name,
    string scalar V_name,
    string scalar results_name,
    string scalar diagnostics_name,
    string scalar variance_diagnostics_name,
    string scalar spectrum_name,
    string scalar status_local,
    string scalar message_local)
{
    struct vckss_inf_units scalar units
    struct vckss_inverse_result scalar full_inverse, working_inverse
    struct vckss_control_basis_result scalar canonical_controls
    struct vckss_target_matrices scalar targets
    real colvector y, worker, firm, frequency, target_weight, deletion_id
    real colvector stayer, working_y, beta, residual, leverage, maker_inverse
    real colvector root, product, variance, scaled_residual
    real rowvector eigenvalues
    real colvector fourth
    real matrix controls, full_design, design, inverse, target_j
    real matrix ratio, influence, score, noise, fitted, pseudo_residual
    real matrix image, previous, quadratic, calibration, signal, V_primitive
    real matrix transform, V, results, spectrum, eigenvectors
    real matrix residualized, precision, collapsed, collapsed_residualized
    real matrix within, load, posted_corrected, total_image
    real rowvector info
    real scalar n, parameters, worker_levels, firm_levels, control_count
    real scalar centered, j, k, z, point_identity, begin, finish, batch
    real scalar pairs, width, effective, truncation

    st_local(status_local,"INVALID_INPUT")
    st_local(message_local,"standard-error inputs were not accepted")
    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target_weight = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    if (strtrim(stayer_name) == "") stayer = J(rows(y),1,0)
    else stayer = st_data(.,stayer_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    n = rows(y)
    if (n == 0 | hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(frequency) | hasmissing(target_weight) |
        hasmissing(deletion_id) | hasmissing(stayer) | hasmissing(controls) |
        any((stayer :!= 0) :& (stayer :!= 1))) {
        st_local(message_local,"standard-error inputs contain missing values")
        return
    }
    if (draws < 2 | draws != floor(draws) | seed < 1 |
        block_tolerance <= 0 | block_tolerance >= 1) {
        st_local(status_local,"INVALID_INFERENCE_TUNING")
        st_local(message_local,"standard-error draws, seed, or tolerance is invalid")
        return
    }
    worker_levels = max(worker)
    firm_levels = max(firm)
    control_count = cols(controls)
    if (control_count > 0) {
        canonical_controls = vckss__canonical_controls(
            controls,frequency,rank_tolerance)
        if (canonical_controls.status != "CONVERGED") {
            st_local(status_local,canonical_controls.status)
            st_local(message_local,canonical_controls.message)
            return
        }
        controls = canonical_controls.controls
        control_count = cols(controls)
    }
    // The offset holds the fitted control index fixed; the worker-firm
    // design alone defines the units, leverages, and kernels below.
    full_design = vckss__design(worker,firm,controls,worker_levels,firm_levels)
    parameters = worker_levels+firm_levels-1
    design = full_design[.,1..parameters]
    working_y = y
    if (control_count > 0) {
        full_inverse = vckss__inverse(
            quadcross(full_design,frequency,full_design),rank_tolerance)
        if (full_inverse.status != "CONVERGED") {
            st_local(status_local,full_inverse.status)
            st_local(message_local,"standard-error full-design inverse failed")
            return
        }
        beta = full_inverse.inverse*quadcross(full_design,frequency,y)
        working_y = y-controls*beta[|parameters+1\parameters+control_count|]
    }
    working_inverse = vckss__inverse(
        quadcross(design,frequency,design),rank_tolerance)
    if (working_inverse.status != "CONVERGED") {
        st_local(status_local,working_inverse.status)
        st_local(message_local,"standard-error worker-firm inverse failed")
        return
    }
    inverse = working_inverse.inverse
    beta = inverse*quadcross(design,frequency,working_y)
    residual = working_y-design*beta
    centered = st_global("VCKSS_CENTERING") != "none" &
        st_global("VCKSS_CENTERING") != ""
    targets = vckss__targets(worker,firm,target_weight,
        worker_levels,firm_levels,parameters)

    if (control_count > 0) {
        // Controls after the two-way effects, and the precision of the
        // control coefficients, over physical observations.
        residualized = controls-design*(inverse*
            quadcross(design,frequency,controls))
        precision = invsym(quadcross(residualized,frequency,residualized))
        collapsed = vckss_rse__aggregate(controls,working_y,frequency,
            deletion_id,stayer,0)
        collapsed_residualized = vckss_rse__aggregate(residualized,
            working_y,frequency,deletion_id,stayer,0)
        within = vckss_rse__aggregate(controls,working_y,frequency,
            deletion_id,stayer,1)
    }

    units = vckss_inf__units(design,working_y,residual,frequency,
        deletion_id,stayer)
    design = units.design
    residual = units.residual
    n = rows(residual)
    root = sqrt(units.mass)
    y = units.y
    if (centered) y = vckss_rse__center(y,root)
    leverage = rowsum((design*inverse):*design)
    if (min(1:-leverage) <= block_tolerance) {
        st_local(status_local,"NONESTIMABLE_DELETION")
        st_local(message_local,
            "a deletion unit loses design rank when it is left out")
        return
    }
    maker_inverse = 1:/(1:-leverage)
    product = y:*residual:*maker_inverse

    posted_corrected = st_matrix(corrected_name)
    ratio = influence = J(n,3,.)
    for (j=1; j<=3; j++) {
        if (j == 1) target_j = targets.worker
        else if (j == 2) target_j = targets.firm
        else target_j = targets.covariance
        ratio[.,j] = vckss__target_diagonal(design*inverse,target_j):*
            maker_inverse
        influence[.,j] = vckss_rse__kernel(y,design,inverse,target_j,ratio[.,j])
        if (centered) influence[.,j] = vckss_rse__center(influence[.,j],root)
        // The unit-space kernel reproduces the posted estimate only when
        // that estimate is the mean-centred fixed-offset one: not under
        // corrected centering, and not when controls stay in the operator.
        point_identity = abs(quadcross(y,influence[.,j])-posted_corrected[1,j])
        if (!check_identity | st_global("VCKSS_CENTERING") == "corrected") {
            point_identity = 0
        }
        if (missing(point_identity) |
            point_identity > 1e-9*max((1,abs(posted_corrected[1,j])))) {
            st_local(status_local,"TARGET_IDENTITY_FAILED")
            st_local(message_local,
                "the unit-space kernel does not reproduce the posted point estimate")
            return
        }
    }

    info = J(1,4,.)
    variance = vckss_rse__working(product,residual,leverage,units.mass,
        units.movers,info)
    if (info[2] <= 0 | missing(info[2]) | hasmissing(variance) | info[3] == n) {
        st_local(status_local,"INFERENCE_VARIANCE_INVALID")
        st_local(message_local,
            "the working variances for the pseudo-noise are not positive")
        return
    }

    // Unit scores: squared, they carry the signal term plus a pure-noise
    // amount that the pseudo-noise calibration removes.
    scaled_residual = residual:*sqrt(maker_inverse)
    score = 2:*influence:*scaled_residual
    if (control_count > 0) {
        load = precision*(-2:*quadcross(collapsed,influence))
        score = (2:*influence+collapsed_residualized*load):*scaled_residual+
            within*load
    }

    // Pseudo-noise with the working variances through the same kernel.
    rseed(seed)
    quadratic = calibration = J(3,3,0)
    fourth = J(4,1,0)
    pairs = 0
    previous = J(0,0,.)
    batch = min((32,draws))
    for (begin=1; begin<=draws; begin=begin+batch) {
        finish = min((draws,begin+batch-1))
        width = finish-begin+1
        noise = sqrt(variance):*rnormal(n,width,0,1)
        if (centered) noise = vckss_rse__center(noise,root)
        fitted = inverse*quadcross(design,noise)
        pseudo_residual = (noise-design*fitted):^2:*maker_inverse
        image = J(n,3*width,.)
        for (j=1; j<=3; j++) {
            if (j == 1) target_j = targets.worker
            else if (j == 2) target_j = targets.firm
            else target_j = targets.covariance
            image[|1,(j-1)*width+1\n,j*width|] = vckss_rse__kernel(noise,
                design,inverse,target_j,ratio[.,j])
        }
        if (centered) image = vckss_rse__center(image,root)
        for (j=1; j<=3; j++) {
            for (k=1; k<=3; k++) {
                quadratic[j,k] = quadratic[j,k]+sum(variance:*
                    image[|1,(j-1)*width+1\n,j*width|]:*
                    image[|1,(k-1)*width+1\n,k*width|])
                calibration[j,k] = calibration[j,k]+sum(pseudo_residual:*
                    image[|1,(j-1)*width+1\n,j*width|]:*
                    image[|1,(k-1)*width+1\n,k*width|])
            }
        }
        // Products of consecutive draws estimate the fourth spectral moment.
        for (k=1; k<=width; k++) {
            total_image = (image[.,k],image[.,width+k],image[.,2*width+k])
            total_image = (total_image,total_image[.,1]+total_image[.,2]+
                2:*total_image[.,3])
            if (cols(previous) > 0) {
                fourth = fourth+
                    diagonal(quadcross(previous,variance,total_image)):^2
                pairs = pairs+1
            }
            previous = total_image
        }
    }
    quadratic = quadratic:/draws
    calibration = calibration:/draws
    signal = quadcross(score,score)-4:*calibration
    signal = (signal+signal'):/2
    eigenvectors = eigenvalues = .
    symeigensystem(signal,eigenvectors,eigenvalues)
    truncation = max((0,-min(eigenvalues)))
    eigenvalues = eigenvalues:*(eigenvalues:>0)
    V_primitive = eigenvectors*diag(eigenvalues)*eigenvectors'+2:*quadratic
    V_primitive = (V_primitive+V_primitive'):/2
    if (hasmissing(V_primitive) | min(diagonal(V_primitive)) <= 0) {
        st_local(status_local,"INFERENCE_VARIANCE_INVALID")
        st_local(message_local,"a component variance estimate is not positive")
        return
    }
    transform = (1,0,0\0,1,0\0,0,1\1,1,2)
    V = transform*V_primitive*transform'
    z = invnormal(1-(1-confidence_level/100)/2)
    results = J(4,4,.)
    results[.,1] = posted_corrected'
    results[.,2] = sqrt(diagonal(V))
    results[.,3] = results[.,1]-z:*results[.,2]
    results[.,4] = results[.,1]+z:*results[.,2]

    // Effective rank tr(W^2)^2/tr(W^4) of the variance-scaled kernel, and
    // the largest share of the squared score held by one unit.
    score = (score,score[.,1]+score[.,2]+2:*score[.,3])
    spectrum = J(4,15,0)
    for (j=1; j<=4; j++) {
        if (j <= 3) effective = quadratic[j,j]
        else effective = quadratic[1,1]+quadratic[2,2]+4*quadratic[3,3]+
            2*quadratic[1,2]+4*quadratic[1,3]+4*quadratic[2,3]
        spectrum[j,3] = spectrum[j,4] = effective
        if (pairs > 0 & fourth[j] > 0) {
            effective = max((1,effective^2/(fourth[j]/pairs)))
        }
        else effective = 1
        spectrum[j,7] = 1/sqrt(effective)
        spectrum[j,13] = draws
        spectrum[j,15] = max(score[.,j]:^2)/max((1e-300,sum(score[.,j]:^2)))
    }
    st_matrix(V_primitive_name,V_primitive)
    st_matrix(V_name,V)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,(draws,seed,.,confidence_level,truncation,
        min(product),max(product),units.movers,n-units.movers,0,info[3],.,.))
    st_matrix(variance_diagnostics_name,(info[1],.,.,.,info[2],info[3],n,info[4]))
    st_matrix(spectrum_name,spectrum)
    st_local("inference_joint_available","1")
    st_local("inference_nuisance_omitted",control_count > 0 ? "0" : "1")
    st_local(status_local,"CONVERGED")
    st_local(message_local,"exact leave-out residual standard errors converged")
}

// ---------------------------------------------------------------------------
// Matrix-free route. The same estimator as above with linear solves in place
// of the dense inverse and random-projection estimates of the leverages and
// target diagonals. It is self-contained: it draws its own projections, so
// the kernel it studies has the form of the point estimator's kernel with an
// independent set of projections. Cost: about three solves per probe, one per
// control, and four per pseudo-noise draw.
// ---------------------------------------------------------------------------

// Unit-space rows are sqrt(mass) times a worker dummy plus a firm dummy.
real matrix vckss_rse__unit_transpose(
    struct vckss_fe_design scalar design,
    real colvector root,
    real matrix value)
{
    return(vckss__fe_transpose(design,root:*value))
}

real matrix vckss_rse__unit_predict(
    struct vckss_fe_design scalar design,
    real colvector root,
    real matrix coefficient)
{
    return(root:*vckss__fe_predict(design,coefficient))
}

// Coefficient-space action of the three target matrices on the columns of
// `coefficient', stacked as worker, firm, covariance blocks of columns. The
// targets are share-weighted variances and the covariance of the worker and
// firm effects over the physical rows.
real matrix vckss_rse__target_action(
    struct vckss_fe_design scalar base,
    real colvector share,
    real matrix coefficient)
{
    real matrix worker_effect, firm_coefficient, firm_effect, full
    real matrix worker_side, firm_side, zero_worker, zero_firm
    real scalar workers, firms, columns

    workers = base.worker_levels
    firms = base.firm_levels
    columns = cols(coefficient)
    worker_effect = coefficient[1..workers,.][base.worker,.]
    firm_coefficient = J(firms,columns,0)
    firm_coefficient[1..(firms-1),.] = coefficient[(workers+1)..(workers+firms-1),.]
    firm_effect = firm_coefficient[base.firm,.]
    worker_effect = share:*(worker_effect:-quadcross(share,worker_effect))
    firm_effect = share:*(firm_effect:-quadcross(share,firm_effect))
    zero_worker = J(workers,columns,0)
    zero_firm = J(firms-1,columns,0)
    full = vckss__fe_transpose_full(base,worker_effect)
    worker_side = full[1..workers,.]
    firm_side = full[(workers+1)..(workers+firms-1),.]
    full = vckss__fe_transpose_full(base,firm_effect)
    return(((worker_side\zero_firm),
        (zero_worker\full[(workers+1)..(workers+firms-1),.]),
        0.5:*((full[1..workers,.]\firm_side))))
}

// Kernel images of the columns of `value' for the three targets, stacked as
// three blocks of columns, and the residual of `value' in `residual'.
real matrix vckss_rse__solve_kernel(
    real matrix value,
    struct vckss_fe_design scalar base,
    struct vckss_fe_design scalar units,
    real colvector root,
    real colvector share,
    real matrix ratio,
    real scalar tolerance,
    real scalar maxiter,
    real matrix residual,
    string scalar status)
{
    struct vckss_solve_result scalar solved
    real matrix right_hand_side, image
    real scalar columns, j

    columns = cols(value)
    status = "INFERENCE_SOLVE_FAILED"
    solved = vckss__fe_solve_matrix(units,
        vckss_rse__unit_transpose(units,root,value),tolerance,maxiter)
    if (solved.status != "CONVERGED") return(J(0,0,.))
    residual = value-vckss_rse__unit_predict(units,root,solved.coefficient)
    right_hand_side = vckss_rse__target_action(base,share,solved.coefficient)
    for (j=1; j<=3; j++) {
        right_hand_side[|1,(j-1)*columns+1\rows(right_hand_side),j*columns|] =
            right_hand_side[|1,(j-1)*columns+1\rows(right_hand_side),j*columns|]+
            0.5:*vckss_rse__unit_transpose(units,root,ratio[.,j]:*value)
    }
    solved = vckss__fe_solve_matrix(units,right_hand_side,tolerance,maxiter)
    if (solved.status != "CONVERGED") return(J(0,0,.))
    image = vckss_rse__unit_predict(units,root,solved.coefficient)
    for (j=1; j<=3; j++) {
        image[|1,(j-1)*columns+1\rows(image),j*columns|] =
            image[|1,(j-1)*columns+1\rows(image),j*columns|]-
            0.5:*ratio[.,j]:*(residual+value)
    }
    status = "CONVERGED"
    return(image)
}

// Residual-maker inverse from projection moments, as the point estimator
// forms it (docs/JLA_FINITE_PROJECTION.md): the ratio estimate of the maker
// diagonal with its second-order bias and variance terms. `moments' holds the
// probe means of U, V, U^2, V^2 and UV, where U and V are the squared
// projection and squared residual of one probe. Returns the inverse;
// `leverage' receives one minus the maker diagonal and `floored' the number
// of units held at the floor.
real colvector vckss_rse__finite_projection(
    real matrix moments,
    real scalar probes,
    real colvector leverage,
    real scalar floored)
{
    real colvector maker, bias, spread, multiplier

    maker = moments[.,2]:/(moments[.,1]+moments[.,2])
    floored = sum(maker:<0.02)
    maker = rowmax((maker,J(rows(maker),1,0.02)))
    leverage = 1:-maker
    bias = (maker:*moments[.,3]-leverage:*moments[.,4]+
        (maker-leverage):*moments[.,5]):/probes
    spread = (maker:^2:*moments[.,3]+leverage:^2:*moments[.,4]-
        2:*leverage:*maker:*moments[.,5]):/probes
    multiplier = 1:-spread:/maker:^2+bias:/maker
    multiplier = rowmin((rowmax((multiplier,J(rows(maker),1,0.5))),
        J(rows(maker),1,2)))
    return(multiplier:/maker)
}

void vckss_residual_se_jla__stata(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar stayer_name,
    string scalar sample_name,
    real scalar confidence_level,
    real scalar draws,
    real scalar seed,
    real scalar probes,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    string scalar corrected_name,
    string scalar V_primitive_name,
    string scalar V_name,
    string scalar results_name,
    string scalar diagnostics_name,
    string scalar variance_diagnostics_name,
    string scalar spectrum_name,
    string scalar status_local,
    string scalar message_local)
{
    struct vckss_fe_design scalar base, units
    struct vckss_solve_result scalar solved
    struct vckss_control_basis_result scalar canonical_controls
    real colvector y, worker, firm, frequency, target_weight, deletion_id
    real colvector stayer, working_y, residual, leverage, maker_inverse
    real colvector root, product, variance, scaled_residual
    real rowvector eigenvalues
    real colvector fourth, share, unit_worker, unit_firm, mass, unit_y
    real colvector ix, permutation, members, gamma
    real matrix controls, residualized, precision, collapsed
    real matrix collapsed_residualized, within, load, noise, image, previous
    real matrix quadratic, calibration, signal, V_primitive, transform, V
    real matrix results, spectrum, eigenvectors, ratio, influence, score
    real matrix posted_corrected, total_image, panel, projected, direction
    real matrix full, pseudo_residual, target_diagonal, outcome_fit
    real matrix control_fit, kernel_residual, moments, probe, square
    real matrix complement
    real rowvector info
    real scalar n, workers, firms, control_count, centered, j, k, z
    real scalar begin, finish, batch, pairs, width, effective, truncation
    real scalar movers, total, g, row, copy, out, clipped, unit_mass
    string scalar kernel_status

    st_local(status_local,"INVALID_INPUT")
    st_local(message_local,"standard-error inputs were not accepted")
    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target_weight = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    if (strtrim(stayer_name) == "") stayer = J(rows(y),1,0)
    else stayer = st_data(.,stayer_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    n = rows(y)
    if (n == 0 | hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(frequency) | hasmissing(target_weight) |
        hasmissing(deletion_id) | hasmissing(stayer) | hasmissing(controls) |
        any((stayer :!= 0) :& (stayer :!= 1))) {
        st_local(message_local,"standard-error inputs contain missing values")
        return
    }
    if (draws < 2 | draws != floor(draws) | seed < 1 | probes < 2 |
        probes != floor(probes) | tolerance <= 0 | maxiter < 1) {
        st_local(status_local,"INVALID_INFERENCE_TUNING")
        st_local(message_local,"standard-error draws, probes, seed, or tolerance is invalid")
        return
    }
    control_count = cols(controls)
    if (control_count > 0) {
        canonical_controls = vckss__canonical_controls(
            controls,frequency,rank_tolerance)
        if (canonical_controls.status != "CONVERGED") {
            st_local(status_local,canonical_controls.status)
            st_local(message_local,canonical_controls.message)
            return
        }
        controls = canonical_controls.controls
        control_count = cols(controls)
    }
    base = vckss__fe_prepare(worker,firm,frequency,rank_tolerance)
    if (base.status != "CONVERGED") {
        st_local(status_local,base.status)
        st_local(message_local,base.message)
        return
    }
    workers = base.worker_levels
    firms = base.firm_levels
    centered = st_global("VCKSS_CENTERING") != "none" &
        st_global("VCKSS_CENTERING") != ""
    share = target_weight:/sum(target_weight)
    rseed(seed)

    // Offset outcome, its residual, and the controls after the two-way
    // effects, over physical observations.
    st_local(status_local,"INFERENCE_SOLVE_FAILED")
    st_local(message_local,"a standard-error linear solve did not converge")
    solved = vckss__fe_solve_matrix(base,
        vckss__fe_transpose(base,frequency:*y),tolerance,maxiter)
    if (solved.status != "CONVERGED") return
    outcome_fit = vckss__fe_predict(base,solved.coefficient)
    working_y = y
    residual = y-outcome_fit
    if (control_count > 0) {
        solved = vckss__fe_solve_matrix(base,
            vckss__fe_transpose(base,frequency:*controls),tolerance,maxiter)
        if (solved.status != "CONVERGED") return
        control_fit = vckss__fe_predict(base,solved.coefficient)
        residualized = controls-control_fit
        precision = invsym(quadcross(residualized,frequency,residualized))
        gamma = precision*quadcross(residualized,frequency,residual)
        working_y = y-controls*gamma
        residual = residual-residualized*gamma
        collapsed = vckss_rse__aggregate(controls,working_y,frequency,
            deletion_id,stayer,0)
        collapsed_residualized = vckss_rse__aggregate(residualized,
            working_y,frequency,deletion_id,stayer,0)
        within = vckss_rse__aggregate(controls,working_y,frequency,
            deletion_id,stayer,1)
    }

    // Deletion units in the order of vckss_rse__aggregate(): mover units by
    // deletion identifier, then one unit per stayer copy.
    ix = selectindex(stayer:==0)
    if (rows(ix) > 0) {
        permutation = ix[order(deletion_id[ix],1)]
        panel = panelsetup(deletion_id[permutation],1)
        movers = rows(panel)
    }
    else movers = 0
    total = movers+sum(select(frequency,stayer:==1))
    unit_worker = unit_firm = J(total,1,.)
    mass = J(total,1,1)
    unit_y = product = J(total,1,0)
    for (g=1; g<=movers; g++) {
        members = permutation[|panel[g,1]\panel[g,2]|]
        unit_mass = sum(frequency[members])
        unit_worker[g] = worker[members[1]]
        unit_firm[g] = firm[members[1]]
        mass[g] = unit_mass
        unit_y[g] = quadcross(frequency[members],working_y[members])/sqrt(unit_mass)
        product[g] = quadcross(frequency[members],residual[members])/sqrt(unit_mass)
    }
    out = movers
    for (row=1; row<=n; row++) {
        if (!stayer[row]) continue
        for (copy=1; copy<=frequency[row]; copy++) {
            out++
            unit_worker[out] = worker[row]
            unit_firm[out] = firm[row]
            unit_y[out] = working_y[row]
            product[out] = residual[row]
        }
    }
    residual = product
    n = total
    root = sqrt(mass)
    y = unit_y
    if (centered) y = vckss_rse__center(y,root)
    units = vckss__fe_prepare(unit_worker,unit_firm,mass,rank_tolerance)
    if (units.status != "CONVERGED") {
        st_local(status_local,units.status)
        st_local(message_local,units.message)
        return
    }

    // Random-projection leverages and target diagonals.
    moments = J(n,5,0)
    target_diagonal = J(n,3,0)
    batch = min((32,probes))
    for (begin=1; begin<=probes; begin=begin+batch) {
        finish = min((probes,begin+batch-1))
        width = finish-begin+1
        probe = rnormal(n,width,0,1)
        solved = vckss__fe_solve_matrix(units,
            vckss_rse__unit_transpose(units,root,probe),tolerance,maxiter)
        if (solved.status != "CONVERGED") return
        projected = vckss_rse__unit_predict(units,root,solved.coefficient)
        square = projected:^2
        complement = (probe-projected):^2
        moments = moments+(rowsum(square),rowsum(complement),
            rowsum(square:^2),rowsum(complement:^2),rowsum(square:*complement))
        // One direction per probe on the target population; its worker and
        // firm sides are solved separately and share the same draw.
        direction = sqrt(share):*rnormal(rows(share),width,0,1)
        direction = direction-share*colsum(direction)
        full = vckss__fe_transpose_full(base,direction)
        solved = vckss__fe_solve_matrix(units,
            ((full[1..workers,.]\J(firms-1,width,0)),
            (J(workers,width,0)\full[(workers+1)..(workers+firms-1),.])),
            tolerance,maxiter)
        if (solved.status != "CONVERGED") return
        projected = vckss_rse__unit_predict(units,root,solved.coefficient)
        target_diagonal[.,1] = target_diagonal[.,1]+
            rowsum(projected[|1,1\n,width|]:^2)
        target_diagonal[.,2] = target_diagonal[.,2]+
            rowsum(projected[|1,width+1\n,2*width|]:^2)
        target_diagonal[.,3] = target_diagonal[.,3]+
            rowsum(projected[|1,1\n,width|]:*projected[|1,width+1\n,2*width|])
    }
    moments = moments:/probes
    target_diagonal = target_diagonal:/probes
    leverage = J(n,1,.)
    clipped = 0
    maker_inverse = vckss_rse__finite_projection(moments,probes,leverage,clipped)
    ratio = target_diagonal:*maker_inverse
    product = y:*residual:*maker_inverse

    kernel_status = ""
    kernel_residual = J(0,0,.)
    influence = vckss_rse__solve_kernel(y,base,units,root,share,ratio,
        tolerance,maxiter,kernel_residual,kernel_status)
    if (kernel_status != "CONVERGED") return
    if (centered) influence = vckss_rse__center(influence,root)

    info = J(1,4,.)
    variance = vckss_rse__working(product,residual,leverage,mass,movers,info)
    if (info[2] <= 0 | missing(info[2]) | hasmissing(variance) | info[3] == n) {
        st_local(status_local,"INFERENCE_VARIANCE_INVALID")
        st_local(message_local,
            "the working variances for the pseudo-noise are not positive")
        return
    }
    scaled_residual = residual:*sqrt(maker_inverse)
    score = 2:*influence:*scaled_residual
    if (control_count > 0) {
        load = precision*(-2:*quadcross(collapsed,influence))
        score = (2:*influence+collapsed_residualized*load):*scaled_residual+
            within*load
    }

    quadratic = calibration = J(3,3,0)
    fourth = J(4,1,0)
    pairs = 0
    previous = J(0,0,.)
    batch = min((16,draws))
    for (begin=1; begin<=draws; begin=begin+batch) {
        finish = min((draws,begin+batch-1))
        width = finish-begin+1
        noise = sqrt(variance):*rnormal(n,width,0,1)
        if (centered) noise = vckss_rse__center(noise,root)
        image = vckss_rse__solve_kernel(noise,base,units,root,share,ratio,
            tolerance,maxiter,kernel_residual,kernel_status)
        if (kernel_status != "CONVERGED") return
        if (centered) image = vckss_rse__center(image,root)
        pseudo_residual = kernel_residual:^2:*maker_inverse
        for (j=1; j<=3; j++) {
            for (k=1; k<=3; k++) {
                quadratic[j,k] = quadratic[j,k]+sum(variance:*
                    image[|1,(j-1)*width+1\n,j*width|]:*
                    image[|1,(k-1)*width+1\n,k*width|])
                calibration[j,k] = calibration[j,k]+sum(pseudo_residual:*
                    image[|1,(j-1)*width+1\n,j*width|]:*
                    image[|1,(k-1)*width+1\n,k*width|])
            }
        }
        for (k=1; k<=width; k++) {
            total_image = (image[.,k],image[.,width+k],image[.,2*width+k])
            total_image = (total_image,total_image[.,1]+total_image[.,2]+
                2:*total_image[.,3])
            if (cols(previous) > 0) {
                fourth = fourth+
                    diagonal(quadcross(previous,variance,total_image)):^2
                pairs = pairs+1
            }
            previous = total_image
        }
    }
    quadratic = quadratic:/draws
    calibration = calibration:/draws
    signal = quadcross(score,score)-4:*calibration
    signal = (signal+signal'):/2
    eigenvectors = eigenvalues = .
    symeigensystem(signal,eigenvectors,eigenvalues)
    truncation = max((0,-min(eigenvalues)))
    eigenvalues = eigenvalues:*(eigenvalues:>0)
    V_primitive = eigenvectors*diag(eigenvalues)*eigenvectors'+2:*quadratic
    V_primitive = (V_primitive+V_primitive'):/2
    if (hasmissing(V_primitive) | min(diagonal(V_primitive)) <= 0) {
        st_local(status_local,"INFERENCE_VARIANCE_INVALID")
        st_local(message_local,"a component variance estimate is not positive")
        return
    }
    transform = (1,0,0\0,1,0\0,0,1\1,1,2)
    V = transform*V_primitive*transform'
    posted_corrected = st_matrix(corrected_name)
    z = invnormal(1-(1-confidence_level/100)/2)
    results = J(4,4,.)
    results[.,1] = posted_corrected'
    results[.,2] = sqrt(diagonal(V))
    results[.,3] = results[.,1]-z:*results[.,2]
    results[.,4] = results[.,1]+z:*results[.,2]
    score = (score,score[.,1]+score[.,2]+2:*score[.,3])
    spectrum = J(4,15,0)
    for (j=1; j<=4; j++) {
        if (j <= 3) effective = quadratic[j,j]
        else effective = quadratic[1,1]+quadratic[2,2]+4*quadratic[3,3]+
            2*quadratic[1,2]+4*quadratic[1,3]+4*quadratic[2,3]
        spectrum[j,3] = spectrum[j,4] = effective
        if (pairs > 0 & fourth[j] > 0) {
            effective = max((1,effective^2/(fourth[j]/pairs)))
        }
        else effective = 1
        spectrum[j,7] = 1/sqrt(effective)
        spectrum[j,13] = draws
        spectrum[j,15] = max(score[.,j]:^2)/max((1e-300,sum(score[.,j]:^2)))
    }
    // The kernel image of the outcome reproduces the point estimate only up
    // to projection noise, because the two sets of projections differ.
    st_matrix(V_primitive_name,V_primitive)
    st_matrix(V_name,V)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,(draws,seed,probes,confidence_level,truncation,
        min(product),max(product),movers,n-movers,clipped,info[3],
        max(abs(quadcross(y,influence[.,1..3])-posted_corrected[1,1..3])),.))
    st_matrix(variance_diagnostics_name,(info[1],.,.,.,info[2],info[3],n,info[4]))
    st_matrix(spectrum_name,spectrum)
    st_local("inference_joint_available","1")
    st_local("inference_nuisance_omitted",control_count > 0 ? "0" : "1")
    st_local(status_local,"CONVERGED")
    st_local(message_local,"matrix-free leave-out residual standard errors converged")
}

// Defined last: present only when the whole file compiled.
real scalar vckss_rse__loaded()
{
    return(1)
}

end
