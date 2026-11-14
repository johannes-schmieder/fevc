*! Leave-out residual standard errors for the exact Mata route: collapsed
*! deletion units, exact kernels, pseudo-noise trace and calibration, and the
*! first-order term for estimated control coefficients. The same pass carries
*! repeat-measurement standard errors: stayers' own sample variances, a
*! two-step fitted model for mover units, and the high-rank covariance with a
*! closed-form term for the pairs of rows of one stayer.
*! See docs/standard_errors/standard_errors.tex for the methods.

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
    return("vckss-residual-se-api6")
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

// Variances for repeat-measurement standard errors. Deletion units that
// share a worker-firm cell have identical regressors and independent errors,
// so a cell with two or more units is a repeat-measurement set and its units
// receive the sample variance of their outcomes: the physical observations
// of an eligible stayer under match deletion, and every match of two or more
// observations under observation deletion. Units alone in their cell receive
// a scale (the working-variance features of leverage and mass ranks) times a
// quadratic in the standardised worker and firm effects limited to three
// standard deviations, both steps by least squares on leave-out products;
// with fewer than a hundred such units the model is fitted on all units of
// movers. `diagonal' holds the three target diagonals. `info' receives
// (terms, floor, floored, minimum) as in vckss_rse__working; `shape' receives
// (scale terms, profile terms, scale floor, scale floored, sets, set units,
// set units floored, worker-variance pair term, six profile coefficients);
// `pair' receives the 3 x 3 closed-form covariance term for the pairs of
// units of one set. A stayer's errors are absorbed by its worker effect, so
// a stayer contributes to the worker variance alone.
real colvector vckss_rse__repeat(
    real colvector product,
    real colvector y,
    real colvector residual,
    real colvector leverage,
    real colvector maker_inverse,
    real colvector mass,
    real colvector unit_set,
    real colvector unit_mover,
    real colvector worker_effect,
    real colvector firm_effect,
    real matrix diagonal,
    real rowvector info,
    real rowvector shape,
    real matrix pair)
{
    real colvector sorted, variance, scale, fitted, a, f, coefficient
    real colvector permutation, members, single, fit, used
    real matrix basis, profile, panel, inverse
    real rowvector weight
    real scalar n, middle, floor_value, scale_floor, permitted, g, count
    real scalar sample, total, mean_effect, deviation, units

    n = rows(product)
    sorted = sort(residual:^2:/(1:-leverage),1)
    middle = floor(n/2)
    floor_value = 1e-8*(mod(n,2) ? sorted[middle+1] :
        (sorted[middle]+sorted[middle+1])/2)
    variance = J(n,1,.)
    info = (0,floor_value,0,.)
    shape = J(1,14,0)
    pair = J(3,3,0)
    if (floor_value <= 0 | missing(floor_value)) return(variance)

    // Sets: the sample variance of the outcomes of one cell, and the pair
    // term. Two units a, b of one cell share the weight B_aa/(1 - P_aa); a
    // stayer's leverage is exactly 1/T, which gives B_aa T/(T-1).
    single = J(n,1,0)
    permutation = order((unit_set,(1::n)),(1,2))
    panel = panelsetup(unit_set[permutation],1)
    for (g=1; g<=rows(panel); g++) {
        members = permutation[|panel[g,1]\panel[g,2]|]
        count = rows(members)
        if (count < 2) {
            if (!unit_mover[members[1]]) {
                info[2] = 0
                return(J(n,1,.))
            }
            single[members[1]] = 1
            continue
        }
        sample = quadvariance(y[members])
        if (unit_mover[members[1]]) {
            weight = mean(diagonal[members,.]:*maker_inverse[members])
        }
        else weight = (mean(diagonal[members,1])*count/(count-1),0,0)
        pair = pair+(4*sample^2*(count-2)*(count-1)/(count+1)):*
            quadcross(weight,weight)
        shape[5] = shape[5]+1
        shape[6] = shape[6]+count
        shape[7] = shape[7]+count*(sample < floor_value)
        variance[members] = J(count,1,max((sample,floor_value)))
    }
    shape[8] = pair[1,1]
    units = sum(single)
    if (units > 0) {
        // The model is fitted where it is used; too few units alone in
        // their cell cannot carry it, and all units of movers then do. Under
        // match deletion the two samples coincide.
        fit = units >= 100 ? selectindex(single) : selectindex(unit_mover)
        used = single[fit]
        // Step 1: the scale, floored at five per cent of the mean product.
        basis = vckss_rse__features(leverage[fit],mass[fit],1)
        inverse = invsym(quadcross(basis,basis))
        scale = basis*(inverse*quadcross(basis,product[fit]))
        scale_floor = 0.05*mean(product[fit])
        if (scale_floor <= 0 | missing(scale_floor)) {
            info[2] = 0
            return(J(n,1,.))
        }
        // Terms that the others do not span, as the native route counts them.
        shape[1] = sum(diagonal(inverse):!=0)
        shape[3] = scale_floor
        shape[4] = sum(used:*(scale:<scale_floor))
        scale = rowmax((scale,J(rows(fit),1,scale_floor)))
        // Step 2: the profile. Effects are standardised over the physical
        // observations of all units, so a common shift between worker and
        // firm effects does not matter.
        total = sum(mass)
        mean_effect = quadcross(mass,worker_effect)/total
        deviation = sqrt(quadcross(mass,(worker_effect:-mean_effect):^2)/total)
        a = deviation > 0 ? (worker_effect[fit]:-mean_effect):/deviation :
            J(rows(fit),1,0)
        mean_effect = quadcross(mass,firm_effect)/total
        deviation = sqrt(quadcross(mass,(firm_effect:-mean_effect):^2)/total)
        f = deviation > 0 ? (firm_effect[fit]:-mean_effect):/deviation :
            J(rows(fit),1,0)
        a = rowmax((rowmin((a,J(rows(fit),1,3))),J(rows(fit),1,-3)))
        f = rowmax((rowmin((f,J(rows(fit),1,3))),J(rows(fit),1,-3)))
        profile = (J(rows(fit),1,1),a,a:^2,f,f:^2,a:*f)
        permitted = max((1,min((6,floor(rows(fit)/100)))))
        profile = scale:*profile[.,1..permitted]
        inverse = invsym(quadcross(profile,profile))
        coefficient = inverse*quadcross(profile,product[fit])
        fitted = select(profile*coefficient,used)
        shape[2] = sum(diagonal(inverse):!=0)
        shape[|9\8+permitted|] = coefficient'
        info[1] = shape[1]+shape[2]
        info[3] = sum(fitted:<floor_value)
        info[4] = min(fitted)
        variance[selectindex(single)] = rowmax((fitted,J(units,1,floor_value)))
    }
    return(variance)
}

// Repeat-measurement sets of the units of vckss_inf__units(). A collapsed
// mover match is scaled by its own mass, so it is a unit of its own.
// Physical observations (stayer copies under match deletion, every
// observation under observation deletion) share a set with the other
// observations of their worker-firm cell; `unit_firm' may use any code for
// the grounded firm. `unit_mover' marks the units of workers with more than
// one firm.
void vckss_rse__sets(
    real scalar movers,
    real colvector unit_worker,
    real colvector unit_firm,
    real colvector unit_set,
    real colvector unit_mover)
{
    real colvector ix, permutation, first, cells, firm_code
    real matrix panel
    real scalar n, g, firms

    n = rows(unit_worker)
    unit_set = (1::n)
    unit_mover = J(n,1,1)
    if (movers >= n) return
    ix = (movers+1)::n
    firm_code = unit_firm[ix]:-min(unit_firm[ix])
    firms = max(firm_code)+1
    unit_set[ix] = n:+(unit_worker[ix]:-1):*firms:+firm_code
    permutation = ix[order((unit_set[ix],ix),(1,2))]
    panel = panelsetup(unit_set[permutation],1)
    first = permutation[panel[.,1]]
    cells = J(max(unit_worker[ix]),1,0)
    for (g=1; g<=rows(first); g++) {
        cells[unit_worker[first[g]]] = cells[unit_worker[first[g]]]+1
    }
    unit_mover[ix] = cells[unit_worker[ix]]:>1
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

// ---------------------------------------------------------------------------
// Leading directions for repeat-measurement standard errors.
//
// For each of the four reported targets (worker variance, firm variance,
// covariance, variance of the sum) the direction is the eigenvector of
// A_t v = lambda S v of largest absolute eigenvalue, with S the normal matrix
// of the unit design. Along it the estimate splits into
// lambda (b^2 - V-hat[b]) + remainder, with b = w'y and w the unit-space
// weights of the direction, and the q = 1 interval of KSS (Section 6) is
// formed from the 2 x 2 covariance of (b, remainder). The remainder's kernel
// is the target's kernel with its diagonal ratio net of the direction, minus
// the rank-one part. The table that reaches Stata has the layout of the
// native export: one row per target and 28 columns.
// ---------------------------------------------------------------------------
struct vckss_rse_leading {
    real matrix mode, ratio, influence
    real rowvector lambda, second, residual, steps, converged
    real rowvector score, leave_out
    real matrix noise_trace
    // Control term (empty without controls): unit aggregates, precision,
    // W'W, leave-out products and residuals.
    real matrix collapsed, residualized, within, precision, omega
    real colvector product, leave_out_residual
}

// With estimated controls the noise of the control-adjusted unit outcome has
// covariance S = Pi' V Pi + Zc Lambda W'W Lambda Zc', Pi = I - Zt Lambda Zc'
// (the cross-covariance of unit errors and within-unit scores is left to the
// linear part). A draw with that covariance from the draw `noise' with
// covariance V: noise - Zc Lambda (Zt' noise + W' xi), xi standard normal
// for the units with a nonzero row of W (`active'), drawn after `noise'.
real matrix vckss_rse__control_draw(
    real matrix noise,
    real matrix collapsed,
    real matrix residualized,
    real matrix within,
    real matrix precision,
    real colvector active)
{
    real matrix shift

    shift = quadcross(residualized,noise)
    if (rows(active) > 0) {
        shift = shift+quadcross(within[active,.],
            rnormal(rows(active),cols(noise),0,1))
    }
    return(noise-collapsed*(precision*shift))
}

// The bilinear form x'Sy of the estimated S for the matching columns of
// `left' and `right' (kernel images of such draws): sum_a v_a (Pi x)_a
// (Pi y)_a + (Lambda Zc' x)' W'W (Lambda Zc' y), one value per column.
real rowvector vckss_rse__control_form(
    real matrix left,
    real matrix right,
    real colvector variance,
    real matrix collapsed,
    real matrix residualized,
    real matrix precision,
    real matrix omega)
{
    real matrix left_load, right_load

    left_load = precision*quadcross(collapsed,left)
    right_load = precision*quadcross(collapsed,right)
    return(quadcolsum(variance:*(left-residualized*left_load):*
        (right-residualized*right_load))+
        quadcolsum(left_load:*(omega*right_load)))
}

// High-rank linear part of the three targets with the first-order control
// term: the starred influences g* = g + Zt c/2 (c = `load' =
// -2 Lambda Zc' g), their cross term with the within-unit loads W c through
// the leave-out residual, and the loads' square.
real matrix vckss_rse__control_linear(
    real matrix influence,
    real colvector variance,
    real colvector leave_out_residual,
    real matrix residualized,
    real matrix within,
    real matrix load)
{
    real matrix starred, within_load, cross_part

    starred = influence+0.5:*residualized*load
    within_load = within*load
    cross_part = quadcross(starred,leave_out_residual,within_load)
    return(4:*quadcross(starred,variance,starred)+
        2:*(cross_part+cross_part')+quadcross(within_load,within_load))
}

// The inputs of the control term for the q = 1 pieces.
void vckss_rse__lead_controls(
    struct vckss_rse_leading scalar lead,
    real matrix collapsed,
    real matrix residualized,
    real matrix within,
    real matrix precision,
    real matrix omega,
    real colvector product,
    real colvector leave_out_residual)
{
    lead.collapsed = collapsed
    lead.residualized = residualized
    lead.within = within
    lead.precision = precision
    lead.omega = omega
    lead.product = product
    lead.leave_out_residual = leave_out_residual
}

// Remainder ratios, leading scores and their leave-out variances, once the
// directions are known. `ratio' is the n x 3 target diagonal over the maker
// diagonal; the fourth target is worker + firm + 2 covariance.
void vckss_rse__lead_prepare(
    struct vckss_rse_leading scalar lead,
    real colvector y,
    real colvector product,
    real matrix ratio,
    real colvector maker_inverse)
{
    real matrix full

    full = (ratio,ratio[.,1]+ratio[.,2]+2:*ratio[.,3])
    lead.ratio = full-(lead.mode:^2):*maker_inverse:*lead.lambda
    lead.score = quadcross(lead.mode,y)'
    lead.leave_out = quadcross(lead.mode:^2,product)'
    lead.noise_trace = J(2,4,0)
}

// Sum over the repeat-measurement sets of 4 c^2 q^2 (T-2)(T-1)/(T+1), with
// c the mean of `mover_weight' over a set of a mover and the sum of
// `stayer_weight' over T-1 for a stayer.
real scalar vckss_rse__pair(
    real colvector y,
    real colvector unit_set,
    real colvector unit_mover,
    real colvector mover_weight,
    real colvector stayer_weight)
{
    real colvector permutation, members
    real matrix panel
    real scalar n, g, count, sample, weight, total

    n = rows(y)
    total = 0
    permutation = order((unit_set,(1::n)),(1,2))
    panel = panelsetup(unit_set[permutation],1)
    for (g=1; g<=rows(panel); g++) {
        members = permutation[|panel[g,1]\panel[g,2]|]
        count = rows(members)
        if (count < 2) continue
        sample = quadvariance(y[members])
        if (unit_mover[members[1]]) weight = mean(mover_weight[members])
        else weight = sum(stayer_weight[members])/(count-1)
        total = total+4*weight^2*sample^2*(count-2)*(count-1)/(count+1)
    }
    return(total)
}

// The 4 x 28 table. `influence' is the n x 3 realized influence of the
// targets, `diagonal' the n x 3 target diagonals, `quadratic' the 3 x 3
// pseudo-noise estimate of tr(K_t V K_s V) (with controls tr(K_t S K_s S)).
// Returns J(0,0,.) when a
// decomposition does not reproduce its direct remainder.
real matrix vckss_rse__lead_finish(
    struct vckss_rse_leading scalar lead,
    real colvector y,
    real colvector variance,
    real matrix influence,
    real colvector maker_inverse,
    real matrix diagonal,
    real colvector unit_set,
    real colvector unit_mover,
    real matrix quadratic,
    real scalar draws,
    real scalar confidence,
    real scalar seed,
    real scalar identity_tolerance)
{
    real matrix table, full_influence, full_diagonal, covariance
    real rowvector interval, weight
    real colvector w, remainder, load, starred, within_score
    real colvector starred_remainder, within_remainder
    real scalar t, lambda, point, direct, fitted, leading, linear, covar
    real scalar share, extra
    real scalar trace_mean, mcse, pair, remainder_variance, correlation
    real scalar determinant, status, curvature, critical, recentred, total
    real scalar simulations

    simulations = 100000
    table = J(4,28,.)
    full_influence = (influence,influence[.,1]+influence[.,2]+2:*influence[.,3])
    full_diagonal = (diagonal,diagonal[.,1]+diagonal[.,2]+2:*diagonal[.,3])
    weight = (1,1,2)
    for (t=1; t<=4; t++) {
        w = lead.mode[.,t]
        lambda = lead.lambda[t]
        remainder = lead.influence[.,t]
        point = quadcross(y,full_influence[.,t])
        direct = quadcross(y,remainder)
        recentred = point-lambda*(lead.score[t]^2-lead.leave_out[t])
        if (abs(direct-recentred) > identity_tolerance*
            max((1,abs(point),abs(direct),abs(recentred)))) return(J(0,0,.))
        fitted = quadcross(w:^2,variance)
        trace_mean = lead.noise_trace[1,t]/draws
        mcse = draws > 1 ? sqrt(max((0,lead.noise_trace[2,t]/draws-trace_mean^2))/(draws-1)) : 0
        // A stayer's diagonal is nonzero for the worker variance and for the
        // variance of the sum only.
        pair = vckss_rse__pair(y,unit_set,unit_mover,
            (full_diagonal[.,t]-lambda:*w:^2):*maker_inverse,
            ((t == 1 | t == 4) ? full_diagonal[.,t] : J(rows(y),1,0))-lambda:*w:^2)
        if (cols(lead.collapsed) > 0) {
            // First-order term of the estimated control coefficients: the
            // score's weights and the remainder's influence projected by
            // Pi = I - Zt Lambda Zc', their loads on the within-unit
            // products, and the cross terms with the leave-out residual.
            // The decomposition above keeps the weights w^2.
            load = -lead.precision*quadcross(lead.collapsed,w)
            starred = w+lead.residualized*load
            within_score = lead.within*load
            load = -2:*lead.precision*quadcross(lead.collapsed,remainder)
            starred_remainder = remainder+0.5:*lead.residualized*load
            within_remainder = lead.within*load
            extra = 2*quadcross(lead.leave_out_residual,starred:*within_score)+
                quadcross(within_score,within_score)
            fitted = quadcross(starred:^2,variance)+extra
            leading = max((fitted,quadcross(starred:^2,lead.product)+extra))
            linear = 4*quadcross(starred_remainder:^2,variance)+
                4*quadcross(lead.leave_out_residual,
                    starred_remainder:*within_remainder)+
                quadcross(within_remainder,within_remainder)
            covar = 2*quadcross(starred:*starred_remainder,variance)+
                quadcross(lead.leave_out_residual,starred:*within_remainder+
                    2:*starred_remainder:*within_score)+
                quadcross(within_score,within_remainder)
        }
        else {
            leading = max((fitted,lead.leave_out[t]))
            linear = 4*quadcross(remainder:^2,variance)
            covar = 2*quadcross(w:*remainder,variance)
        }
        // Share of the rank-one part in the trace: with controls both belong
        // to the covariance of the control-adjusted noise, as natively.
        share = fitted
        // With controls the trace is the bilinear form of S (lead_trace).
        remainder_variance = linear-2*trace_mean+pair
        correlation = covar/sqrt(leading)/sqrt(remainder_variance)
        determinant = (1-abs(correlation))*(1+abs(correlation))
        status = 0
        if (leading <= 0 | remainder_variance <= 0 | missing(remainder_variance)) status = 1
        else if (missing(determinant) | determinant <= 1e-8) status = 2
        curvature = critical = .
        interval = (.,.)
        if (status == 0) {
            curvature = 2*abs(lambda)*leading/sqrt(remainder_variance*determinant)
            critical = vckss_inf__critical(curvature,confidence,simulations,seed+t)
            covariance = (leading,covar\covar,remainder_variance)
            interval = vckss_inf__am_ci((lead.score[t]\recentred),covariance,
                critical,lambda)
            if (hasmissing(interval)) {
                status = 3
                critical = .
            }
        }
        // tr(K V K V) of the target from the primitive traces.
        if (t <= 3) total = quadratic[t,t]
        else total = weight*quadratic*weight'
        // The conditional operator binds more loosely than the comma.
        table[t,.] = (status,lead.converged[t],lead.steps[t],
            (status == 0 ? simulations : 0),lambda,lead.second[t],
            lead.residual[t],(total > 0 ? (lambda*share)^2/total : .),
            max(w:^2),point,lead.score[t],lead.leave_out[t],fitted,leading,
            recentred,abs(direct-recentred),covar,remainder_variance,linear,
            2*trace_mean,2*mcse,pair,determinant,curvature,critical,interval[1],
            interval[2],(leading > 0 ? lead.score[t]^2/leading : .))
    }
    return(table)
}

// Exact directions from the dense normal-matrix inverse: the eigenvector of
// S^-1/2 A_t S^-1/2 of largest absolute eigenvalue, mapped back.
void vckss_rse__lead_exact(
    struct vckss_rse_leading scalar lead,
    real matrix design,
    real matrix inverse,
    struct vckss_target_matrices scalar targets)
{
    real matrix basis, root_inverse, vectors, target
    real rowvector values, magnitude
    real colvector permutation
    real scalar t

    symeigensystem(inverse,basis,values)
    root_inverse = (basis:*sqrt(colmax((values\J(1,cols(values),0)))))*basis'
    lead.mode = J(rows(design),4,.)
    lead.lambda = lead.second = lead.residual = lead.steps = J(1,4,0)
    lead.converged = J(1,4,1)
    for (t=1; t<=4; t++) {
        target = root_inverse*vckss_rse__lead_target(targets,t)*root_inverse
        symeigensystem((target+target'):/2,vectors,values)
        magnitude = abs(values)
        permutation = order(magnitude',-1)
        lead.lambda[t] = values[permutation[1]]
        lead.second[t] = cols(values) > 1 ? values[permutation[2]] : 0
        lead.mode[.,t] = design*(root_inverse*vectors[.,permutation[1]])
        lead.mode[.,t] = lead.mode[.,t]:/sqrt(quadcross(lead.mode[.,t],lead.mode[.,t]))
    }
}

// Coefficient-space image of one column under target t (1 to 4).
real colvector vckss_rse__lead_apply(
    struct vckss_fe_design scalar base,
    real colvector share,
    real colvector coefficient,
    real scalar t)
{
    real matrix action

    action = vckss_rse__target_action(base,share,coefficient)
    if (t <= 3) return(action[.,t])
    return(action[.,1]+action[.,2]+2:*action[.,3])
}

// Leading direction of target t by a thick-restart Lanczos iteration in the
// S inner product: one linear solve per step. For u = S^-1 g the inner
// product <u, v>_S is g'v; a second pass with the normal matrix applied to
// the new direction restores orthogonality to rounding. `info' receives
// (eigenvalue, second Ritz value, relative residual, solves, converged).
// Returns the S-normalised coefficient direction, or J(0,1,.) if a solve fails.
real colvector vckss_rse__lead_solve(
    struct vckss_fe_design scalar base,
    struct vckss_fe_design scalar units,
    real colvector root,
    real colvector share,
    real scalar t,
    real colvector start,
    real scalar cap,
    real scalar tol,
    real scalar tolerance,
    real scalar maxiter,
    real rowvector info)
{
    struct vckss_solve_result scalar solved
    real matrix basis, projected, vectors, reduced
    real rowvector values
    real colvector image, next, next_image, border, permutation, prediction
    real colvector direction, kept
    real scalar steps, size, lead_index, scale, nrm, residual, invariant
    real scalar converged

    info = J(1,5,.)
    direction = start
    prediction = vckss_rse__unit_predict(units,root,direction)
    nrm = sqrt(quadcross(prediction,prediction))
    if (nrm <= 0 | missing(nrm)) return(J(0,1,.))
    basis = direction:/nrm
    image = vckss_rse__lead_apply(base,share,basis,t)
    projected = quadcross(basis,image)
    steps = 0
    while (1) {
        solved = vckss__fe_solve_matrix(units,image,tolerance,maxiter)
        if (solved.status != "CONVERGED") return(J(0,1,.))
        steps++
        next = solved.coefficient
        next = next-basis*quadcross(basis,image)
        prediction = vckss_rse__unit_predict(units,root,next)
        next = next-basis*quadcross(basis,
            vckss_rse__unit_transpose(units,root,prediction))
        prediction = vckss_rse__unit_predict(units,root,next)
        nrm = sqrt(quadcross(prediction,prediction))
        size = cols(basis)
        symeigensystem((projected+projected'):/2,vectors,values)
        permutation = order(abs(values)',-1)
        lead_index = permutation[1]
        scale = abs(values[lead_index])
        invariant = (scale == 0 | nrm <= 1e-12*scale)
        residual = 0
        if (!invariant) {
            next = next:/nrm
            next_image = vckss_rse__lead_apply(base,share,next,t)
            border = quadcross(basis,next_image)
            residual = abs(quadcross(border,vectors[.,lead_index]))/scale
        }
        converged = invariant | (residual <= tol & steps >= 2)
        if (converged | steps >= cap) {
            direction = basis*vectors[.,lead_index]
            prediction = vckss_rse__unit_predict(units,root,direction)
            nrm = sqrt(quadcross(prediction,prediction))
            if (nrm <= 0 | missing(nrm)) return(J(0,1,.))
            direction = direction:/nrm
            info = (quadcross(direction,vckss_rse__lead_apply(base,share,direction,t)),
                (size > 1 ? values[permutation[2]] : 0),residual,steps,converged)
            return(direction)
        }
        if (size >= 6) {
            // Thick restart: the three Ritz vectors of largest magnitude and
            // the new direction span the next basis.
            kept = permutation[|1\3|]
            reduced = vectors[.,kept]
            basis = basis*reduced
            projected = (diag(values[kept]),quadcross(reduced,border)\
                quadcross(border,reduced),quadcross(next,next_image))
        }
        else projected = (projected,border\border',quadcross(next,next_image))
        basis = (basis,next)
        image = next_image
    }
}

// Remainder kernel images of the columns of `value' for the four targets,
// stacked as four blocks of columns: one fit and four solves per column.
real matrix vckss_rse__lead_images(
    struct vckss_rse_leading scalar lead,
    real matrix value,
    struct vckss_fe_design scalar base,
    struct vckss_fe_design scalar units,
    real colvector root,
    real colvector share,
    real scalar tolerance,
    real scalar maxiter,
    string scalar status)
{
    struct vckss_solve_result scalar solved
    real matrix residual, action, right_hand_side, image
    real scalar columns, t, first, last

    columns = cols(value)
    status = "INFERENCE_SOLVE_FAILED"
    solved = vckss__fe_solve_matrix(units,
        vckss_rse__unit_transpose(units,root,value),tolerance,maxiter)
    if (solved.status != "CONVERGED") return(J(0,0,.))
    residual = value-vckss_rse__unit_predict(units,root,solved.coefficient)
    action = vckss_rse__target_action(base,share,solved.coefficient)
    right_hand_side = (action,action[|1,1\rows(action),columns|]+
        action[|1,columns+1\rows(action),2*columns|]+
        2:*action[|1,2*columns+1\rows(action),3*columns|])
    for (t=1; t<=4; t++) {
        first = (t-1)*columns+1
        last = t*columns
        right_hand_side[|1,first\rows(right_hand_side),last|] =
            right_hand_side[|1,first\rows(right_hand_side),last|]+
            0.5:*vckss_rse__unit_transpose(units,root,lead.ratio[.,t]:*value)
    }
    solved = vckss__fe_solve_matrix(units,right_hand_side,tolerance,maxiter)
    if (solved.status != "CONVERGED") return(J(0,0,.))
    image = vckss_rse__unit_predict(units,root,solved.coefficient)
    for (t=1; t<=4; t++) {
        first = (t-1)*columns+1
        last = t*columns
        image[|1,first\rows(image),last|] = image[|1,first\rows(image),last|]-
            0.5:*lead.ratio[.,t]:*(residual+value)-
            lead.lambda[t]:*lead.mode[.,t]*quadcross(lead.mode[.,t],value)
    }
    status = "CONVERGED"
    return(image)
}

// Add the draws' contributions sum_a v_a (K_r zeta)_a^2 of the four targets;
// with controls the bilinear form of S in place of V.
void vckss_rse__lead_trace(
    struct vckss_rse_leading scalar lead,
    real matrix image,
    real colvector variance,
    real scalar width)
{
    real rowvector value
    real matrix block
    real scalar t

    for (t=1; t<=4; t++) {
        block = image[|1,(t-1)*width+1\rows(image),t*width|]
        if (cols(lead.collapsed) > 0) {
            value = vckss_rse__control_form(block,block,variance,
                lead.collapsed,lead.residualized,lead.precision,lead.omega)
        }
        else value = quadcross(variance,block:^2)
        lead.noise_trace[1,t] = lead.noise_trace[1,t]+sum(value)
        lead.noise_trace[2,t] = lead.noise_trace[2,t]+sum(value:^2)
    }
}

// Target matrix t (1 to 4) of the exact route.
real matrix vckss_rse__lead_target(
    struct vckss_target_matrices scalar targets,
    real scalar t)
{
    if (t == 1) return(targets.worker)
    if (t == 2) return(targets.firm)
    if (t == 3) return(targets.covariance)
    return(targets.worker+targets.firm+2:*targets.covariance)
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
    real rowvector info, shape
    real colvector unit_worker, worker_effect, firm_effect
    real colvector unit_set, unit_mover, unit_firm
    real matrix pair, lead_image, lead_table
    real matrix omega
    real colvector active
    struct vckss_rse_leading scalar lead
    real scalar leading, control_term
    real scalar n, parameters, worker_levels, firm_levels, control_count
    real scalar centered, j, k, z, point_identity, begin, finish, batch
    real scalar pairs, width, effective, truncation, repeat

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
        omega = quadcross(within,within)
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

    // Repeat-measurement standard errors share this pass and differ in the
    // unit variances and in how the covariance is assembled from them.
    repeat = st_global("VCKSS_SE_MODEL") == "repeat_measurement"
    info = J(1,4,.)
    shape = J(1,14,0)
    if (repeat) {
        unit_worker = round((design[|1,1\n,worker_levels|]*
            (1::worker_levels)):/root)
        worker_effect = (design[|1,1\n,worker_levels|]*
            beta[|1\worker_levels|]):/root
        if (firm_levels > 1) {
            firm_effect = (design[|1,worker_levels+1\n,parameters|]*
                beta[|worker_levels+1\parameters|]):/root
        }
        else firm_effect = J(n,1,0)
        if (firm_levels > 1) {
            unit_firm = round((design[|1,worker_levels+1\n,parameters|]*
                (1::(firm_levels-1))):/root)
        }
        else unit_firm = J(n,1,0)
        vckss_rse__sets(units.movers,unit_worker,unit_firm,unit_set,unit_mover)
        variance = vckss_rse__repeat(product,y,residual,leverage,
            maker_inverse,units.mass,unit_set,unit_mover,worker_effect,
            firm_effect,ratio:/maker_inverse,info,shape,pair)
    }
    else variance = vckss_rse__working(product,residual,leverage,units.mass,
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

    // Repeat-measurement standard errors include the first-order term of the
    // estimated control coefficients; without controls the path is unchanged.
    control_term = repeat & control_count > 0
    if (control_term) active = selectindex(rowsum(within:!=0):>0)

    // Leading directions and the realized influence of every remainder.
    leading = repeat & st_global("VCKSS_SE_LEADING") != ""
    if (leading) {
        vckss_rse__lead_exact(lead,design,inverse,targets)
        vckss_rse__lead_prepare(lead,y,product,ratio,maker_inverse)
        if (control_term) {
            vckss_rse__lead_controls(lead,collapsed,collapsed_residualized,
                within,precision,omega,product,residual:*maker_inverse)
        }
        lead.influence = J(n,4,.)
        for (j=1; j<=4; j++) {
            lead.influence[.,j] = vckss_rse__kernel(y,design,inverse,
                vckss_rse__lead_target(targets,j),lead.ratio[.,j])-
                (lead.lambda[j]*lead.score[j]):*lead.mode[.,j]
        }
        if (centered) lead.influence = vckss_rse__center(lead.influence,root)
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
        // With controls the draw has the covariance of the adjusted outcome,
        // and is centred after the adjustment, as the outcome is.
        if (control_term) {
            noise = vckss_rse__control_draw(noise,collapsed,
                collapsed_residualized,within,precision,active)
        }
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
        if (leading) {
            lead_image = J(n,4*width,.)
            for (j=1; j<=4; j++) {
                lead_image[|1,(j-1)*width+1\n,j*width|] = vckss_rse__kernel(
                    noise,design,inverse,vckss_rse__lead_target(targets,j),
                    lead.ratio[.,j])-lead.lambda[j]:*lead.mode[.,j]*
                    quadcross(lead.mode[.,j],noise)
            }
            if (centered) lead_image = vckss_rse__center(lead_image,root)
            vckss_rse__lead_trace(lead,lead_image,variance,width)
        }
        for (j=1; j<=3; j++) {
            for (k=1; k<=3; k++) {
                // With controls the trace is the bilinear form of S.
                if (control_term) {
                    quadratic[j,k] = quadratic[j,k]+sum(vckss_rse__control_form(
                        image[|1,(j-1)*width+1\n,j*width|],
                        image[|1,(k-1)*width+1\n,k*width|],variance,collapsed,
                        collapsed_residualized,precision,omega))
                }
                else {
                    quadratic[j,k] = quadratic[j,k]+sum(variance:*
                        image[|1,(j-1)*width+1\n,j*width|]:*
                        image[|1,(k-1)*width+1\n,k*width|])
                }
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
                if (control_term) {
                    fourth = fourth+vckss_rse__control_form(previous,
                        total_image,variance,collapsed,collapsed_residualized,
                        precision,omega)':^2
                }
                else {
                    fourth = fourth+
                        diagonal(quadcross(previous,variance,total_image)):^2
                }
                pairs = pairs+1
            }
            previous = total_image
        }
    }
    quadratic = quadratic:/draws
    calibration = calibration:/draws
    if (repeat) {
        // The high-rank form with the unit variances, plus the pair term of
        // the repeat-measurement sets. Nothing is truncated: a nonpositive
        // diagonal is reported as such below.
        if (control_term) {
            signal = vckss_rse__control_linear(influence,variance,
                residual:*maker_inverse,collapsed_residualized,within,load)-
                4:*quadratic+pair
        }
        else signal = 4:*quadcross(influence,variance,influence)-4:*quadratic+pair
        signal = (signal+signal'):/2
        truncation = 0
        V_primitive = signal+2:*quadratic
        score = 2:*influence:*sqrt(variance)
    }
    else {
        signal = quadcross(score,score)-4:*calibration
        signal = (signal+signal'):/2
        eigenvectors = eigenvalues = .
        symeigensystem(signal,eigenvectors,eigenvalues)
        truncation = max((0,-min(eigenvalues)))
        eigenvalues = eigenvalues:*(eigenvalues:>0)
        V_primitive = eigenvectors*diag(eigenvalues)*eigenvectors'+2:*quadratic
    }
    V_primitive = (V_primitive+V_primitive'):/2
    // With leading directions a nonpositive high-rank variance is the case
    // the q = 1 interval exists for: the component then has no standard
    // error, and no joint covariance is posted.
    if (hasmissing(V_primitive) | (min(diagonal(V_primitive)) <= 0 & !leading)) {
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
    if (leading) {
        lead_table = vckss_rse__lead_finish(lead,y,variance,influence,
            maker_inverse,ratio:/maker_inverse,unit_set,unit_mover,quadratic,
            draws,confidence_level/100,seed,1e-8)
        if (rows(lead_table) == 0) {
            st_local(status_local,"TARGET_IDENTITY_FAILED")
            st_local(message_local,
                "the q=1 decomposition does not reproduce its remainder")
            return
        }
        st_matrix("__fevc_se_leading",lead_table)
    }
    st_matrix(V_primitive_name,V_primitive)
    st_matrix(V_name,V)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,(draws,seed,.,confidence_level,truncation,
        min(product),max(product),units.movers,n-units.movers,0,info[3],.,.))
    st_matrix(variance_diagnostics_name,(info[1],.,.,.,info[2],info[3],n,info[4]))
    st_matrix(spectrum_name,spectrum)
    st_local("inference_joint_available",min(diagonal(V)) > 0 ? "1" : "0")
    // Both models include the first-order term of estimated controls.
    st_local("inference_nuisance_omitted",control_count > 0 ? "0" : "1")
    st_local("inference_repeat_shape",
        repeat ? invtokens(strofreal(shape,"%21.0g")) : "")
    st_local(status_local,"CONVERGED")
    st_local(message_local,repeat ?
        "exact repeat-measurement standard errors converged" :
        "exact leave-out residual standard errors converged")
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
    real colvector unit_set, unit_mover, lead_vector
    real matrix pair, lead_image, lead_table
    struct vckss_rse_leading scalar lead
    real rowvector lead_info
    real scalar leading
    string scalar lead_status
    real colvector ix, permutation, members, gamma
    real matrix controls, residualized, precision, collapsed
    real matrix collapsed_residualized, within, load, noise, image, previous
    real matrix quadratic, calibration, signal, V_primitive, transform, V
    real matrix results, spectrum, eigenvectors, ratio, influence, score
    real matrix posted_corrected, total_image, panel, projected, direction
    real matrix full, pseudo_residual, target_diagonal, outcome_fit
    real matrix control_fit, kernel_residual, moments, probe, square
    real matrix complement, effects
    real matrix omega
    real colvector active
    real rowvector info, shape
    real colvector worker_effect, firm_effect
    real scalar n, workers, firms, control_count, centered, j, k, z
    real scalar begin, finish, batch, pairs, width, effective, truncation
    real scalar repeat, control_term
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
    effects = solved.coefficient
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
        effects = effects-solved.coefficient*gamma
        working_y = y-controls*gamma
        residual = residual-residualized*gamma
        collapsed = vckss_rse__aggregate(controls,working_y,frequency,
            deletion_id,stayer,0)
        collapsed_residualized = vckss_rse__aggregate(residualized,
            working_y,frequency,deletion_id,stayer,0)
        within = vckss_rse__aggregate(controls,working_y,frequency,
            deletion_id,stayer,1)
        omega = quadcross(within,within)
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

    repeat = st_global("VCKSS_SE_MODEL") == "repeat_measurement"
    info = J(1,4,.)
    shape = J(1,14,0)
    if (repeat) {
        // Effects of the offset outcome; the last firm is the grounded one.
        worker_effect = effects[unit_worker]
        firm_effect = J(n,1,0)
        ix = selectindex(unit_firm:<firms)
        if (rows(ix) > 0) firm_effect[ix] = effects[workers:+unit_firm[ix]]
        vckss_rse__sets(movers,unit_worker,unit_firm,unit_set,unit_mover)
        variance = vckss_rse__repeat(product,y,residual,leverage,
            maker_inverse,mass,unit_set,unit_mover,worker_effect,firm_effect,
            target_diagonal,info,shape,pair)
    }
    else variance = vckss_rse__working(product,residual,leverage,mass,movers,info)
    if (info[2] <= 0 | missing(info[2]) | hasmissing(variance) | info[3] == n) {
        st_local(status_local,"INFERENCE_VARIANCE_INVALID")
        st_local(message_local,
            "the working variances for the pseudo-noise are not positive")
        return
    }
    // Leading directions, each search started from the previous direction,
    // and the realized influence of every remainder.
    leading = repeat & st_global("VCKSS_SE_LEADING") != ""
    if (leading) {
        lead.mode = J(n,4,.)
        lead.lambda = lead.second = lead.residual = lead.steps =
            lead.converged = J(1,4,.)
        // A fixed start that depends on nothing but the coefficient index.
        lead_vector = cos((1::(workers+firms-1)):*0.7390851332)+
            sin((1::(workers+firms-1)):^2:*0.3183098862)
        for (j=1; j<=4; j++) {
            lead_vector = vckss_rse__lead_solve(base,units,root,share,j,
                lead_vector,strtoreal(st_global("VCKSS_SE_LEADING_CAP")),1e-3,
                tolerance,maxiter,lead_info)
            if (rows(lead_vector) == 0) return
            lead.lambda[j] = lead_info[1]
            lead.second[j] = lead_info[2]
            lead.residual[j] = lead_info[3]
            lead.steps[j] = lead_info[4]
            lead.converged[j] = lead_info[5]
            lead.mode[.,j] = vckss_rse__unit_predict(units,root,lead_vector)
        }
        vckss_rse__lead_prepare(lead,y,product,ratio,maker_inverse)
        if (control_count > 0) {
            vckss_rse__lead_controls(lead,collapsed,collapsed_residualized,
                within,precision,omega,product,residual:*maker_inverse)
        }
        lead.influence = vckss_rse__lead_images(lead,y,base,units,root,share,
            tolerance,maxiter,lead_status)
        if (lead_status != "CONVERGED") return
        if (centered) lead.influence = vckss_rse__center(lead.influence,root)
    }
    scaled_residual = residual:*sqrt(maker_inverse)
    score = 2:*influence:*scaled_residual
    if (control_count > 0) {
        load = precision*(-2:*quadcross(collapsed,influence))
        score = (2:*influence+collapsed_residualized*load):*scaled_residual+
            within*load
    }
    // Repeat-measurement standard errors include the first-order term of the
    // estimated control coefficients; without controls the path is unchanged.
    control_term = repeat & control_count > 0
    if (control_term) active = selectindex(rowsum(within:!=0):>0)

    quadratic = calibration = J(3,3,0)
    fourth = J(4,1,0)
    pairs = 0
    previous = J(0,0,.)
    batch = min((16,draws))
    for (begin=1; begin<=draws; begin=begin+batch) {
        finish = min((draws,begin+batch-1))
        width = finish-begin+1
        noise = sqrt(variance):*rnormal(n,width,0,1)
        // With controls the draw has the covariance of the adjusted outcome,
        // and is centred after the adjustment, as the outcome is.
        if (control_term) {
            noise = vckss_rse__control_draw(noise,collapsed,
                collapsed_residualized,within,precision,active)
        }
        if (centered) noise = vckss_rse__center(noise,root)
        image = vckss_rse__solve_kernel(noise,base,units,root,share,ratio,
            tolerance,maxiter,kernel_residual,kernel_status)
        if (kernel_status != "CONVERGED") return
        if (centered) image = vckss_rse__center(image,root)
        if (leading) {
            lead_image = vckss_rse__lead_images(lead,noise,base,units,root,
                share,tolerance,maxiter,lead_status)
            if (lead_status != "CONVERGED") return
            if (centered) lead_image = vckss_rse__center(lead_image,root)
            vckss_rse__lead_trace(lead,lead_image,variance,width)
        }
        pseudo_residual = kernel_residual:^2:*maker_inverse
        for (j=1; j<=3; j++) {
            for (k=1; k<=3; k++) {
                // With controls the trace is the bilinear form of S.
                if (control_term) {
                    quadratic[j,k] = quadratic[j,k]+sum(vckss_rse__control_form(
                        image[|1,(j-1)*width+1\n,j*width|],
                        image[|1,(k-1)*width+1\n,k*width|],variance,collapsed,
                        collapsed_residualized,precision,omega))
                }
                else {
                    quadratic[j,k] = quadratic[j,k]+sum(variance:*
                        image[|1,(j-1)*width+1\n,j*width|]:*
                        image[|1,(k-1)*width+1\n,k*width|])
                }
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
                if (control_term) {
                    fourth = fourth+vckss_rse__control_form(previous,
                        total_image,variance,collapsed,collapsed_residualized,
                        precision,omega)':^2
                }
                else {
                    fourth = fourth+
                        diagonal(quadcross(previous,variance,total_image)):^2
                }
                pairs = pairs+1
            }
            previous = total_image
        }
    }
    quadratic = quadratic:/draws
    calibration = calibration:/draws
    if (repeat) {
        // The high-rank form with the unit variances, plus the pair term of
        // the repeat-measurement sets. Nothing is truncated: a nonpositive
        // diagonal is reported as such below.
        if (control_term) {
            signal = vckss_rse__control_linear(influence,variance,
                residual:*maker_inverse,collapsed_residualized,within,load)-
                4:*quadratic+pair
        }
        else signal = 4:*quadcross(influence,variance,influence)-4:*quadratic+pair
        signal = (signal+signal'):/2
        truncation = 0
        V_primitive = signal+2:*quadratic
        score = 2:*influence:*sqrt(variance)
    }
    else {
        signal = quadcross(score,score)-4:*calibration
        signal = (signal+signal'):/2
        eigenvectors = eigenvalues = .
        symeigensystem(signal,eigenvectors,eigenvalues)
        truncation = max((0,-min(eigenvalues)))
        eigenvalues = eigenvalues:*(eigenvalues:>0)
        V_primitive = eigenvectors*diag(eigenvalues)*eigenvectors'+2:*quadratic
    }
    V_primitive = (V_primitive+V_primitive'):/2
    // With leading directions a nonpositive high-rank variance is the case
    // the q = 1 interval exists for: the component then has no standard
    // error, and no joint covariance is posted.
    if (hasmissing(V_primitive) | (min(diagonal(V_primitive)) <= 0 & !leading)) {
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
    if (leading) {
        lead_table = vckss_rse__lead_finish(lead,y,variance,influence[.,1..3],
            maker_inverse,target_diagonal,unit_set,unit_mover,quadratic,
            draws,confidence_level/100,seed,1e-6)
        if (rows(lead_table) == 0) {
            st_local(status_local,"TARGET_IDENTITY_FAILED")
            st_local(message_local,
                "the q=1 decomposition does not reproduce its remainder")
            return
        }
        st_matrix("__fevc_se_leading",lead_table)
    }
    st_matrix(V_primitive_name,V_primitive)
    st_matrix(V_name,V)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,(draws,seed,probes,confidence_level,truncation,
        min(product),max(product),movers,n-movers,clipped,info[3],
        max(abs(quadcross(y,influence[.,1..3])-posted_corrected[1,1..3])),.))
    st_matrix(variance_diagnostics_name,(info[1],.,.,.,info[2],info[3],n,info[4]))
    st_matrix(spectrum_name,spectrum)
    st_local("inference_joint_available",min(diagonal(V)) > 0 ? "1" : "0")
    // Both models include the first-order term of estimated controls.
    st_local("inference_nuisance_omitted",control_count > 0 ? "0" : "1")
    st_local("inference_repeat_shape",
        repeat ? invtokens(strofreal(shape,"%21.0g")) : "")
    st_local(status_local,"CONVERGED")
    st_local(message_local,repeat ?
        "matrix-free repeat-measurement standard errors converged" :
        "matrix-free leave-out residual standard errors converged")
}

// Defined last: present only when the whole file compiled.
real scalar vckss_rse__loaded()
{
    return(1)
}

end
