*! fevc inference runtime 0.5.0-rc.1 05sep2026

version 18.0

mata:
mata set matastrict on
mata set matalnum off

real scalar vckss_inference__api_level()
{
    return(2)
}

string scalar vckss_inference__build_id()
{
    return("vckss-inference-api2-q1-target-status-projection-mean1-component-mean1-mixed1")
}

real colvector vckss_inf__mover(
    real colvector worker,
    real colvector firm)
{
    real scalar group
    real colvector mover, permutation, sorted_worker
    real matrix panel

    mover = J(rows(worker),1,0)
    permutation = order(worker,1)
    sorted_worker = worker[permutation]
    panel = panelsetup(sorted_worker,1)
    for (group=1; group<=rows(panel); group++) {
        if (min(firm[permutation[|panel[group,1]\panel[group,2]|]]) !=
            max(firm[permutation[|panel[group,1]\panel[group,2]|]])) {
            mover[permutation[|panel[group,1]\panel[group,2]|]] = J(
                panel[group,2]-panel[group,1]+1,1,1)
        }
    }
    return(mover)
}

real colvector vckss_inf__rank_bins(
    real colvector value,
    real scalar bins)
{
    real scalar row, n
    real colvector out, permutation

    n = rows(value)
    out = J(n,1,.)
    permutation = order(value,1)
    for (row=1; row<=n; row++) {
        out[permutation[row]] = min((bins,
            floor((row-1)*bins/n)+1))
    }
    return(out)
}

real colvector vckss_inf__smooth_stratum(
    real colvector leverage,
    real colvector target_diagonal,
    real colvector raw_variance,
    real scalar maximum_cells)
{
    real scalar cell, cells, dimension_bins, finish, group
    real scalar bandwidth, neighbors, scale_leverage, scale_target
    real scalar fitted, denominator, local_count
    real colvector bin_leverage, bin_target, permutation, joint_id
    real colvector cell_leverage, cell_target, cell_variance, cell_weight
    real colvector distance, distance_sorted, weight, output, local_index
    real matrix key, panel, local_design, local_gram
    struct vckss_inverse_result scalar local_inverse

    if (rows(leverage) == 0) return(J(0,1,.))
    dimension_bins = max((1,floor(sqrt(maximum_cells))))
    dimension_bins = min((dimension_bins,rows(leverage)))
    bin_leverage = vckss_inf__rank_bins(leverage,dimension_bins)
    bin_target = vckss_inf__rank_bins(target_diagonal,dimension_bins)
    key = (bin_leverage,bin_target)
    permutation = order(key,(1,2))
    panel = panelsetup(key[permutation,.],1)
    cells = rows(panel)
    joint_id = J(rows(leverage),1,.)
    cell_leverage = cell_target = cell_variance = cell_weight = J(cells,1,.)
    for (cell=1; cell<=cells; cell++) {
        finish = panel[cell,2]-panel[cell,1]+1
        joint_id[permutation[|panel[cell,1]\panel[cell,2]|]] =
            J(finish,1,cell)
        cell_leverage[cell] = mean(leverage[
            permutation[|panel[cell,1]\panel[cell,2]|]])
        cell_target[cell] = mean(target_diagonal[
            permutation[|panel[cell,1]\panel[cell,2]|]])
        cell_variance[cell] = mean(raw_variance[
            permutation[|panel[cell,1]\panel[cell,2]|]])
        cell_weight[cell] = finish
    }
    if (cells < 6) return(J(rows(leverage),1,
        sum(cell_weight:*cell_variance)/sum(cell_weight)))

    scale_leverage = sqrt(mean((cell_leverage:-mean(cell_leverage)):^2))
    scale_target = sqrt(mean((cell_target:-mean(cell_target)):^2))
    if (scale_leverage == 0) scale_leverage = 1
    if (scale_target == 0) scale_target = 1
    neighbors = min((cells,max((8,ceil(cells^(2/3))))))
    output = J(rows(leverage),1,.)
    for (cell=1; cell<=cells; cell++) {
        distance = ((cell_leverage:-cell_leverage[cell]):/
            scale_leverage):^2 + ((cell_target:-cell_target[cell]):/
            scale_target):^2
        distance_sorted = sort(distance,1)
        bandwidth = sqrt(distance_sorted[neighbors])
        if (bandwidth <= 0) {
            weight = cell_weight :* (distance :== 0)
        }
        else {
            weight = J(cells,1,0)
            local_index = selectindex(distance :< bandwidth^2)
            local_count = rows(local_index)
            if (local_count > 0) {
                weight[local_index] = cell_weight[local_index] :*
                    (1:-(sqrt(distance[local_index]):/bandwidth):^3):^3
            }
        }
        denominator = sum(weight)
        if (denominator <= 0) {
            fitted = sum(cell_weight:*cell_variance)/sum(cell_weight)
        }
        else {
            local_design = (J(cells,1,1),
                (cell_leverage:-cell_leverage[cell]):/scale_leverage,
                (cell_target:-cell_target[cell]):/scale_target)
            local_gram = local_design'*(weight:*local_design)
            local_inverse = vckss__inverse(local_gram,1e-12)
            if (local_inverse.status == "CONVERGED") {
                fitted = (local_inverse.inverse *
                    (local_design'*(weight:*cell_variance)))[1]
            }
            else fitted = sum(weight:*cell_variance)/denominator
        }
        output[selectindex(joint_id:==cell)] = J(
            rows(selectindex(joint_id:==cell)),1,fitted)
    }
    return(output)
}

real colvector vckss_inf__smooth(
    real colvector leverage,
    real colvector target_diagonal,
    real colvector raw_variance,
    real colvector mover,
    real scalar maximum_cells)
{
    real scalar stratum
    real colvector index, output

    output = J(rows(leverage),1,.)
    for (stratum=0; stratum<=1; stratum++) {
        index = selectindex(mover:==stratum)
        if (rows(index) > 0) {
            output[index] = vckss_inf__smooth_stratum(
                leverage[index],target_diagonal[index],
                raw_variance[index],maximum_cells)
        }
    }
    return(output)
}

real colvector vckss_inf__W(
    real colvector y,
    real matrix design,
    real matrix inverse,
    real matrix target,
    real colvector beta,
    real colvector leverage,
    real colvector leaveout_residual,
    real colvector target_diagonal)
{
    real colvector first, xi_input, xi

    first = design*inverse*target*beta
    xi_input = target_diagonal:*y:/(1:-leverage)
    xi = xi_input-design*inverse*(design'*xi_input)
    return(first-0.5:*(target_diagonal:*leaveout_residual+xi))
}

real colvector vckss_inf__simquad(
    real matrix design,
    real matrix inverse,
    real matrix target,
    real colvector leverage,
    real colvector target_diagonal,
    real colvector variance,
    real scalar simulations,
    real scalar seed,
    real colvector mode,
    real scalar mode_eigenvalue)
{
    real scalar begin, batch, finish
    real matrix draw, simulated_y, simulated_beta, simulated_residual
    real matrix simulated_leaveout
    real rowvector plugin, correction
    real colvector output

    rseed(seed)
    output = J(simulations,1,.)
    batch = min((32,simulations))
    for (begin=1; begin<=simulations; begin=begin+batch) {
        finish = min((simulations,begin+batch-1))
        draw = rnormal(rows(design),finish-begin+1,0,1)
        // Fixed-c inference keeps the quadratic error kernel unchanged:
        // these are uncentered zero-mean errors, not new observed outcomes.
        simulated_y = (sqrt(variance)*J(1,cols(draw),1)):*draw
        simulated_beta = inverse*(design'*simulated_y)
        simulated_residual = simulated_y-design*simulated_beta
        simulated_leaveout = simulated_residual :/
            ((1:-leverage)*J(1,cols(draw),1))
        plugin = colsum(simulated_beta:*(target*simulated_beta))
        if (rows(mode) > 0) {
            plugin = plugin-mode_eigenvalue:*(mode'*simulated_y):^2
        }
        correction = colsum((target_diagonal*J(1,cols(draw),1)):*
            simulated_y:*simulated_leaveout)
        output[|begin\finish|] = (plugin-correction)'
    }
    return(output)
}

real scalar vckss_inf__variance(real colvector value)
{
    if (rows(value) < 2) return(.)
    return(sum((value:-mean(value)):^2)/(rows(value)-1))
}

real scalar vckss_inf__critical(
    real scalar curvature,
    real scalar confidence,
    real scalar simulations,
    real scalar seed)
{
    real scalar index
    real colvector first, second, distance

    rseed(seed)
    first = abs(rnormal(simulations,1,0,1))
    if (curvature <= 1e-10) distance = first
    else {
        second = abs(rnormal(simulations,1,0,1))
        distance = (first:^2+second:^2+2:*first:/curvature) :/
            (sqrt(second:^2+(first:+1/curvature):^2):+1/curvature)
    }
    distance = sort(distance,1)
    index = min((simulations,max((1,ceil(confidence*simulations)))))
    return(distance[index])
}

real scalar vckss_inf__objective(
    real scalar angle,
    real colvector center,
    real matrix root,
    real scalar radius,
    real scalar eigenvalue)
{
    real colvector point

    point = center+radius*root*(cos(angle)\sin(angle))
    return(eigenvalue*point[1]^2+point[2])
}

real scalar vckss_inf__refine(
    real scalar left,
    real scalar right,
    real colvector center,
    real matrix root,
    real scalar radius,
    real scalar eigenvalue,
    real scalar maximize)
{
    real scalar iteration, first, second, first_value, second_value
    real scalar golden

    golden = (sqrt(5)-1)/2
    first = right-golden*(right-left)
    second = left+golden*(right-left)
    for (iteration=1; iteration<=80; iteration++) {
        first_value = vckss_inf__objective(
            first,center,root,radius,eigenvalue)
        second_value = vckss_inf__objective(
            second,center,root,radius,eigenvalue)
        if ((maximize & first_value < second_value) |
            (!maximize & first_value > second_value)) {
            left = first
            first = second
            second = left+golden*(right-left)
        }
        else {
            right = second
            second = first
            first = right-golden*(right-left)
        }
    }
    return((left+right)/2)
}

real rowvector vckss_inf__am_ci(
    real colvector center,
    real matrix covariance,
    real scalar radius,
    real scalar eigenvalue)
{
    real scalar angle, grid, index, lower, step, upper
    real colvector objective
    real matrix root

    root = cholesky(covariance)
    if (hasmissing(root)) return((.,.))
    grid = 8192
    step = 2*pi()/grid
    objective = J(grid,1,.)
    for (index=1; index<=grid; index++) {
        angle = (index-1)*step
        objective[index] = vckss_inf__objective(
            angle,center,root,radius,eigenvalue)
    }
    index = selectindex(objective:==min(objective))[1]
    angle = vckss_inf__refine((index-2)*step,
        index*step,center,root,radius,eigenvalue,0)
    lower = vckss_inf__objective(
        angle,center,root,radius,eigenvalue)
    index = selectindex(objective:==max(objective))[1]
    angle = vckss_inf__refine((index-2)*step,
        index*step,center,root,radius,eigenvalue,1)
    upper = vckss_inf__objective(
        angle,center,root,radius,eigenvalue)
    return((lower,upper))
}

// Independent inference units: collapsed original mover blocks followed by
// literal stayer observations. No observation-by-observation operator is built.
struct vckss_inf_units {
    real matrix design
    real colvector y, residual, mass
    real scalar movers
}

struct vckss_inf_units scalar vckss_inf__units(
    real matrix design, real colvector y, real colvector residual,
    real colvector frequency, real colvector deletion, real colvector stayer)
{
    struct vckss_inf_units scalar u
    real colvector ix, permutation, members
    real matrix panel
    real scalar g, first, last, row, copy, out, total, mass
    ix = selectindex(stayer:==0)
    permutation = ix[order(deletion[ix],1)]
    panel = panelsetup(deletion[permutation],1)
    u.movers = rows(panel)
    total = u.movers+sum(select(frequency,stayer:==1))
    u.design = J(total,cols(design),0)
    u.y = u.residual = u.mass = J(total,1,0)
    for (g=1; g<=u.movers; g++) {
        first = panel[g,1]
        last = panel[g,2]
        members = permutation[|first\last|]
        mass = sum(frequency[members])
        u.design[g,.] = sqrt(mass)*design[members[1],.]
        u.y[g] = quadcross(frequency[members],y[members])/sqrt(mass)
        u.residual[g] = quadcross(frequency[members],residual[members])/sqrt(mass)
        u.mass[g] = mass
    }
    out = u.movers
    for (row=1; row<=rows(y); row++) {
        if (!stayer[row]) continue
        for (copy=1; copy<=frequency[row]; copy++) {
            out++
            u.design[out,.] = design[row,.]
            u.y[out] = y[row]
            u.residual[out] = residual[row]
            u.mass[out] = 1
        }
    }
    return(u)
}

real colvector vckss_inf__midranks(real colvector x)
{
    real colvector permutation, result
    real scalar first, last, n
    n = rows(x)
    permutation = order(x,1)
    result = J(n,1,.)
    first = 1
    while (first<=n) {
        last = first
        while (last<n) {
            if (x[permutation[last+1]]!=x[permutation[first]]) break
            last++
        }
        result[permutation[|first\last|]] =
            J(last-first+1,1,(first+last-1)/n-1)
        first = last+1
    }
    return(result)
}

real matrix vckss_inf__reduce_basis(real matrix Z)
{
    real matrix orthogonal, kept
    real colvector residual
    real scalar j, k, pass, original, remaining, scale, error
    orthogonal = kept = J(rows(Z),0,.)
    for (j=1; j<=cols(Z); j++) {
        residual = Z[.,j]
        original = sqrt(quadcross(residual,residual))
        scale = max(abs(residual))
        for (pass=1; pass<=2; pass++) {
            for (k=1; k<=cols(orthogonal); k++) {
                residual = residual-orthogonal[.,k]*
                    quadcross(orthogonal[.,k],residual)
            }
        }
        remaining = sqrt(quadcross(residual,residual))
        error = original==0 ? 0 : max((remaining/original,max(abs(residual))/scale))
        if (j!=1 & error<=1e-12) continue
        kept = (kept,Z[.,j])
        orthogonal = (orthogonal,residual/remaining)
    }
    return(kept)
}

real colvector vckss_inf__feature_ranks(real colvector values)
{
    real scalar scale, grid
    scale = max(abs(values))
    if (scale==0) return(vckss_inf__midranks(values))
    // Same explicit roundoff tie convention as the pooled native model.
    grid = 2^(floor(ln(scale)/ln(2))-40)
    return(vckss_inf__midranks(sign(values):*floor(abs(values):/grid:+.5)))
}

real matrix vckss_inf__polynomial(real matrix values, string scalar model)
{
    real scalar j, k
    real matrix ranks, Z
    ranks = values
    for (j=1; j<=cols(values); j++) ranks[.,j] = vckss_inf__feature_ranks(values[.,j])
    if (model=="structured_leverage") return((J(rows(values),1,1),ranks[.,1],ranks[.,1]:^2))
    Z = (J(rows(values),1,1),ranks,ranks:^2)
    for (j=1; j<=cols(ranks); j++) {
        for (k=j+1; k<=cols(ranks); k++) Z = (Z,ranks[.,j]:*ranks[.,k])
    }
    return(Z)
}

struct vckss_inf_variance {
    string scalar status
    real colvector sigma, raw, coefficients
    real matrix gram, basis
    real scalar floor, floored, moment_residual, gram_rcond, gram_relres
}

struct vckss_inf_variance scalar vckss_inf__structured(
    real matrix X, real matrix inverse, real colvector residual,
    real colvector h, real matrix diagonals, real colvector mass,
    real scalar movers, string scalar model, real scalar rank_tolerance)
{
    struct vckss_inf_variance scalar fit
    struct vckss_inverse_result scalar system
    real matrix Zm, Zs, Z, B, root
    real colvector rhs, sorted
    real scalar j, n, middle, scale
    fit.status = "INFERENCE_VARIANCE_SUPPORT"
    n = rows(X)
    Zm = vckss_inf__reduce_basis(vckss_inf__polynomial(
        (h[|1\movers|],diagonals[|1,1\movers,3|],mass[|1\movers|]),model))
    if (movers<5*cols(Zm)) return(fit)
    if (movers<n) {
        // A stayer perturbation changes only its worker effect. Firm and
        // covariance diagonals are structural zeros, not rankable roundoff.
        Zs = vckss_inf__reduce_basis(vckss_inf__polynomial(
            (h[|movers+1\n|],diagonals[|movers+1,1\n,1|],J(n-movers,2,0)),model))
        if (n-movers<5*cols(Zs)) return(fit)
        Z = (Zm,J(movers,cols(Zs),0)\J(n-movers,cols(Zm),0),Zs)
        Z[.,1] = J(n,1,1)
    }
    else Z = Zm
    // tr(H^-1 A_a H^-1 A_b) retains cross-type projection terms.
    // Each column of B is a vectorized coefficient-space symmetric matrix.
    root = cholesky(inverse)
    B = J(cols(X)^2,cols(Z),.)
    for (j=1; j<=cols(Z); j++) B[.,j] = vec(root'*quadcross(X,Z[.,j],X)*root)
    fit.gram = quadcross(Z,(1:-2:*h),Z)+quadcross(B,B)
    fit.gram = (fit.gram+fit.gram')/2
    system = vckss__inverse(fit.gram,rank_tolerance)
    fit.gram_rcond = system.rcond
    fit.gram_relres = system.relres
    fit.status = system.status
    if (system.status!="CONVERGED") return(fit)
    rhs = quadcross(Z,residual:^2)
    fit.coefficients = system.inverse*rhs
    fit.moment_residual = sqrt(quadcross(fit.gram*fit.coefficients-rhs,
        fit.gram*fit.coefficients-rhs))/max((1e-300,sqrt(quadcross(rhs,rhs))))
    if (fit.moment_residual>1e-9 | missing(fit.moment_residual)) {
        fit.status = "INVERSE_RESIDUAL_FAILED"
        return(fit)
    }
    sorted = sort(residual:^2:/(1:-h),1)
    middle = floor(n/2)
    scale = mod(n,2) ? sorted[middle+1] : (sorted[middle]+sorted[middle+1])/2
    fit.floor = 1e-8*scale
    fit.raw = Z*fit.coefficients
    fit.floored = sum(fit.raw:<fit.floor)
    if (fit.floor<=0 | missing(fit.floor) | hasmissing(fit.raw) | fit.floored==n) {
        fit.status = "INFERENCE_VARIANCE_INVALID"
        return(fit)
    }
    fit.sigma = rowmax((fit.raw,J(n,1,fit.floor)))
    fit.basis = Z
    return(fit)
}

void vckss_inference__stata(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar base_deletion_name,
    string scalar base_sample_name,
    string scalar hybrid_worker_name,
    string scalar hybrid_firm_name,
    string scalar hybrid_deletion_name,
    string scalar hybrid_stayer_name,
    string scalar hybrid_sample_name,
    string scalar deletion_mode,
    string scalar stayers_mode,
    string scalar nuisance,
    string scalar inference,
    real scalar confidence_level,
    real scalar simulations,
    real scalar inference_seed,
    real scalar inference_bins,
    string scalar project_names,
    string scalar project_effect,
    string scalar project_weight,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit,
    string scalar corrected_name,
    string scalar V_primitive_name,
    string scalar V_name,
    string scalar highrank_name,
    string scalar q1_name,
    string scalar q1_status_name,
    string scalar projection_b_name,
    string scalar projection_V_name,
    string scalar projection_V_naive_name,
    string scalar projection_results_name,
    string scalar diagnostics_name,
    string scalar status_local,
    string scalar message_local,
    | string scalar component_model,
    string scalar variance_diagnostics_name, string scalar spectrum_name)
{
    real scalar structured, joint_available, point_identity
    real colvector common_sigma, unit_mass
    struct vckss_inf_units scalar units
    struct vckss_inf_variance scalar variance_fit
    real scalar control_count, critical_simulations, firm_levels, j, k, n
    real scalar parameters, worker_levels, z, scale, psd_cleanup
    real scalar projection_psd_cleanup, variance_floor_count
    real scalar lambda, eigen_index, eigen_share, max_weight_sq
    real scalar variance_b, covariance_b_theta, variance_theta
    real scalar raw_variance_b, direct_remainder, remainder_identity_error
    real scalar standardized_determinant, remainder_influence, remainder_trace
    real scalar curvature, critical_value, F_statistic, tiny
    real scalar diagnostic_simulations, diagnostic_seed, diagnostic_bins
    real scalar projection_count, row, unit_frequency, group, begin, finish
    real scalar minimum_maker, eigmax, inverse_forward_bound
    real scalar rank_verification_margin, control_downstream_bound
    real scalar block_solver_residual
    real colvector y, worker, firm, frequency, target_weight, deletion_id
    real colvector stayer, row_order, index, block_frequency
    real colvector working_y, component_y, beta, residual, leverage, leaveout_residual
    real colvector raw_variance, projection_raw_variance, projection_y, mover
    real colvector transformed_residual, deleted_residual
    real colvector diagonal_j, sigma_j, W_j
    real colvector eigenvalues, mode, qsim2, projection_weight_vector
    real colvector projection_b, projection_se, projection_naive_se
    real matrix controls, full_design, design, information, working_information
    real matrix inverse, inverse_factor, deleted_information, block_design
    real matrix low_rank, sorted_delete, deletion_panel
    real matrix targets_worker, targets_firm, targets_covariance, target_j
    real matrix target_diagonal, W, qsim, V_primitive, transform, V
    real matrix highrank, q1, eigenvectors, inverse_root, eigen_system, component_spectrum
    real matrix q1_status
    real matrix interval, covariance_q1, center
    real matrix projects, projection_design, projection_gram
    real matrix projection_cross, projection_loading, projection_score
    real matrix projection_V, projection_V_naive, projection_results
    real rowvector projection_y_score, projection_deleted_score
    real rowvector projection_residual_score
    real matrix diagnostics, posted_corrected
    struct vckss_inverse_result scalar full_inverse, working_inverse
    struct vckss_inverse_result scalar deleted_information_inverse
    struct vckss_inverse_result scalar projection_inverse
    struct vckss_maker_result scalar reduced_maker
    struct vckss_control_basis_result scalar canonical_controls
    struct vckss_target_matrices scalar targets

    structured = component_model=="structured_common" | component_model=="structured_leverage"
    joint_available = 1
    st_local("inference_joint_available","1")
    st_local(status_local,"INVALID_INPUT")
    st_local(message_local,"inference inputs were not accepted")
    if (deletion_mode == "match" & stayers_mode == "both") {
        worker_name = hybrid_worker_name
        firm_name = hybrid_firm_name
        base_deletion_name = hybrid_deletion_name
        base_sample_name = hybrid_sample_name
    }
    y = st_data(.,y_name,base_sample_name)
    worker = st_data(.,worker_name,base_sample_name)
    firm = st_data(.,firm_name,base_sample_name)
    frequency = st_data(.,frequency_name,base_sample_name)
    target_weight = st_data(.,target_name,base_sample_name)
    deletion_id = st_data(.,base_deletion_name,base_sample_name)
    if (deletion_mode == "match" & stayers_mode == "both") {
        stayer = st_data(.,hybrid_stayer_name,base_sample_name)
    }
    else stayer = J(rows(y),1,0)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),base_sample_name)
    if (strtrim(project_names) == "") projects = J(rows(y),0,.)
    else projects = st_data(.,tokens(project_names),base_sample_name)
    n = rows(y)
    if (n == 0 | hasmissing(y) | hasmissing(worker) | hasmissing(firm) |
        hasmissing(frequency) | hasmissing(target_weight) |
        hasmissing(deletion_id) | hasmissing(stayer) |
        hasmissing(controls) | hasmissing(projects) |
        rows(deletion_id) != n | rows(stayer) != n |
        any((stayer :!= 0) :& (stayer :!= 1))) {
        st_local(message_local,"inference inputs contain missing values")
        return
    }
    if (deletion_mode != "observation" & deletion_mode != "match") {
        st_local(status_local,"UNSUPPORTED_DELETION")
        st_local(message_local,
            "projection inference deletion mode must be observation or match")
        return
    }
    if (inference != "none" & deletion_mode != "observation" & !structured) {
        st_local(status_local,"INFERENCE_DELETION_UNSUPPORTED")
        st_local(message_local,
            "high-rank and q1 component inference require observation deletion")
        return
    }
    if (inference != "none" & sum(stayer) != 0 & !structured) {
        st_local(status_local,"INFERENCE_STAYER_UNSUPPORTED")
        st_local(message_local,
            "high-rank and q1 component inference require the retained observation population")
        return
    }
    if (deletion_mode == "observation" & sum(stayer) != 0) {
        st_local(status_local,"INTERNAL_INVARIANT_FAILED")
        st_local(message_local,
            "observation deletion received a mixed-deletion stayer mask")
        return
    }
    if (block_tolerance <= 0 | block_tolerance >= 1 |
        blocksize_limit < 1 | blocksize_limit != floor(blocksize_limit)) {
        st_local(status_local,"INVALID_TOLERANCE")
        st_local(message_local,"projection block gates are invalid")
        return
    }
    unit_frequency = (min(frequency) == 1 & max(frequency) == 1)
    if (!unit_frequency & inference != "none" & !structured) {
        st_local(status_local,"INFERENCE_FREQUENCY_UNSUPPORTED")
        st_local(message_local,
            "high-rank and q1 inference currently require unit frequency weights")
        return
    }
    worker_levels = max(worker)
    firm_levels = max(firm)
    control_count = cols(controls)
    if (structured & st_global("VCKSS_MEMORY_ACTIVE")=="1") {
        // Price stored and transformed designs, exact coefficient contractions,
        // variance bases, and common simulations before allocation or RNG.
        parameters = worker_levels+firm_levels-1+control_count
        scale = n+sum(select(frequency,stayer:==1))
        z = component_model=="structured_common" ? 36 : 6
        scale = 8*(4*n*parameters+3*scale*parameters+(z+20)*parameters^2+
            (4*z+120)*scale+5*simulations+64*z^2)
        st_global("VCKSS_MEMORY_FORECAST",strofreal(max((scale,
            strtoreal(st_global("VCKSS_MEMORY_FORECAST")))),"%21.0f"))
        if (st_global("VCKSS_MEMORY_ADVISORY")!="1" &
            scale>strtoreal(st_global("VCKSS_MEMORY_BYTES"))) {
            st_local(status_local,"RESOURCE_LIMIT")
            st_local(message_local,"exact structured inference allocation forecast exceeds memory_gib()")
            return
        }
    }
    if (control_count > 0) {
        canonical_controls = vckss__canonical_controls(
            controls,frequency,rank_tolerance)
        if (canonical_controls.status != "CONVERGED") {
            st_local(status_local,canonical_controls.status)
            st_local(message_local,canonical_controls.message)
            return
        }
        controls = canonical_controls.controls
    }
    full_design = vckss__design(worker,firm,controls,
        worker_levels,firm_levels)
    if (unit_frequency) information = full_design'*full_design
    else information = full_design'*(frequency:*full_design)
    full_inverse = vckss__inverse(information,rank_tolerance)
    if (full_inverse.status != "CONVERGED") {
        st_local(status_local,full_inverse.status)
        st_local(message_local,"inference full-design inverse failed")
        return
    }
    if (nuisance == "fixedoffset" & control_count > 0) {
        if (unit_frequency) beta = full_inverse.inverse*(full_design'*y)
        else beta = full_inverse.inverse*(full_design'*(frequency:*y))
        working_y = y-controls*beta[
            (cols(full_design)-control_count+1)..cols(full_design)]
        parameters = worker_levels+firm_levels-1
        design = full_design[.,1..parameters]
        if (unit_frequency) working_information = design'*design
        else working_information = design'*(frequency:*design)
        working_inverse = vckss__inverse(
            working_information,rank_tolerance)
        if (working_inverse.status != "CONVERGED") {
            st_local(status_local,working_inverse.status)
            st_local(message_local,"inference fixed-offset inverse failed")
            return
        }
        inverse = working_inverse.inverse
    }
    else {
        working_y = y
        design = full_design
        parameters = cols(design)
        working_information = information
        working_inverse = full_inverse
    }
    inverse = working_inverse.inverse
    if (unit_frequency) beta = inverse*(design'*working_y)
    else beta = inverse*(design'*(frequency:*working_y))
    residual = working_y-design*beta
    leverage = rowsum((design*inverse):*design)
    leaveout_residual = J(n,1,.)
    if (deletion_mode == "observation") {
        if (min(1:-leverage) <= block_tolerance) {
            st_local(status_local,"NONESTIMABLE_DELETION")
            st_local(message_local,
                "inference requires every observation-deleted fit to remain identified")
            return
        }
        leaveout_residual = residual:/(1:-leverage)
    }
    else if (sum(stayer) > 0) {
        index = selectindex(stayer :== 1)
        if (min(1:-leverage[index]) <= block_tolerance) {
            st_local(status_local,"NONESTIMABLE_DELETION")
            st_local(message_local,
                "a stayer physical-observation deletion loses combined-design rank")
            return
        }
        leaveout_residual[index] = residual[index]:/(1:-leverage[index])
    }
    // Freeze the retained working-outcome mean for component inference.
    // The fitted coefficients and deleted residuals stay on the original
    // outcome; target matrices annihilate the common-shift coefficient direction.
    component_y = working_y
    if (st_global("VCKSS_CENTERING") == "mean") {
        component_y = working_y :-
            vckss_nmc__center_mean(working_y,frequency)
    }
    raw_variance = J(n,1,.)
    if (inference != "none") raw_variance = component_y:*leaveout_residual
    // Projection uses the same physical-frequency mean only in its variance
    // proxy, independently of target and projection weights.
    projection_y = component_y
    projection_raw_variance = J(n,1,.)
    if (deletion_mode == "observation") {
        projection_raw_variance = projection_y:*leaveout_residual
    }
    else if (sum(stayer) > 0) {
        index = selectindex(stayer :== 1)
        projection_raw_variance[index] =
            projection_y[index]:*leaveout_residual[index]
    }
    mover = vckss_inf__mover(worker,firm)
    targets = vckss__targets(worker,firm,target_weight,
        worker_levels,firm_levels,parameters)
    targets_worker = targets.worker
    targets_firm = targets.firm
    targets_covariance = targets.covariance
    if (structured) {
        if (deletion_mode!="match" | nuisance!="fixedoffset" | cols(projects)>0) {
            st_local(status_local,"STRUCTURED_INFERENCE_TUPLE_REQUIRED")
            return
        }
        units = vckss_inf__units(design,component_y,residual,
            frequency,deletion_id,stayer)
        design = units.design
        component_y = units.y
        residual = units.residual
        unit_mass = units.mass
        n = rows(component_y)
        leverage = rowsum((design*inverse):*design)
        if (min(1:-leverage)<=block_tolerance) {
            st_local(status_local,"NONESTIMABLE_DELETION")
            return
        }
        leaveout_residual = residual:/(1:-leverage)
        raw_variance = component_y:*leaveout_residual
        mover = (J(units.movers,1,1)\J(n-units.movers,1,0))
        target_diagonal = (vckss__target_diagonal(design*inverse,targets_worker),
            vckss__target_diagonal(design*inverse,targets_firm),
            vckss__target_diagonal(design*inverse,targets_covariance))
        variance_fit = vckss_inf__structured(design,inverse,residual,
            leverage,target_diagonal,unit_mass,units.movers,
            component_model,rank_tolerance)
        if (variance_fit.status!="CONVERGED") {
            st_local(status_local,variance_fit.status)
            st_local(message_local,"the joint residual-moment variance model failed its support, rank, or positivity gate")
            return
        }
        common_sigma = variance_fit.sigma
    }
    posted_corrected = st_matrix(corrected_name)
    z = invnormal(1-(1-confidence_level/100)/2)
    tiny = 1e-10*max((1e-30,max(abs(raw_variance))))

    V_primitive = V = highrank = q1 = J(0,0,.)
    psd_cleanup = 0
    projection_psd_cleanup = 0
    variance_floor_count = structured ? variance_fit.floored : 0
    if (inference != "none") {
        target_diagonal = W = J(n,3,.)
        qsim = J(simulations,3,.)
        V_primitive = J(3,3,.)
        for (j=1; j<=3; j++) {
            if (j == 1) target_j = targets_worker
            else if (j == 2) target_j = targets_firm
            else target_j = targets_covariance
            diagonal_j = vckss__target_diagonal(design*inverse,target_j)
            sigma_j = structured ? common_sigma : vckss_inf__smooth(leverage,
                diagonal_j,raw_variance,mover,inference_bins)
            if (min(sigma_j) < -tiny | hasmissing(sigma_j)) {
                st_local(status_local,"NEGATIVE_INFERENCE_VARIANCE")
                st_local(message_local,
                    "the binned local-linear variance fit produced a materially negative value")
                return
            }
            variance_floor_count = variance_floor_count+
                sum(sigma_j:<0)
            sigma_j[selectindex(sigma_j:<0)] = J(
                rows(selectindex(sigma_j:<0)),1,0)
            W_j = vckss_inf__W(component_y,design,inverse,
                target_j,beta,leverage,leaveout_residual,diagonal_j)
            if (structured) {
                point_identity = abs(quadcross(component_y,W_j)-posted_corrected[1,j])
                if (missing(point_identity) | point_identity>1e-9*max((1,abs(posted_corrected[1,j])))) {
                    st_local(status_local,"TARGET_IDENTITY_FAILED")
                    st_local(message_local,"mixed-unit component influence does not reproduce the pooled point estimate")
                    return
                }
            }
            target_diagonal[.,j] = diagonal_j
            W[.,j] = W_j
            qsim[.,j] = vckss_inf__simquad(
                design,inverse,target_j,leverage,diagonal_j,
                sigma_j,simulations,inference_seed,J(0,1,.),0)
            V_primitive[j,j] = 4*sum(W_j:^2:*sigma_j)-
                vckss_inf__variance(qsim[.,j])
            if (!structured & (V_primitive[j,j] <= 0 | missing(V_primitive[j,j]))) {
                st_local(status_local,"INFERENCE_VARIANCE_INVALID")
                st_local(message_local,
                    "a scalar component variance estimate is nonpositive")
                return
            }
        }
        for (j=1; j<=3; j++) {
            for (k=j+1; k<=3; k++) {
                if (j == 1) target_j = targets_worker
                else if (j == 2) target_j = targets_firm
                else target_j = targets_covariance
                if (k == 1) target_j = target_j+targets_worker
                else if (k == 2) target_j = target_j+targets_firm
                else target_j = target_j+targets_covariance
                diagonal_j = target_diagonal[.,j]+target_diagonal[.,k]
                sigma_j = structured ? common_sigma : vckss_inf__smooth(leverage,diagonal_j,
                    raw_variance,mover,inference_bins)
                if (min(sigma_j) < -tiny | hasmissing(sigma_j)) {
                    st_local(status_local,"NEGATIVE_INFERENCE_VARIANCE")
                    st_local(message_local,
                        "a polarized variance fit produced a materially negative value")
                    return
                }
                variance_floor_count = variance_floor_count+
                    sum(sigma_j:<0)
                sigma_j[selectindex(sigma_j:<0)] = J(
                    rows(selectindex(sigma_j:<0)),1,0)
                qsim2 = vckss_inf__simquad(design,inverse,target_j,
                    leverage,diagonal_j,sigma_j,simulations,
                    inference_seed,J(0,1,.),0)
                scale = 4*sum((W[.,j]+W[.,k]):^2:*sigma_j)-
                    vckss_inf__variance(qsim2)
                V_primitive[j,k] = V_primitive[k,j] =
                    (scale-V_primitive[j,j]-V_primitive[k,k])/2
            }
        }
        eigenvectors = eigenvalues = .
        symeigensystem(V_primitive,eigenvectors,eigenvalues)
        scale = max((1e-30,max(abs(diagonal(V_primitive)))))
        if (min(eigenvalues) < -1e-8*scale |
            min(diagonal(V_primitive)) <= 0 | hasmissing(V_primitive)) {
            if (!structured) {
                st_local(status_local,"INFERENCE_COVARIANCE_NOT_PSD")
                st_local(message_local,
                    "the joint component covariance estimate is not positive semidefinite")
                return
            }
            joint_available = 0
            st_local("inference_joint_available","0")
        }
        if (joint_available & min(eigenvalues) < 0) {
            psd_cleanup = abs(min(eigenvalues))
            eigenvalues[selectindex(eigenvalues:<0)] = J(
                1,cols(selectindex(eigenvalues:<0)),0)
            V_primitive = eigenvectors*diag(eigenvalues)*eigenvectors'
        }
        transform = (1,0,0\0,1,0\0,0,1\1,1,2)
        V = transform*V_primitive*transform'
        highrank = J(4,4,.)
        highrank[.,1] = posted_corrected'
        highrank[.,2] = sqrt(diagonal(V))
        if (structured) {
            for (j=1; j<=4; j++) if (V[j,j]<=0) highrank[j,2] = .
        }
        highrank[.,3] = highrank[.,1]-z:*highrank[.,2]
        highrank[.,4] = highrank[.,1]+z:*highrank[.,2]
    }

    if (structured) {
        component_spectrum = J(4,15,0)
        inverse_root = cholesky(inverse)
        for (j=1; j<=4; j++) {
            if (j==1) target_j = targets_worker
            else if (j==2) target_j = targets_firm
            else if (j==3) target_j = targets_covariance
            else target_j = targets_worker+targets_firm+2:*targets_covariance
            eigen_system = inverse_root'*target_j*inverse_root
            symeigensystem(eigen_system,eigenvectors,eigenvalues)
            index = order(abs(eigenvalues)',-1)
            lambda = eigenvalues[index[1]]
            scale = sum(eigenvalues:^2)
            mode = design*inverse_root*eigenvectors[.,index[1]]
            component_spectrum[j,1..10] = (lambda,eigenvalues[index[2]],
                scale,scale,0,0,lambda^2/scale,0,
                eigenvalues[index[2]]^2/max((1e-300,scale-lambda^2)),max(mode:^2))
            component_spectrum[j,11] = sqrt(quadcross(
                eigen_system*eigenvectors[.,index[1]]-lambda*eigenvectors[.,index[1]],
                eigen_system*eigenvectors[.,index[1]]-lambda*eigenvectors[.,index[1]]))/max((1e-300,abs(lambda)))
            if (j<=3) W_j=W[.,j]
            else W_j=W[.,1]+W[.,2]+2:*W[.,3]
            component_spectrum[j,15] = max(W_j:^2:*common_sigma)/max((1e-300,sum(W_j:^2:*common_sigma)))
        }
    }

    q1_status = J(0,0,.)
    if (inference == "q1") {
        q1 = J(4,17,.)
        q1_status = J(4,7,.)
        q1_status[.,1] = J(4,1,0)
        critical_simulations = max((100000,100*simulations))
        inverse_root = cholesky(inverse)
        if (hasmissing(inverse_root)) {
            st_local(status_local,"INFERENCE_EIGEN_FAILURE")
            st_local(message_local,
                "the information inverse square root could not be formed")
            return
        }
        for (j=1; j<=4; j++) {
            q1[j,1..4] = (posted_corrected[1,j],sqrt(V[j,j]),
                posted_corrected[1,j]-z*sqrt(V[j,j]),
                posted_corrected[1,j]+z*sqrt(V[j,j]))
            if (j == 1) target_j = targets_worker
            else if (j == 2) target_j = targets_firm
            else if (j == 3) target_j = targets_covariance
            else target_j = targets_worker+targets_firm+
                2:*targets_covariance
            eigen_system = inverse_root'*target_j*inverse_root
            eigenvectors = eigenvalues = .
            symeigensystem(eigen_system,eigenvectors,eigenvalues)
            if (hasmissing(eigenvalues) |
                max(abs(eigenvalues)) <= 1e-14) {
                q1_status[j,1] = 4
                continue
            }
            eigen_index = selectindex(abs(eigenvalues):==
                max(abs(eigenvalues)))[1]
            lambda = eigenvalues[eigen_index]
            mode = design*inverse_root*eigenvectors[.,eigen_index]
            mode = mode/sqrt(quadcross(mode,mode))
            eigen_share = lambda^2/sum(eigenvalues:^2)
            max_weight_sq = max(mode:^2)
            diagonal_j = vckss__target_diagonal(design*inverse,target_j)
            sigma_j = structured ? common_sigma : vckss_inf__smooth(leverage,
                diagonal_j,raw_variance,mover,inference_bins)
            if (min(sigma_j) < -tiny | hasmissing(sigma_j)) {
                q1_status[j,1] = 5
                continue
            }
            variance_floor_count = variance_floor_count+
                sum(sigma_j:<0)
            sigma_j[selectindex(sigma_j:<0)] = J(
                rows(selectindex(sigma_j:<0)),1,0)
            diagonal_j = diagonal_j-lambda:*mode:^2
            W_j = vckss_inf__W(component_y,design,inverse,
                target_j,beta,leverage,leaveout_residual,
                diagonal_j)-lambda:*mode:*(mode'*component_y)
            variance_b = sum(mode:^2:*sigma_j)
            covariance_b_theta = 2*sum(mode:*sigma_j:*W_j)
            qsim2 = vckss_inf__simquad(
                design,inverse,target_j,leverage,diagonal_j,
                sigma_j,simulations,inference_seed,mode,lambda)
            remainder_influence = 4*sum(W_j:^2:*sigma_j)
            remainder_trace = vckss_inf__variance(qsim2)
            variance_theta = remainder_influence-remainder_trace
            raw_variance_b = sum(mode:^2:*raw_variance)
            center = ((mode'*component_y)[1]\
                posted_corrected[1,j]-lambda*((mode'*component_y)[1]^2-
                raw_variance_b))
            direct_remainder = quadcross(component_y,W_j)
            remainder_identity_error = abs(center[2]-direct_remainder)
            if (missing(remainder_identity_error) |
                remainder_identity_error > 1e-9*
                max((1,abs(center[2]),abs(direct_remainder)))) {
                st_local(status_local,"TARGET_IDENTITY_FAILED")
                st_local(message_local,
                    "the q=1 raw leave-out recenter does not reproduce the direct remainder")
                return
            }
            standardized_determinant = 1-(covariance_b_theta/
                sqrt(variance_b)/sqrt(variance_theta))^2
            q1_status[j,2..7] = (standardized_determinant,
                remainder_influence,remainder_trace,raw_variance_b,
                remainder_identity_error,.)
            q1[j,7..14] = (lambda,eigen_share,max_weight_sq,
                variance_b,covariance_b_theta,variance_theta,center')
            if (variance_b <= 0 | variance_theta <= 0 |
                missing(variance_b) | missing(variance_theta)) {
                q1_status[j,1] = 1
                continue
            }
            if (missing(standardized_determinant) |
                standardized_determinant <= 1e-12) {
                q1_status[j,1] = 2
                continue
            }
            covariance_q1 = (variance_b,covariance_b_theta\
                covariance_b_theta,variance_theta)
            curvature = 2*abs(lambda)*variance_b/
                sqrt(variance_theta*standardized_determinant)
            critical_value = vckss_inf__critical(
                curvature,confidence_level/100,critical_simulations,
                mod(inference_seed+104729*j,2147483629)+1)
            interval = vckss_inf__am_ci(center,
                covariance_q1,critical_value,lambda)
            F_statistic = center[1]^2/variance_b
            if (hasmissing(interval) | missing(critical_value) |
                critical_value <= 0 | missing(F_statistic)) {
                q1_status[j,1] = 3
                continue
            }
            q1[j,.] = (posted_corrected[1,j],sqrt(V[j,j]),
                posted_corrected[1,j]-z*sqrt(V[j,j]),
                posted_corrected[1,j]+z*sqrt(V[j,j]),
                interval[1],interval[2],lambda,eigen_share,
                max_weight_sq,variance_b,covariance_b_theta,
                variance_theta,center[1],center[2],F_statistic,
                curvature,critical_value)
        }
    }

    projection_b = projection_V = projection_V_naive =
        projection_results = J(0,0,.)
    if (cols(projects) > 0) {
        projection_design = (J(n,1,1),projects)
        projection_count = cols(projection_design)
        if (project_weight == "target") {
            projection_weight_vector = target_weight
        }
        else projection_weight_vector = frequency
        projection_gram = projection_design' *
            (projection_weight_vector:*projection_design)
        projection_inverse = vckss__inverse(
            projection_gram,rank_tolerance)
        if (projection_inverse.status != "CONVERGED") {
            st_local(status_local,"PROJECTION_DESIGN_SINGULAR")
            st_local(message_local,
                "project() is collinear after adding the automatic constant")
            return
        }
        projection_cross = J(parameters,projection_count,0)
        for (row=1; row<=n; row++) {
            if (project_effect == "worker") {
                projection_cross[worker[row],.] =
                    projection_cross[worker[row],.] +
                    projection_weight_vector[row]:*projection_design[row,.]
            }
            else if (firm[row] < firm_levels) {
                projection_cross[worker_levels+firm[row],.] =
                    projection_cross[worker_levels+firm[row],.] +
                    projection_weight_vector[row]:*projection_design[row,.]
            }
        }
        projection_loading = projection_cross*projection_inverse.inverse
        projection_b = projection_loading'*beta
        projection_score = design*inverse*projection_loading
        projection_V = J(projection_count,projection_count,0)
        projection_V_naive = J(projection_count,projection_count,0)
        inverse_forward_bound = working_inverse.relres /
            max((working_inverse.rcond,rank_tolerance))
        if (hasmissing(inverse_forward_bound) |
            inverse_forward_bound >= 0.01) {
            st_local(status_local,"INVERSE_FORWARD_ERROR_FAILED")
            st_local(message_local,
                "the projection information inverse is too ill-conditioned for deletion-rank certification")
            return
        }
        rank_verification_margin = max((block_tolerance,
            10*inverse_forward_bound))
        inverse_factor = cholesky(inverse)
        if (hasmissing(inverse_factor) |
            vckss__norm2(inverse_factor*inverse_factor'-inverse) >
            100*rank_tolerance*(1+vckss__norm2(inverse))) {
            st_local(status_local,"INVERSE_RESIDUAL_FAILED")
            st_local(message_local,
                "the projection information inverse square root failed its residual gate")
            return
        }
        block_solver_residual = 0
        if (deletion_mode == "observation") {
            for (row=1; row<=n; row++) {
                minimum_maker = 1-leverage[row]
                if (control_count > 0 & nuisance == "joint") {
                    control_downstream_bound = vckss__propagate_error(
                        canonical_controls.forward_error,minimum_maker)
                    if (hasmissing(control_downstream_bound) |
                        control_downstream_bound >
                            vckss__control_forward_limit()) {
                        st_local(status_local,"AMBIGUOUS_CONTROL_BASIS")
                        st_local(message_local,
                            "observation-deletion conditioning cannot certify projection control-basis invariance")
                        return
                    }
                }
                if (minimum_maker <= rank_verification_margin) {
                    deleted_information = working_information-
                        design[row,.]'*design[row,.]
                    deleted_information_inverse = vckss__inverse(
                        deleted_information,rank_tolerance)
                    if (deleted_information_inverse.status != "CONVERGED") {
                        st_local(status_local,"NONESTIMABLE_DELETION")
                        st_local(message_local,
                            "a direct deleted-information factorization rejects an observation deletion")
                        return
                    }
                }
            }
            projection_V = projection_score'*(frequency:*
                projection_raw_variance:*projection_score)
            projection_V_naive = projection_score'*(frequency:*
                residual:^2:*projection_score)
        }
        else {
            index = selectindex(stayer :== 0)
            if (rows(index) == 0) {
                st_local(status_local,"NO_MOVER_SAMPLE")
                st_local(message_local,
                    "match-deletion projection inference requires retained movers")
                return
            }
            row_order = index[order(deletion_id[index],1)]
            sorted_delete = deletion_id[row_order]
            deletion_panel = panelsetup(sorted_delete,1)
            for (group=1; group<=rows(deletion_panel); group++) {
                begin = deletion_panel[group,1]
                finish = deletion_panel[group,2]
                index = row_order[|begin\finish|]
                if (rows(index) > blocksize_limit) {
                    st_local(status_local,"BLOCK_SIZE_LIMIT")
                    st_local(message_local,
                        "a projection deletion block exceeds blocksize_limit()")
                    return
                }
                if (min(worker[index]) != max(worker[index]) |
                    min(firm[index]) != max(firm[index])) {
                    st_local(status_local,"CROSS_COORDINATE_MATCH")
                    st_local(message_local,
                        "each projection deletion ID must remain within one worker-firm coordinate")
                    return
                }
                block_frequency = sqrt(frequency[index])
                block_design = block_frequency:*design[index,.]
                low_rank = block_design*inverse_factor
                transformed_residual = block_frequency:*residual[index]
                reduced_maker = vckss__low_rank_maker(
                    low_rank,transformed_residual,
                    rank_tolerance,block_tolerance)
                if (reduced_maker.status != "CONVERGED") {
                    st_local(status_local,reduced_maker.status)
                    st_local(message_local,reduced_maker.message)
                    return
                }
                block_solver_residual = max((block_solver_residual,
                    reduced_maker.relres))
                eigmax = reduced_maker.eigmax
                minimum_maker = 1-eigmax
                if (minimum_maker <= block_tolerance) {
                    st_local(status_local,"NONESTIMABLE_DELETION")
                    st_local(message_local,
                        "a mover match deletion loses projection-model rank")
                    return
                }
                if (control_count > 0 & nuisance == "joint") {
                    control_downstream_bound = vckss__propagate_error(
                        canonical_controls.forward_error,minimum_maker)
                    if (hasmissing(control_downstream_bound) |
                        control_downstream_bound >
                            vckss__control_forward_limit()) {
                        st_local(status_local,"AMBIGUOUS_CONTROL_BASIS")
                        st_local(message_local,
                            "match-deletion conditioning cannot certify projection control-basis invariance")
                        return
                    }
                }
                if (minimum_maker <= rank_verification_margin) {
                    deleted_information = working_information-
                        block_design'*block_design
                    deleted_information_inverse = vckss__inverse(
                        deleted_information,rank_tolerance)
                    if (deleted_information_inverse.status != "CONVERGED") {
                        st_local(status_local,"NONESTIMABLE_DELETION")
                        st_local(message_local,
                            "a direct deleted-information factorization rejects a mover match deletion")
                        return
                    }
                }
                deleted_residual = reduced_maker.actions
                projection_raw_variance[index] = projection_y[index]:*
                    deleted_residual:/block_frequency
                projection_y_score = colsum(
                    (frequency[index]:*projection_y[index]):*
                    projection_score[index,.])
                projection_deleted_score = colsum(
                    (block_frequency:*deleted_residual):*
                    projection_score[index,.])
                projection_residual_score = colsum(
                    (frequency[index]:*residual[index]):*
                    projection_score[index,.])
                projection_V = projection_V + 0.5:*(
                    projection_y_score'*projection_deleted_score +
                    projection_deleted_score'*projection_y_score)
                projection_V_naive = projection_V_naive +
                    projection_residual_score'*projection_residual_score
            }
            index = selectindex(stayer :== 1)
            if (rows(index) > 0) {
                for (row=1; row<=rows(index); row++) {
                    minimum_maker = 1-leverage[index[row]]
                    if (control_count > 0 & nuisance == "joint") {
                        control_downstream_bound = vckss__propagate_error(
                            canonical_controls.forward_error,minimum_maker)
                        if (hasmissing(control_downstream_bound) |
                            control_downstream_bound >
                                vckss__control_forward_limit()) {
                            st_local(status_local,"AMBIGUOUS_CONTROL_BASIS")
                            st_local(message_local,
                                "stayer-deletion conditioning cannot certify projection control-basis invariance")
                            return
                        }
                    }
                    if (minimum_maker <= rank_verification_margin) {
                        deleted_information = working_information-
                            design[index[row],.]'*design[index[row],.]
                        deleted_information_inverse = vckss__inverse(
                            deleted_information,rank_tolerance)
                        if (deleted_information_inverse.status != "CONVERGED") {
                            st_local(status_local,"NONESTIMABLE_DELETION")
                            st_local(message_local,
                                "a direct deleted-information factorization rejects a stayer observation deletion")
                            return
                        }
                    }
                }
                projection_V = projection_V +
                    projection_score[index,.]'*
                    (frequency[index]:*projection_raw_variance[index]:*
                    projection_score[index,.])
                projection_V_naive = projection_V_naive +
                    projection_score[index,.]'*(frequency[index]:*
                    residual[index]:^2:*projection_score[index,.])
            }
            if (hasmissing(projection_raw_variance)) {
                st_local(status_local,"INTERNAL_INVARIANT_FAILED")
                st_local(message_local,
                    "the mixed projection deletion partition is incomplete")
                return
            }
        }
        projection_V = 0.5:*(projection_V+projection_V')
        projection_V_naive = 0.5:*(projection_V_naive+
            projection_V_naive')
        eigenvectors = eigenvalues = .
        symeigensystem(projection_V,eigenvectors,eigenvalues)
        scale = max((1e-30,max(abs(diagonal(projection_V)))))
        if (min(diagonal(projection_V)) <= 0 |
            min(eigenvalues) < -1e-8*scale | hasmissing(projection_V)) {
            st_local(status_local,"PROJECTION_COVARIANCE_INVALID")
            st_local(message_local,
                "the KSS projection covariance is not positive semidefinite")
            return
        }
        if (min(eigenvalues) < 0) {
            projection_psd_cleanup = abs(min(eigenvalues))
            eigenvalues[selectindex(eigenvalues:<0)] = J(
                1,cols(selectindex(eigenvalues:<0)),0)
            projection_V = eigenvectors*diag(eigenvalues)*eigenvectors'
        }
        projection_se = sqrt(diagonal(projection_V))
        projection_naive_se = sqrt(diagonal(projection_V_naive))
        projection_results = (projection_b,projection_se,
            projection_b:/projection_se,
            2:*normal(-abs(projection_b:/projection_se)),
            projection_b:-z:*projection_se,
            projection_b:+z:*projection_se,projection_naive_se)
    }

    diagnostic_simulations = simulations
    diagnostic_seed = inference_seed
    diagnostic_bins = inference_bins
    if (inference == "none") {
        diagnostic_simulations = diagnostic_seed = diagnostic_bins = .
    }
    diagnostics = (diagnostic_simulations,diagnostic_seed,diagnostic_bins,
        confidence_level,psd_cleanup,min(raw_variance),
        max(raw_variance),sum(mover),n-sum(mover),
        projection_psd_cleanup,variance_floor_count,
        min(projection_raw_variance),max(projection_raw_variance))
    st_matrix(V_primitive_name,V_primitive)
    st_matrix(V_name,V)
    st_matrix(highrank_name,highrank)
    st_matrix(q1_name,q1)
    st_matrix(q1_status_name,q1_status)
    st_matrix(projection_b_name,projection_b')
    st_matrix(projection_V_name,projection_V)
    st_matrix(projection_V_naive_name,projection_V_naive)
    st_matrix(projection_results_name,projection_results)
    if (structured) {
        st_matrix(variance_diagnostics_name,(cols(variance_fit.basis),
            variance_fit.gram_rcond,variance_fit.gram_relres,
            variance_fit.moment_residual,variance_fit.floor,
            variance_fit.floored,rows(variance_fit.sigma),min(variance_fit.raw)))
        st_matrix(spectrum_name,component_spectrum)
    }
    st_matrix(diagnostics_name,diagnostics)
    st_local(status_local,"CONVERGED")
    if (cols(projects) > 0) {
        st_local(message_local,
            "exact projection inference converged under the requested deletion partition")
    }
    else st_local(message_local,structured ?
        "exact mixed-unit structured component inference converged" :
        "exact-observation component inference converged")
}

end
