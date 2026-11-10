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
    return("vckss-residual-se-api1")
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
    basis = vckss_rse__features(leverage[|1\movers|],mass[|1\movers|],1)
    terms = cols(basis)
    fitted[|1\movers|] = basis*(invsym(quadcross(basis,basis))*
        quadcross(basis,product[|1\movers|]))
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
    permutation = ix[order(deletion[ix],1)]
    panel = panelsetup(deletion[permutation],1)
    movers = rows(panel)
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
    real colvector root, product, variance, scaled_residual, eigenvalues
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
        // Under corrected centering the posted estimate adds an increment
        // that this mean-centred kernel does not carry, so nothing to check.
        point_identity = abs(quadcross(y,influence[.,j])-posted_corrected[1,j])
        if (st_global("VCKSS_CENTERING") == "corrected") point_identity = 0
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

end
