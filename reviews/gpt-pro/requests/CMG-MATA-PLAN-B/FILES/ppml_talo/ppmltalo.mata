*! ppmltalo Mata loader 0.0.4-dev 13aug2026

version 18.0

mata:
mata set matastrict on

string scalar ppmltalo__version()
{
    return("0.0.4-dev")
}

real scalar ppmltalo__api_level()
{
    return(9)
}

struct ppmltalo_dense_result
{
    real scalar    plugin
    real scalar    correction
    real scalar    talo
    real scalar    max_leverage
    real colvector leverage
    real colvector deleted_mu
    real colvector omega
    real matrix    G
}

struct ppmltalo_dense_lo_result
{
    real colvector deleted_mu
    real matrix    beta
    real colvector score_relres
    real colvector iterations
    string scalar  status
}

struct ppmltalo_dense_inverse_result
{
    real matrix inverse
    real scalar relres
    real scalar rcond
    string scalar status
}

void ppmltalo__dense_validate(
    real colvector y,
    real matrix X,
    real colvector beta,
    real matrix Q,
    real colvector eta,
    real colvector mu)
{
    real scalar n, p

    n = rows(X)
    p = cols(X)
    if (n == 0 | p == 0) _error(3300, "empty dense TALO design")
    if (rows(y) != n | cols(y) != 1) _error(3200, "y must be n by 1")
    if (rows(beta) != p | cols(beta) != 1) _error(3200, "beta must be p by 1")
    if (rows(Q) != p | cols(Q) != p) _error(3200, "Q must be p by p")
    if (rows(eta) != n | cols(eta) != 1) _error(3200, "eta must be n by 1")
    if (rows(mu) != n | cols(mu) != 1) _error(3200, "mu must be n by 1")
    if (hasmissing(y) | hasmissing(X) | hasmissing(beta) |
        hasmissing(Q) | hasmissing(eta) | hasmissing(mu)) {
        _error(3351, "dense TALO inputs must be finite")
    }
    if (min(y) < 0) _error(3498, "outcomes must be nonnegative")
    if (min(mu) <= 0) _error(3498, "fitted means must be positive")
    if (max(abs(eta)) > 700 | mreldif(mu,exp(eta)) > 1e-10) {
        _error(3498, "fitted means and indexes are incoherent")
    }
    if (mreldif(Q, Q') > 1e-12) _error(3498, "Q must be symmetric")
}

real matrix ppmltalo__dense_information(real matrix X, real colvector mu)
{
    return(X' * (mu :* X))
}

struct ppmltalo_dense_inverse_result scalar ppmltalo__dense_inverse(
    real matrix X,
    real colvector mu)
{
    struct ppmltalo_dense_inverse_result scalar out
    real matrix H, scaled, scaled_inverse
    real colvector inverse_scale, eigen
    real scalar dimension

    out.status = "INVALID_INPUT"
    out.inverse = J(0,0,.)
    out.relres = .
    out.rcond = .
    H = ppmltalo__dense_information(X, mu)
    dimension = rows(H)
    if (dimension == 0 | rows(H) != cols(H) | hasmissing(H) |
        min(diagonal(H)) <= 0) {
        out.status = "SINGULAR"
        return(out)
    }
    inverse_scale = 1:/sqrt(diagonal(H))
    scaled = (inverse_scale*inverse_scale'):*H
    eigen = Re(eigenvalues(0.5:*(scaled+scaled')))
    if (hasmissing(eigen) | max(eigen) <= 0 | min(eigen) <= 0) {
        out.status = "SINGULAR"
        out.rcond = 0
        return(out)
    }
    out.rcond = min(eigen)/max(eigen)
    if (rank(scaled) != dimension) {
        out.status = "SINGULAR"
        return(out)
    }
    scaled_inverse = invsym(scaled)
    out.relres = ppmltalo__norm2(vec(scaled*scaled_inverse-I(dimension))) /
        sqrt(dimension)
    if (hasmissing(scaled_inverse) | hasmissing(out.relres) |
        out.relres > 1e-9) {
        out.status = "INVERSE_FAILED"
        return(out)
    }
    out.inverse = (inverse_scale*inverse_scale'):*scaled_inverse
    if (hasmissing(out.inverse)) {
        out.status = "INVERSE_FAILED"
        return(out)
    }
    out.status = "CONVERGED"
    return(out)
}

real matrix ppmltalo__dense_invinfo(real matrix X, real colvector mu)
{
    struct ppmltalo_dense_inverse_result scalar result

    result = ppmltalo__dense_inverse(X,mu)
    if (result.status == "SINGULAR") {
        _error(3498,"information is singular on supplied coordinates")
    }
    if (result.status != "CONVERGED") {
        _error(3498,"scaled information inversion failed its residual gate")
    }
    return(result.inverse)
}

real matrix ppmltalo__dense_half_hessian(
    real matrix X,
    real colvector mu,
    real colvector beta,
    real matrix Q)
{
    real matrix A, C, G
    real colvector sQ

    A = ppmltalo__dense_invinfo(X, mu)
    C = X * A * X'
    sQ = X * A * Q * beta
    G = X * A * Q * A * X' - C * diag(mu :* sQ) * C
    return(0.5 :* (G + G'))
}

real colvector ppmltalo__dense_G_action(
    real colvector z,
    real matrix X,
    real colvector mu,
    real colvector beta,
    real matrix Q)
{
    real matrix A
    real colvector Cz, sQ, first

    if (rows(z) != rows(X) | cols(z) != 1) _error(3200, "z must be n by 1")
    A = ppmltalo__dense_invinfo(X, mu)
    Cz = X * A * X' * z
    sQ = X * A * Q * beta
    first = X * A * Q * A * X' * z
    return(first - X * A * X' * ((mu :* sQ) :* Cz))
}

struct ppmltalo_dense_result scalar ppmltalo__dense_row(
    real colvector y,
    real matrix X,
    real colvector beta,
    real matrix Q,
    real colvector eta,
    real colvector mu)
{
    struct ppmltalo_dense_result scalar out
    real matrix A, C
    real colvector shift, moment

    ppmltalo__dense_validate(y, X, beta, Q, eta, mu)
    A = ppmltalo__dense_invinfo(X, mu)
    C = X * A * X'
    out.leverage = mu :* diagonal(C)
    if (min(out.leverage) < -1e-12 | max(out.leverage) >= 1) {
        _error(3498, "row leverage lies outside [0,1)")
    }
    shift = -(out.leverage :/ (1 :- out.leverage)) :* ((y :- mu) :/ mu)
    out.deleted_mu = exp(eta + shift)
    if (hasmissing(out.deleted_mu) | min(out.deleted_mu) <= 0) {
        _error(3498, "analytic deleted prediction is nonfinite")
    }
    out.G = ppmltalo__dense_half_hessian(X, mu, beta, Q)
    out.omega = diagonal(out.G)
    moment = y :* (y :- out.deleted_mu)
    out.plugin = (beta' * Q * beta)[1,1]
    out.correction = (out.omega' * moment)[1,1]
    out.talo = out.plugin - out.correction
    out.max_leverage = max(out.leverage)
    if (hasmissing(out.leverage) | hasmissing(out.omega) |
        hasmissing(out.plugin) | hasmissing(out.correction) |
        hasmissing(out.talo) | hasmissing(out.max_leverage)) {
        _error(3498,"dense TALO output is nonfinite")
    }
    return(out)
}

struct ppmltalo_dense_result scalar ppmltalo__dense_cluster(
    real colvector y,
    real matrix X,
    real colvector beta,
    real matrix Q,
    real colvector eta,
    real colvector mu,
    real colvector cluster)
{
    struct ppmltalo_dense_result scalar out
    real matrix A, Pc, Xc, Gcc
    real colvector ids, idx, muc, rc, ac, qc, shift, ec, yc
    real scalar j, m, eigmax

    ppmltalo__dense_validate(y, X, beta, Q, eta, mu)
    if (rows(cluster) != rows(X) | cols(cluster) != 1 | hasmissing(cluster)) {
        _error(3200, "cluster must be a finite n by 1 vector")
    }
    A = ppmltalo__dense_invinfo(X, mu)
    out.G = ppmltalo__dense_half_hessian(X, mu, beta, Q)
    out.omega = diagonal(out.G)
    out.deleted_mu = J(rows(X), 1, .)
    out.leverage = mu :* diagonal(X * A * X')
    out.max_leverage = 0
    out.correction = 0
    ids = uniqrows(sort(cluster, 1))
    for (j=1; j<=rows(ids); j++) {
        idx = selectindex(cluster :== ids[j])
        Xc = X[idx,.]
        muc = mu[idx]
        rc = y[idx] :- muc
        m = rows(idx)
        Pc = diag(sqrt(muc)) * Xc * A * Xc' * diag(sqrt(muc))
        Pc = 0.5 :* (Pc + Pc')
        eigmax = max(Re(eigenvalues(Pc)))
        if (eigmax >= 1 - 1e-12) {
            _error(3498, "cluster deletion is rank unstable")
        }
        out.max_leverage = max((out.max_leverage, eigmax))
        ac = rc :/ sqrt(muc)
        qc = invsym(I(m) - Pc) * ac
        shift = -(qc :- ac) :/ sqrt(muc)
        out.deleted_mu[idx] = exp(eta[idx] + shift)
        ec = y[idx] :- out.deleted_mu[idx]
        yc = y[idx]
        Gcc = out.G[idx,idx]
        out.correction = out.correction + (yc' * Gcc * ec)[1,1]
    }
    if (hasmissing(out.deleted_mu) | min(out.deleted_mu) <= 0) {
        _error(3498, "cluster analytic deleted prediction is nonfinite")
    }
    out.plugin = (beta' * Q * beta)[1,1]
    out.talo = out.plugin - out.correction
    if (hasmissing(out.leverage) | hasmissing(out.omega) |
        hasmissing(out.plugin) | hasmissing(out.correction) |
        hasmissing(out.talo) | hasmissing(out.max_leverage)) {
        _error(3498,"dense cluster TALO output is nonfinite")
    }
    return(out)
}

struct ppmltalo_dense_result scalar ppmltalo__dense_match_weighted(
    real colvector y,
    real matrix X,
    real colvector beta,
    real matrix Q,
    real colvector eta,
    real colvector mu,
    real colvector frequency,
    real colvector match)
{
    struct ppmltalo_dense_result scalar out
    struct ppmltalo_dense_inverse_result scalar information_inverse
    real matrix H, A, C, Pc, Xc, Gcc
    real colvector ids, idx, wc, ac, qc, shift, ec, yc, sQ
    real scalar j, m, eigmax

    ppmltalo__dense_validate(y, X, beta, Q, eta, mu)
    if (rows(frequency) != rows(X) | cols(frequency) != 1 |
        hasmissing(frequency) | min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0) {
        _error(3200, "frequency must be a positive integer n by 1 vector")
    }
    if (rows(match) != rows(X) | cols(match) != 1 | hasmissing(match)) {
        _error(3200, "match must be a finite n by 1 vector")
    }
    H = X' * ((frequency:*mu) :* X)
    information_inverse = ppmltalo__dense_inverse(X,frequency:*mu)
    if (information_inverse.status == "SINGULAR") {
        _error(3498, "weighted information is singular on supplied coordinates")
    }
    if (information_inverse.status != "CONVERGED") {
        _error(3498,"weighted information inversion failed its residual gate")
    }
    A = information_inverse.inverse
    C = X * A * X'
    sQ = X * A * Q * beta
    out.G = X*A*Q*A*X' - C*diag(frequency:*mu:*sQ)*C
    out.G = 0.5 :* (out.G + out.G')
    out.omega = diagonal(out.G)
    out.deleted_mu = J(rows(X),1,.)
    // This is the collapsed-pattern projection diagonal f_j mu_j C_jj,
    // not the leverage of one physical replica (mu_j C_jj).
    out.leverage = frequency:*mu:*diagonal(C)
    out.max_leverage = 0
    out.correction = 0
    ids = uniqrows(sort(match,1))
    for (j=1; j<=rows(ids); j++) {
        idx = selectindex(match :== ids[j])
        Xc = X[idx,.]
        wc = frequency[idx] :* mu[idx]
        m = rows(idx)
        Pc = diag(sqrt(wc)) * Xc * A * Xc' * diag(sqrt(wc))
        Pc = 0.5 :* (Pc + Pc')
        eigmax = max(Re(eigenvalues(Pc)))
        if (eigmax >= 1-1e-12) {
            _error(3498, "weighted match deletion is rank unstable")
        }
        out.max_leverage = max((out.max_leverage,eigmax))
        ac = sqrt(frequency[idx]:/mu[idx]) :* (y[idx]:-mu[idx])
        qc = invsym(I(m)-Pc) * ac
        shift = -(qc:-ac) :/ sqrt(wc)
        out.deleted_mu[idx] = exp(eta[idx]+shift)
        if (hasmissing(out.deleted_mu[idx]) | min(out.deleted_mu[idx]) <= 0) {
            _error(3498, "weighted match analytic prediction is nonfinite")
        }
        yc = frequency[idx] :* y[idx]
        ec = frequency[idx] :* (y[idx]:-out.deleted_mu[idx])
        Gcc = out.G[idx,idx]
        out.correction = out.correction + (yc'*Gcc*ec)[1,1]
    }
    out.plugin = (beta'*Q*beta)[1,1]
    out.talo = out.plugin - out.correction
    if (hasmissing(out.leverage) | hasmissing(out.omega) |
        hasmissing(out.plugin) | hasmissing(out.correction) |
        hasmissing(out.talo) | hasmissing(out.max_leverage)) {
        _error(3498,"weighted dense match TALO output is nonfinite")
    }
    return(out)
}

struct ppmltalo_dense_lo_result scalar ppmltalo__dense_match_lo_w(
    real colvector y,
    real matrix X,
    real colvector start,
    real colvector offset,
    real colvector frequency,
    real colvector match,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_dense_lo_result scalar out
    real colvector ids, idx, keep, beta, eta, mu, score, step
    real colvector candidate_beta, candidate_eta, candidate_mu
    real matrix Xkeep, H
    real scalar n, p, j, iteration, line_iteration, relres
    real scalar objective, candidate_objective, step_length, gain, accepted
    real scalar converged
    real scalar step_norm, information_scale

    n = rows(X)
    p = cols(X)
    if (n == 0 | p == 0 | rows(y) != n | cols(y) != 1 |
        hasmissing(y) | min(y) < 0 | hasmissing(X)) {
        _error(3200, "invalid weighted exact-match PPML data")
    }
    if (rows(start) != p | cols(start) != 1 | hasmissing(start) |
        rows(offset) != n | cols(offset) != 1 | hasmissing(offset)) {
        _error(3200, "invalid weighted exact-match start or offset")
    }
    if (rows(frequency) != n | cols(frequency) != 1 |
        hasmissing(frequency) | min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0) {
        _error(3200, "frequency must be a positive integer n by 1 vector")
    }
    if (rows(match) != n | cols(match) != 1 | hasmissing(match)) {
        _error(3200, "match must be a finite n by 1 vector")
    }
    if (tolerance <= 0 | tolerance >= 1 | maxiter < 1 |
        maxiter != floor(maxiter)) {
        _error(3498, "invalid weighted exact-match solver settings")
    }
    ids = uniqrows(sort(match,1))
    out.deleted_mu = J(n,1,.)
    out.beta = J(n,p,.)
    out.score_relres = J(n,1,.)
    out.iterations = J(n,1,.)
    for (j=1; j<=rows(ids); j++) {
        idx = selectindex(match :== ids[j])
        keep = selectindex(match :!= ids[j])
        if (rows(keep) < p) {
            _error(3498, "weighted exact match deletion leaves too few rows")
        }
        Xkeep = X[keep,.]
        if (rank(Xkeep) != p) {
            _error(3498, "weighted exact match deletion loses design rank")
        }
        beta = start
        relres = .
        converged = 0
        for (iteration=1; iteration<=maxiter; iteration++) {
            eta = offset[keep] + Xkeep*beta
            if (hasmissing(eta) | max(abs(eta)) > 700) break
            mu = exp(eta)
            score = Xkeep' * (frequency[keep] :* (y[keep]:-mu))
            relres = ppmltalo__norm2(score) /
                (1 + ppmltalo__norm2(Xkeep'*(frequency[keep]:*y[keep])) +
                 ppmltalo__norm2(Xkeep'*(frequency[keep]:*mu)))
            H = Xkeep' * ((frequency[keep]:*mu) :* Xkeep)
            if (rank(H) != p) break
            information_scale = max(abs(H))
            if (information_scale <= 0 | missing(information_scale)) break
            step = invsym(H:/information_scale)*(score:/information_scale)
            step_norm = max(abs(step))
            if (relres <= tolerance & step_norm <= sqrt(tolerance)) {
                converged = 1
                break
            }
            objective = sum(frequency[keep] :* (y[keep]:*eta-mu))
            step_length = 1
            accepted = 0
            for (line_iteration=1; line_iteration<=40; line_iteration++) {
                candidate_beta = beta + step_length:*step
                candidate_eta = offset[keep] + Xkeep*candidate_beta
                if (!hasmissing(candidate_eta) & max(abs(candidate_eta)) <= 700) {
                    candidate_mu = exp(candidate_eta)
                    candidate_objective = sum(frequency[keep] :*
                        (y[keep]:*candidate_eta-candidate_mu))
                    gain = candidate_objective-objective
                    if (gain >= -1e-12*(1+abs(objective))) {
                        beta = candidate_beta
                        accepted = 1
                        break
                    }
                }
                step_length = step_length/2
            }
            if (!accepted) break
        }
        if (!converged) {
            _error(3498, "weighted exact match-deleted PPML did not converge")
        }
        eta = offset[idx] + X[idx,.]*beta
        if (hasmissing(eta) | max(abs(eta)) > 700) {
            _error(3498, "weighted exact match-deleted prediction is nonfinite")
        }
        out.deleted_mu[idx] = exp(eta)
        out.beta[idx,.] = J(rows(idx),1,1)*beta'
        out.score_relres[idx] = J(rows(idx),1,relres)
        out.iterations[idx] = J(rows(idx),1,iteration)
    }
    out.status = "CONVERGED"
    return(out)
}

struct ppmltalo_dense_lo_result scalar ppmltalo__dense_exact_lo(
    real colvector y,
    real matrix X,
    real colvector start,
    real colvector offset,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_dense_lo_result scalar out
    real colvector all_rows, keep, beta, eta, mu, score, score_y, score_mu
    real colvector step, candidate_beta, candidate_eta, candidate_mu
    real matrix Xkeep, H
    real scalar n, p, row, iteration, line_iteration, relres
    real scalar objective, candidate_objective, step_length, gain, accepted
    real scalar information_scale

    n = rows(X)
    p = cols(X)
    if (n <= p | p == 0) _error(3498, "exact leave-one-out oracle requires n greater than p")
    if (rows(y) != n | cols(y) != 1 | hasmissing(y) | min(y) < 0) {
        _error(3200, "exact leave-one-out outcome must be finite and nonnegative")
    }
    if (rows(start) != p | cols(start) != 1 | hasmissing(start)) {
        _error(3200, "exact leave-one-out start must be p by 1 and finite")
    }
    if (rows(offset) != n | cols(offset) != 1 | hasmissing(offset)) {
        _error(3200, "exact leave-one-out offset must be n by 1 and finite")
    }
    if (hasmissing(X)) _error(3200, "exact leave-one-out design must be finite")
    if (tolerance <= 0 | tolerance >= 1) {
        _error(3498, "exact leave-one-out tolerance must lie in (0,1)")
    }
    if (maxiter < 1 | maxiter != floor(maxiter)) {
        _error(3498, "exact leave-one-out maxiter must be a positive integer")
    }

    all_rows = (1::n)
    out.deleted_mu = J(n,1,.)
    out.beta = J(n,p,.)
    out.score_relres = J(n,1,.)
    out.iterations = J(n,1,.)
    for (row=1; row<=n; row++) {
        keep = selectindex(all_rows :!= row)
        Xkeep = X[keep,.]
        if (rank(Xkeep) != p) {
            _error(3498, "exact leave-one-out deletion is rank unstable")
        }
        beta = start
        for (iteration=0; iteration<=maxiter; iteration++) {
            eta = offset[keep] + Xkeep*beta
            if (hasmissing(eta) | max(eta) > 700 | min(eta) < -700) {
                _error(3498, "exact deleted PPML index overflow; possible deleted face")
            }
            mu = exp(eta)
            score = Xkeep'*(y[keep]:-mu)
            score_y = Xkeep'*y[keep]
            score_mu = Xkeep'*mu
            relres = sqrt((score'*score)[1,1]) /
                (1 + sqrt((score_y'*score_y)[1,1]) +
                sqrt((score_mu'*score_mu)[1,1]))
            H = Xkeep'*(mu:*Xkeep)
            if (rank(H) != p) {
                _error(3498, "exact deleted PPML information lost rank; possible deleted face")
            }
            information_scale = max(abs(H))
            if (information_scale <= 0 | missing(information_scale)) {
                _error(3498, "exact deleted PPML information is nonfinite")
            }
            step = invsym(H:/information_scale)*(score:/information_scale)
            if (relres <= tolerance & max(abs(step)) <= sqrt(tolerance)) break
            if (iteration == maxiter) {
                _error(3498, "exact deleted PPML refit did not converge; possible deleted face")
            }
            gain = (score'*step)[1,1]
            if (gain <= 0 | gain >= .) {
                _error(3498, "exact deleted PPML Newton direction is invalid")
            }
            objective = sum(y[keep]:*eta-mu)
            step_length = 1
            accepted = 0
            for (line_iteration=1; line_iteration<=60; line_iteration++) {
                candidate_beta = beta + step_length:*step
                candidate_eta = offset[keep] + Xkeep*candidate_beta
                if (!hasmissing(candidate_eta) & max(candidate_eta) <= 700 &
                    min(candidate_eta) >= -700) {
                    candidate_mu = exp(candidate_eta)
                    candidate_objective = sum(y[keep]:*candidate_eta-candidate_mu)
                    if (candidate_objective >=
                        objective + 1e-4*step_length*gain) {
                        beta = candidate_beta
                        accepted = 1
                        break
                    }
                }
                step_length = step_length/2
            }
            if (!accepted) {
                _error(3498, "exact deleted PPML line search failed; possible deleted face")
            }
        }
        eta = offset[row] + X[row,.]*beta
        if (eta > 700 | eta < -700 | eta >= .) {
            _error(3498, "exact deleted prediction is nonfinite")
        }
        out.deleted_mu[row] = exp(eta)
        out.beta[row,.] = beta'
        out.score_relres[row] = relres
        out.iterations[row] = iteration
    }
    out.status = "CONVERGED"
    return(out)
}

struct ppmltalo_hdfe_design
{
    real scalar    n
    real scalar    k
    real scalar    q
    real scalar    p
    real matrix    id
    real matrix    controls
    real matrix    order
    real rowvector levels
    real rowvector offsets
    real rowvector params
    real scalar    control_offset
    real colvector mu
    real colvector diagH
    real colvector firm_use
    real colvector firm_coef_index
    real colvector firm_rest_index
    real colvector schur_diag
    pointer(real matrix) rowvector panelinfo
}

struct ppmltalo_joint_design
{
    struct ppmltalo_hdfe_design scalar full
    struct ppmltalo_hdfe_design scalar base
    real scalar    nuisance_parameters
    real matrix    V
    real matrix    S_inverse
    real scalar    preparation_relres
    real scalar    small_inverse_relres
    real scalar    schur_rcond
    string scalar  status
}

struct ppmltalo_deletion_gate_result
{
    real scalar bridge_rows
    real scalar positive_margin_rows
    real scalar positive_graph_missing_nodes
    real scalar positive_graph_components
    real scalar positive_graph_bridge_rows
    string scalar status
}

struct ppmltalo_pcg_result
{
    real colvector x
    real scalar    relres
    real scalar    iterations
    string scalar  status
}

struct ppmltalo_pcg_matrix_result
{
    real matrix    x
    real rowvector relres
    real rowvector column_iterations
    string rowvector column_status
    real scalar    iterations
    string scalar  status
}

struct ppmltalo_leverage_result
{
    real colvector leverage
    real colvector mcse
    real scalar    probes
    real scalar    max_solve_relres
    real scalar    max_solve_iterations
    string scalar  status
}

struct ppmltalo_correction_result
{
    real rowvector plugin
    real rowvector correction
    real rowvector mcse
    real rowvector talo
    real colvector deleted_mu
    real scalar    probes
    real scalar    max_leverage
    real scalar    max_solve_relres
    real scalar    max_solve_iterations
    real scalar    information_inverse_relres
    real scalar    information_rcond
    string scalar  status
}

struct ppmltalo_match_result
{
    real rowvector plugin
    real rowvector correction
    real rowvector mcse
    real rowvector talo
    real colvector deleted_mu
    real scalar    probes
    real scalar    blocks
    real scalar    expanded_n
    real scalar    max_block_patterns
    real scalar    max_local_rank
    real scalar    max_block_eigen
    real scalar    max_positive_block_eigen
    real scalar    information_inverse_relres
    real scalar    positive_inverse_relres
    real scalar    information_rcond
    real scalar    positive_rcond
    real scalar    min_local_kernel_rcond
    real scalar    min_local_system_rcond
    real scalar    trace_directions
    real scalar    failure_block_index
    real scalar    failure_stage
    real scalar    failure_metric_min
    real scalar    failure_metric_max
    string scalar  status
}

struct ppmltalo_local_design
{
    real colvector index
    real matrix    X
}

struct ppmltalo_scaled_inverse_result
{
    real matrix    inverse
    real scalar    relres
    real scalar    rcond
    string scalar  status
}

real scalar ppmltalo__norm2(real colvector value)
{
    real scalar scale

    if (rows(value) == 0) return(0)
    if (hasmissing(value)) return(.)
    scale = max(abs(value))
    if (scale == 0) return(0)
    return(scale*sqrt(sum((value:/scale):^2)))
}

struct ppmltalo_scaled_inverse_result scalar ppmltalo__scaled_sym_inverse(
    real matrix H,
    real scalar tolerance)
{
    struct ppmltalo_scaled_inverse_result scalar out
    real matrix scaled, scaled_inverse
    real colvector inverse_scale, eigen
    real scalar dimension

    out.status = "INVALID_INPUT"
    out.inverse = J(0,0,.)
    out.relres = .
    out.rcond = .
    if (rows(H) == 0 | rows(H) != cols(H) | hasmissing(H) |
        tolerance <= 0 | tolerance >= 1) return(out)
    dimension = rows(H)
    if (min(diagonal(H)) <= 0) {
        out.status = "SINGULAR"
        return(out)
    }
    inverse_scale = 1:/sqrt(diagonal(H))
    scaled = (inverse_scale*inverse_scale'):*H
    eigen = Re(eigenvalues(0.5:*(scaled+scaled')))
    if (hasmissing(eigen) | max(eigen) <= 0 | min(eigen) <= 0) {
        out.status = "SINGULAR"
        out.rcond = 0
        return(out)
    }
    out.rcond = min(eigen)/max(eigen)
    if (rank(scaled) != dimension) {
        out.status = "SINGULAR"
        out.rcond = 0
        return(out)
    }
    scaled_inverse = invsym(scaled)
    out.relres = ppmltalo__norm2(vec(scaled*scaled_inverse-I(dimension))) /
        sqrt(dimension)
    if (hasmissing(scaled_inverse) | out.relres > tolerance) {
        out.status = "INVERSE_FAILED"
        return(out)
    }
    out.inverse = (inverse_scale*inverse_scale'):*scaled_inverse
    if (hasmissing(out.inverse)) {
        out.status = "INVERSE_FAILED"
        return(out)
    }
    out.status = "CONVERGED"
    return(out)
}

real rowvector ppmltalo__colnorm2(real matrix value)
{
    real rowvector out
    real scalar column

    out = J(1,cols(value),.)
    for (column=1; column<=cols(value); column++) {
        out[column] = ppmltalo__norm2(value[,column])
    }
    return(out)
}

real rowvector ppmltalo__colmaxabs(real matrix value)
{
    real rowvector out
    real scalar column

    out = J(1,cols(value),.)
    for (column=1; column<=cols(value); column++) {
        if (hasmissing(value[,column])) out[column] = .
        else out[column] = max(abs(value[,column]))
    }
    return(out)
}

real scalar ppmltalo__dot(real colvector left, real colvector right)
{
    if (rows(left) != rows(right) | cols(left) != 1 | cols(right) != 1) {
        _error(3200, "dot-product inputs must be conformable column vectors")
    }
    if (hasmissing(left) | hasmissing(right)) return(.)
    return(quadcross(left,right))
}

real rowvector ppmltalo__coldot(real matrix left, real matrix right)
{
    real rowvector out
    real scalar column

    if (rows(left) != rows(right) | cols(left) != cols(right)) {
        _error(3200, "column dot-product inputs must be conformable")
    }
    out = J(1,cols(left),.)
    for (column=1; column<=cols(left); column++) {
        out[column] = ppmltalo__dot(left[,column],right[,column])
    }
    return(out)
}

real rowvector ppmltalo__coltriple(
    real matrix left,
    real colvector middle,
    real matrix right)
{
    real rowvector out
    real scalar column

    if (rows(left) != rows(right) | cols(left) != cols(right) |
        rows(left) != rows(middle) | cols(middle) != 1) {
        _error(3200, "column triple-product inputs must be conformable")
    }
    out = J(1,cols(left),.)
    if (hasmissing(middle)) return(out)
    for (column=1; column<=cols(left); column++) {
        out[column] = ppmltalo__dot(left[,column],
            middle:*right[,column])
    }
    return(out)
}

real matrix ppmltalo__compensated_update(
    real matrix state,
    real matrix values)
{
    real scalar row, column, old_total, value, new_total

    if (rows(state) != 2 | cols(state) != cols(values)) {
        _error(3200, "compensated-sum state has incompatible dimensions")
    }
    for (row=1; row<=rows(values); row++) {
        for (column=1; column<=cols(values); column++) {
            old_total = state[1,column]
            value = values[row,column]
            new_total = old_total+value
            if (abs(old_total) >= abs(value)) {
                state[2,column] = state[2,column] +
                    ((old_total-new_total)+value)
            }
            else {
                state[2,column] = state[2,column] +
                    ((value-new_total)+old_total)
            }
            state[1,column] = new_total
        }
    }
    return(state)
}

real scalar ppmltalo__pcg_status_ok(string scalar status)
{
    return(substr(status,1,9) == "CONVERGED")
}

string scalar ppmltalo__pcg_matrix_status(string rowvector status)
{
    real scalar column

    for (column=1; column<=cols(status); column++) {
        if (!ppmltalo__pcg_status_ok(status[column])) return(status[column])
    }
    return("CONVERGED")
}

pointer(real matrix) scalar ppmltalo__panelinfo(real colvector sorted_id)
{
    real matrix info

    info = panelsetup(sorted_id, 1)
    return(&info)
}

struct ppmltalo_hdfe_design scalar ppmltalo__hdfe_build_x(
    real matrix id,
    real matrix controls,
    real colvector mu)
{
    struct ppmltalo_hdfe_design scalar D
    real scalar j, g, row, left, right, root_left, root_right, total_nodes
    real colvector observed, parent, tree_size, roots, take, block_diagonal
    real colvector pair_code, pair_order, cell_worker, cell_firm
    real colvector cell_sum, cell_adjustment
    real colvector firm_order, firm_adjustment, firm_diagonal
    real matrix pair_panel, firm_panel

    if (rows(id) == 0 | cols(id) < 1) _error(3300, "empty HDFE design")
    if (rows(mu) != rows(id) | cols(mu) != 1) _error(3200, "mu must be n by 1")
    if (rows(controls) != rows(id) | hasmissing(controls)) {
        _error(3200, "controls must be a finite n by q matrix")
    }
    if (hasmissing(id) | hasmissing(mu) | min(mu) < 0 | sum(mu) <= 0) {
        _error(3498, "HDFE identifiers and information weights must be finite, nonnegative, and nonzero")
    }
    if (max(abs(id - floor(id))) != 0 | min(id) < 1) {
        _error(3498, "HDFE identifiers must be positive integers")
    }
    D.n = rows(id)
    D.k = cols(id)
    D.q = cols(controls)
    D.id = id
    D.controls = controls
    D.mu = mu
    D.levels = J(1, D.k, .)
    D.offsets = J(1, D.k, .)
    D.params = J(1, D.k, .)
    D.order = J(D.n, D.k, .)
    D.panelinfo = J(1, D.k, NULL)
    D.firm_use = J(0,1,.)
    D.firm_coef_index = J(0,1,.)
    D.firm_rest_index = J(0,1,.)
    D.schur_diag = J(0,1,.)
    D.p = 0
    for (j=1; j<=D.k; j++) {
        g = max(id[,j])
        observed = uniqrows(sort(id[,j], 1))
        if (rows(observed) != g | max(abs(observed - (1::g))) != 0) {
            _error(3498, "HDFE identifiers must be densely encoded")
        }
        D.levels[j] = g
        D.offsets[j] = D.p
        D.params[j] = g - (j > 1)
        D.p = D.p + D.params[j]
        D.order[,j] = order(id[,j], 1)
        D.panelinfo[j] = ppmltalo__panelinfo(id[D.order[,j],j])
    }
    D.control_offset = D.p
    D.p = D.p + D.q
    if (D.k >= 2) {
        if (D.levels[2] == 1) {
            _error(3498, "a one-level second fixed-effect block is unsupported")
        }
        D.firm_use = selectindex(D.id[,2] :< D.levels[2])
        D.firm_coef_index = D.offsets[2] :+ D.id[D.firm_use,2]
        D.firm_rest_index = D.id[D.firm_use,2]
    }
    D.diagH = J(D.p, 1, .)
    for (j=1; j<=D.k; j++) {
        block_diagonal = panelsum(mu[D.order[,j]], *D.panelinfo[j])
        take = (D.offsets[j]+1::D.offsets[j]+D.params[j])
        D.diagH[take] = block_diagonal[(1::D.params[j])]
    }
    if (D.q) {
        take = (D.control_offset+1::D.p)
        D.diagH[take] = colsum((mu*J(1,D.q,1)):*(controls:^2))'
    }
    if (D.k == 2 & D.q == 0) {
        pair_code = (D.id[,1]:-1):*D.levels[2] + D.id[,2]
        pair_order = order(pair_code,1)
        pair_panel = panelsetup(pair_code[pair_order],1)
        cell_sum = panelsum(D.mu[pair_order],pair_panel)
        cell_worker = D.id[pair_order[pair_panel[,1]],1]
        cell_firm = D.id[pair_order[pair_panel[,1]],2]
        cell_adjustment = (cell_sum:^2) :/ D.diagH[cell_worker]
        firm_order = order(cell_firm,1)
        firm_panel = panelsetup(cell_firm[firm_order],1)
        firm_adjustment = panelsum(cell_adjustment[firm_order],firm_panel)
        firm_diagonal = panelsum(D.mu[D.order[,2]],*D.panelinfo[2])
        firm_diagonal = firm_diagonal - firm_adjustment
        D.schur_diag = firm_diagonal[(1::D.params[2])]
        if (hasmissing(D.schur_diag) | min(D.schur_diag) <= 0) {
            _error(3498, "two-way Schur diagonal is nonpositive")
        }
    }
    if (D.k >= 2) {
        total_nodes = D.levels[1] + D.levels[2]
        parent = (1::total_nodes)
        tree_size = J(total_nodes,1,1)
        for (row=1; row<=D.n; row++) {
            left = D.id[row,1]
            right = D.levels[1] + D.id[row,2]
            root_left = left
            while (parent[root_left] != root_left) root_left = parent[root_left]
            root_right = right
            while (parent[root_right] != root_right) root_right = parent[root_right]
            if (root_left != root_right) {
                if (tree_size[root_left] < tree_size[root_right]) {
                    parent[root_left] = root_right
                    tree_size[root_right] = tree_size[root_right] +
                        tree_size[root_left]
                }
                else {
                    parent[root_right] = root_left
                    tree_size[root_left] = tree_size[root_left] +
                        tree_size[root_right]
                }
            }
        }
        roots = J(total_nodes,1,.)
        for (row=1; row<=total_nodes; row++) {
            root_left = row
            while (parent[root_left] != root_left) root_left = parent[root_left]
            roots[row] = root_left
        }
        if (rows(uniqrows(sort(roots,1))) != 1) {
            _error(3498, "worker-firm target graph is disconnected")
        }
    }
    return(D)
}

struct ppmltalo_hdfe_design scalar ppmltalo__hdfe_build(
    real matrix id,
    real colvector mu)
{
    return(ppmltalo__hdfe_build_x(id,J(rows(id),0,.),mu))
}

real colvector ppmltalo__worker_firm_bridges(
    struct ppmltalo_hdfe_design scalar D)
{
    real colvector degree, offset, cursor, neighbor, edge, discovery, low
    real colvector parent_node, parent_edge, next_position, stack, bridge
    real scalar nodes, row, left, right, position, time, top, node, other
    real scalar parent

    if (D.k != 2) {
        _error(3498, "bridge check requires exactly worker and firm blocks")
    }
    nodes = D.levels[1] + D.levels[2]
    degree = J(nodes,1,0)
    for (row=1; row<=D.n; row++) {
        left = D.id[row,1]
        right = D.levels[1] + D.id[row,2]
        degree[left] = degree[left] + 1
        degree[right] = degree[right] + 1
    }
    offset = J(nodes+1,1,1)
    for (node=1; node<=nodes; node++) offset[node+1] = offset[node]+degree[node]
    cursor = offset[(1::nodes)]
    neighbor = J(2*D.n,1,.)
    edge = J(2*D.n,1,.)
    for (row=1; row<=D.n; row++) {
        left = D.id[row,1]
        right = D.levels[1] + D.id[row,2]
        position = cursor[left]
        neighbor[position] = right
        edge[position] = row
        cursor[left] = cursor[left] + 1
        position = cursor[right]
        neighbor[position] = left
        edge[position] = row
        cursor[right] = cursor[right] + 1
    }
    discovery = J(nodes,1,0)
    low = J(nodes,1,0)
    parent_node = J(nodes,1,0)
    parent_edge = J(nodes,1,0)
    next_position = offset[(1::nodes)]
    stack = J(nodes,1,0)
    bridge = J(D.n,1,0)
    time = 1
    top = 1
    stack[top] = 1
    discovery[1] = 1
    low[1] = 1
    while (top > 0) {
        node = stack[top]
        if (next_position[node] < offset[node+1]) {
            position = next_position[node]
            next_position[node] = next_position[node] + 1
            if (edge[position] == parent_edge[node]) continue
            other = neighbor[position]
            if (discovery[other] == 0) {
                time++
                discovery[other] = time
                low[other] = time
                parent_node[other] = node
                parent_edge[other] = edge[position]
                top++
                stack[top] = other
            }
            else low[node] = min((low[node],discovery[other]))
        }
        else {
            top--
            parent = parent_node[node]
            if (parent > 0) {
                low[parent] = min((low[parent],low[node]))
                if (low[node] > discovery[parent]) {
                    bridge[parent_edge[node]] = 1
                }
            }
        }
    }
    if (sum(discovery:==0)) _error(3498, "bridge check found a disconnected graph")
    return(bridge)
}

real scalar ppmltalo__bipartite_components(
    real matrix id,
    real scalar worker_levels,
    real scalar firm_levels)
{
    real colvector parent, tree_size, roots
    real scalar nodes, row, left, right, root_left, root_right

    nodes = worker_levels + firm_levels
    parent = (1::nodes)
    tree_size = J(nodes,1,1)
    for (row=1; row<=rows(id); row++) {
        left = id[row,1]
        right = worker_levels + id[row,2]
        root_left = left
        while (parent[root_left] != root_left) root_left = parent[root_left]
        root_right = right
        while (parent[root_right] != root_right) root_right = parent[root_right]
        if (root_left != root_right) {
            if (tree_size[root_left] < tree_size[root_right]) {
                parent[root_left] = root_right
                tree_size[root_right] = tree_size[root_right] +
                    tree_size[root_left]
            }
            else {
                parent[root_right] = root_left
                tree_size[root_left] = tree_size[root_left] +
                    tree_size[root_right]
            }
        }
    }
    roots = J(nodes,1,.)
    for (row=1; row<=nodes; row++) {
        root_left = row
        while (parent[root_left] != root_left) root_left = parent[root_left]
        roots[row] = root_left
    }
    return(rows(uniqrows(sort(roots,1))))
}

struct ppmltalo_deletion_gate_result scalar ppmltalo__row_deletion_gate(
    struct ppmltalo_hdfe_design scalar D,
    real colvector y)
{
    struct ppmltalo_deletion_gate_result scalar out
    struct ppmltalo_hdfe_design scalar Dpositive
    real matrix positive_id
    real colvector bridge, margin_failure
    real colvector positive_rows, positive_worker_degree, positive_firm_degree

    if (D.k != 2) {
        _error(3498, "row-deletion certificate requires exactly two fixed-effect blocks")
    }
    if (rows(y) != D.n | cols(y) != 1 | hasmissing(y) | min(y) < 0) {
        _error(3200, "deletion-gate outcome must be finite and nonnegative")
    }
    bridge = ppmltalo__worker_firm_bridges(D)
    out.bridge_rows = sum(bridge)
    positive_rows = selectindex(y:>0)
    positive_worker_degree = J(D.levels[1],1,0)
    positive_firm_degree = J(D.levels[2],1,0)
    margin_failure = J(D.n,1,0)
    out.positive_graph_missing_nodes = D.levels[1] + D.levels[2]
    out.positive_graph_components = out.positive_graph_missing_nodes
    out.positive_graph_bridge_rows = .
    if (rows(positive_rows)) {
        positive_id = D.id[positive_rows,.]
        positive_worker_degree = panelsum((y:>0)[D.order[,1]],
            *D.panelinfo[1])
        positive_firm_degree = panelsum((y:>0)[D.order[,2]],
            *D.panelinfo[2])
        margin_failure[positive_rows] =
            (positive_worker_degree[positive_id[,1]]:<=1) :|
            (positive_firm_degree[positive_id[,2]]:<=1)
        out.positive_graph_missing_nodes =
            sum(positive_worker_degree:==0) + sum(positive_firm_degree:==0)
        if (rows(positive_rows) == D.n) {
            out.positive_graph_components = 1
            out.positive_graph_bridge_rows = out.bridge_rows
        }
        else {
            out.positive_graph_components = ppmltalo__bipartite_components(
                positive_id,D.levels[1],D.levels[2])
            if (out.positive_graph_components == 1) {
                Dpositive.n = rows(positive_id)
                Dpositive.k = 2
                Dpositive.id = positive_id
                Dpositive.levels = D.levels[(1,2)]
                out.positive_graph_bridge_rows =
                    sum(ppmltalo__worker_firm_bridges(Dpositive))
            }
        }
    }
    out.positive_margin_rows = sum(margin_failure)
    if (out.bridge_rows > 0) out.status = "ROW_DELETION_RANK_FAILURE"
    else if (out.positive_margin_rows > 0) {
        out.status = "ROW_DELETION_POSITIVE_MARGIN_FAILURE"
    }
    else if (out.positive_graph_missing_nodes > 0 |
             out.positive_graph_components > 1 |
             (!missing(out.positive_graph_bridge_rows) &
              out.positive_graph_bridge_rows > 0)) {
        out.status = "ROW_DELETION_FACE_CERTIFICATE_FAILURE"
    }
    else out.status = "PASSED_TWO_WAY_DELETION_CERTIFICATE"
    return(out)
}

struct ppmltalo_deletion_gate_result scalar ppmltalo__row_gate_weighted(
    struct ppmltalo_hdfe_design scalar D,
    real colvector y,
    real colvector frequency)
{
    struct ppmltalo_deletion_gate_result scalar out
    struct ppmltalo_hdfe_design scalar Dpositive
    real matrix positive_id
    real colvector bridge, positive_bridge, margin_failure
    real colvector positive_rows, worker_mass, firm_mass, positive_frequency

    if (D.k != 2) {
        _error(3498,"weighted row certificate requires two fixed-effect blocks")
    }
    if (rows(y) != D.n | rows(frequency) != D.n | cols(y) != 1 |
        cols(frequency) != 1 | hasmissing(y) | hasmissing(frequency) |
        min(y) < 0 | min(frequency) < 1 |
        max(abs(frequency-floor(frequency))) != 0) {
        _error(3200,"weighted deletion-gate inputs are invalid")
    }
    bridge = ppmltalo__worker_firm_bridges(D):*(frequency:==1)
    out.bridge_rows = sum(bridge)
    positive_rows = selectindex(y:>0)
    worker_mass = J(D.levels[1],1,0)
    firm_mass = J(D.levels[2],1,0)
    margin_failure = J(D.n,1,0)
    out.positive_graph_missing_nodes = D.levels[1]+D.levels[2]
    out.positive_graph_components = out.positive_graph_missing_nodes
    out.positive_graph_bridge_rows = .
    if (rows(positive_rows)) {
        positive_id = D.id[positive_rows,.]
        positive_frequency = frequency[positive_rows]
        worker_mass = panelsum((frequency:*(y:>0))[D.order[,1]],
            *D.panelinfo[1])
        firm_mass = panelsum((frequency:*(y:>0))[D.order[,2]],
            *D.panelinfo[2])
        margin_failure[positive_rows] =
            (worker_mass[positive_id[,1]]:<=1):|
            (firm_mass[positive_id[,2]]:<=1)
        out.positive_graph_missing_nodes =
            sum(worker_mass:==0)+sum(firm_mass:==0)
        if (rows(positive_rows) == D.n) {
            out.positive_graph_components = 1
            out.positive_graph_bridge_rows = out.bridge_rows
        }
        else {
            out.positive_graph_components = ppmltalo__bipartite_components(
                positive_id,D.levels[1],D.levels[2])
            if (out.positive_graph_components == 1) {
                Dpositive.n = rows(positive_id)
                Dpositive.k = 2
                Dpositive.id = positive_id
                Dpositive.levels = D.levels[(1,2)]
                positive_bridge = ppmltalo__worker_firm_bridges(Dpositive):*
                    (positive_frequency:==1)
                out.positive_graph_bridge_rows = sum(positive_bridge)
            }
        }
    }
    out.positive_margin_rows = sum(margin_failure)
    if (out.bridge_rows > 0) out.status = "ROW_DELETION_RANK_FAILURE"
    else if (out.positive_margin_rows > 0) {
        out.status = "ROW_DELETION_POSITIVE_MARGIN_FAILURE"
    }
    else if (out.positive_graph_missing_nodes > 0 |
             out.positive_graph_components > 1 |
             (!missing(out.positive_graph_bridge_rows) &
              out.positive_graph_bridge_rows > 0)) {
        out.status = "ROW_DELETION_FACE_CERTIFICATE_FAILURE"
    }
    else out.status = "PASSED_TWO_WAY_DELETION_CERTIFICATE"
    return(out)
}

real colvector ppmltalo__hdfe_x(
    struct ppmltalo_hdfe_design scalar D,
    real colvector v)
{
    real colvector out, idx, use
    real scalar j

    if (rows(v) != D.p | cols(v) != 1) _error(3200, "coefficient vector has wrong dimension")
    out = J(D.n, 1, 0)
    for (j=1; j<=D.k; j++) {
        if (j == 1) {
            idx = D.offsets[j] :+ D.id[,j]
            out = out + v[idx]
        }
        else {
            if (D.k == 2 & j == 2) {
                use = D.firm_use
                idx = D.firm_coef_index
            }
            else {
                use = selectindex(D.id[,j] :< D.levels[j])
                idx = D.offsets[j] :+ D.id[use,j]
            }
            if (rows(use)) {
                out[use] = out[use] + v[idx]
            }
        }
    }
    if (D.q) {
        out = out + D.controls*v[(D.control_offset+1::D.p)]
    }
    return(out)
}

real matrix ppmltalo__hdfe_xm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V)
{
    real matrix out
    real colvector idx, use
    real scalar j

    if (rows(V) != D.p) _error(3200, "coefficient matrix has wrong row dimension")
    out = J(D.n, cols(V), 0)
    for (j=1; j<=D.k; j++) {
        if (j == 1) {
            idx = D.offsets[j] :+ D.id[,j]
            out = out + V[idx,.]
        }
        else {
            if (D.k == 2 & j == 2) {
                use = D.firm_use
                idx = D.firm_coef_index
            }
            else {
                use = selectindex(D.id[,j] :< D.levels[j])
                idx = D.offsets[j] :+ D.id[use,j]
            }
            if (rows(use)) {
                out[use,.] = out[use,.] + V[idx,.]
            }
        }
    }
    if (D.q) {
        out = out + D.controls*V[(D.control_offset+1::D.p),.]
    }
    return(out)
}

real colvector ppmltalo__hdfe_xt(
    struct ppmltalo_hdfe_design scalar D,
    real colvector z)
{
    real colvector out, ord, sums, take
    real matrix info
    real scalar j

    if (rows(z) != D.n | cols(z) != 1) _error(3200, "row vector has wrong dimension")
    out = J(D.p, 1, 0)
    for (j=1; j<=D.k; j++) {
        ord = D.order[,j]
        sums = panelsum(z[ord], *D.panelinfo[j])
        take = (D.offsets[j]+1::D.offsets[j]+D.params[j])
        out[take] = sums[(1::D.params[j])]
    }
    if (D.q) {
        out[(D.control_offset+1::D.p)] = D.controls'*z
    }
    return(out)
}

real matrix ppmltalo__hdfe_xtm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix Z)
{
    real matrix out, sums
    real colvector ord, take
    real scalar j

    if (rows(Z) != D.n) _error(3200, "row matrix has wrong row dimension")
    out = J(D.p, cols(Z), 0)
    for (j=1; j<=D.k; j++) {
        ord = D.order[,j]
        sums = panelsum(Z[ord,.], *D.panelinfo[j])
        take = (D.offsets[j]+1::D.offsets[j]+D.params[j])
        out[take,.] = sums[(1::D.params[j]),.]
    }
    if (D.q) {
        out[(D.control_offset+1::D.p),.] = D.controls'*Z
    }
    return(out)
}

real colvector ppmltalo__hdfe_H(
    struct ppmltalo_hdfe_design scalar D,
    real colvector v)
{
    return(ppmltalo__hdfe_xt(D, D.mu :* ppmltalo__hdfe_x(D, v)))
}

real matrix ppmltalo__hdfe_Hm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V)
{
    return(ppmltalo__hdfe_xtm(D,
        (D.mu * J(1,cols(V),1)) :* ppmltalo__hdfe_xm(D, V)))
}

real matrix ppmltalo__lexicographic_panel(
    real matrix keys,
    real colvector ord)
{
    real matrix sorted
    real colvector boundary, start, finish
    real scalar n

    n = rows(keys)
    if (n == 0 | cols(keys) != 2 | rows(ord) != n) {
        _error(3200,"lexicographic panel inputs are invalid")
    }
    sorted = keys[ord,.]
    boundary = J(n,1,0)
    boundary[1] = 1
    if (n > 1) {
        boundary[(2::n)] =
            (sorted[(2::n),1] :!= sorted[(1::n-1),1]) :|
            (sorted[(2::n),2] :!= sorted[(1::n-1),2])
    }
    start = selectindex(boundary)
    if (rows(start) == 1) finish = n
    else finish = start[(2::rows(start))]:-1 \ n
    return((start,finish))
}

real matrix ppmltalo__hdfe_dense_H(
    struct ppmltalo_hdfe_design scalar D,
    real colvector weight)
{
    real matrix H, pair_panel, pair_values, grouped, control_cross
    real colvector pair_order, pair_sum, left_id, right_id
    real colvector use, left_index, right_index, ord, take
    real scalar left_block, right_block, position

    if (rows(weight) != D.n | cols(weight) != 1 | hasmissing(weight) |
        min(weight) < 0 | sum(weight) <= 0) {
        _error(3200, "dense information weights must be finite, nonnegative, and nonzero")
    }
    H = J(D.p,D.p,0)
    for (left_block=1; left_block<=D.k; left_block++) {
        ord = D.order[,left_block]
        grouped = panelsum(weight[ord],*D.panelinfo[left_block])
        take = (D.offsets[left_block]+1::
            D.offsets[left_block]+D.params[left_block])
        for (position=1; position<=rows(take); position++) {
            H[take[position],take[position]] = grouped[position]
        }
    }
    for (left_block=1; left_block<D.k; left_block++) {
        for (right_block=left_block+1; right_block<=D.k; right_block++) {
            // Sort the two identifiers lexicographically.  Arithmetic pair
            // encodings can collide once an integer product exceeds the exact
            // range of an IEEE double.
            pair_values = D.id[,(left_block,right_block)]
            pair_order = order(pair_values,(1,2))
            pair_panel = ppmltalo__lexicographic_panel(
                pair_values,pair_order)
            pair_sum = panelsum(weight[pair_order],pair_panel)
            left_id = pair_values[pair_order[pair_panel[,1]],1]
            right_id = pair_values[pair_order[pair_panel[,1]],2]
            use = selectindex(((left_block:==1):|(left_id:<D.levels[left_block])) :&
                              (right_id:<D.levels[right_block]))
            if (rows(use)) {
                left_index = D.offsets[left_block] :+ left_id[use]
                right_index = D.offsets[right_block] :+ right_id[use]
                for (position=1; position<=rows(use); position++) {
                    H[left_index[position],right_index[position]] =
                        pair_sum[use[position]]
                    H[right_index[position],left_index[position]] =
                        pair_sum[use[position]]
                }
            }
        }
    }
    if (D.q) {
        take = (D.control_offset+1::D.p)
        control_cross = D.controls' * (weight:*D.controls)
        H[take,take] = control_cross
        for (left_block=1; left_block<=D.k; left_block++) {
            ord = D.order[,left_block]
            grouped = panelsum((weight:*D.controls)[ord,.],
                *D.panelinfo[left_block])
            left_index = (D.offsets[left_block]+1::
                D.offsets[left_block]+D.params[left_block])
            H[left_index,take] = grouped[(1::D.params[left_block]),.]
            H[take,left_index] = H[left_index,take]'
        }
    }
    return(0.5:*(H+H'))
}

real matrix ppmltalo__hdfe_first_xm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V)
{
    if (rows(V) != D.params[1]) {
        _error(3200, "first-block coefficient matrix has wrong row dimension")
    }
    return(V[D.id[,1],.])
}

real matrix ppmltalo__hdfe_rest_xm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V)
{
    real matrix out
    real colvector use, idx
    real scalar j, p1

    p1 = D.params[1]
    if (rows(V) != D.p-p1) {
        _error(3200, "Schur coefficient matrix has wrong row dimension")
    }
    out = J(D.n, cols(V), 0)
    if (D.k == 2) {
        if (rows(D.firm_use)) {
            out[D.firm_use,.] = V[D.firm_rest_index,.]
        }
        return(out)
    }
    for (j=2; j<=D.k; j++) {
        use = selectindex(D.id[,j] :< D.levels[j])
        if (rows(use)) {
            idx = D.offsets[j] - p1 :+ D.id[use,j]
            out[use,.] = out[use,.] + V[idx,.]
        }
    }
    if (D.q) {
        idx = (D.control_offset-p1+1::D.p-p1)
        out = out + D.controls*V[idx,.]
    }
    return(out)
}

real matrix ppmltalo__hdfe_rest_xtm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix Z)
{
    real matrix out, sums
    real colvector ord, take
    real scalar j, p1

    if (rows(Z) != D.n) _error(3200, "Schur row matrix has wrong row dimension")
    p1 = D.params[1]
    out = J(D.p-p1, cols(Z), 0)
    for (j=2; j<=D.k; j++) {
        ord = D.order[,j]
        sums = panelsum(Z[ord,.], *D.panelinfo[j])
        take = (D.offsets[j]-p1+1::D.offsets[j]-p1+D.params[j])
        out[take,.] = sums[(1::D.params[j]),.]
    }
    if (D.q) {
        take = (D.control_offset-p1+1::D.p-p1)
        out[take,.] = D.controls'*Z
    }
    return(out)
}

real matrix ppmltalo__hdfe_schur_Hm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V)
{
    real matrix values, weighted, worker_sums, worker_means
    real colvector ord
    real scalar m

    m = cols(V)
    values = ppmltalo__hdfe_rest_xm(D, V)
    weighted = (D.mu*J(1,m,1)) :* values
    ord = D.order[,1]
    worker_sums = panelsum(weighted[ord,.], *D.panelinfo[1])
    worker_means = worker_sums :/
        (D.diagH[(1::D.params[1])]*J(1,m,1))
    values = values - worker_means[D.id[,1],.]
    return(ppmltalo__hdfe_rest_xtm(D,
        (D.mu*J(1,m,1)) :* values))
}

struct ppmltalo_pcg_result scalar ppmltalo__hdfe_schur_pcg(
    struct ppmltalo_hdfe_design scalar D,
    real colvector b,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_pcg_result scalar out
    real colvector bwork, b1, br, rhs, xrest, r, z, direction, Sd
    real colvector first_base, cross_first, rest_index, diagonal_rest
    real scalar p1, bnorm, rhsnorm, rho, rho_new, denom, alpha, bscale
    real scalar relres, iteration, best_relres, stale

    p1 = D.params[1]
    bscale = max(abs(b))
    if (bscale == 0) {
        out.x = J(D.p,1,0)
        out.relres = 0
        out.iterations = 0
        out.status = "CONVERGED_ZERO_RHS"
        return(out)
    }
    bwork = b:/bscale
    bnorm = ppmltalo__norm2(bwork)
    rest_index = (p1+1::D.p)
    b1 = bwork[(1::p1)]
    br = bwork[rest_index]
    first_base = ppmltalo__hdfe_first_xm(D,
        b1 :/ D.diagH[(1::p1)])
    rhs = br - ppmltalo__hdfe_rest_xtm(D, D.mu :* first_base)
    if (D.k == 2 & D.q == 0) diagonal_rest = D.schur_diag
    else diagonal_rest = D.diagH[rest_index]
    xrest = J(D.p-p1,1,0)
    r = rhs
    rhsnorm = ppmltalo__norm2(rhs)
    out.status = "CONVERGED"
    iteration = 0
    if (rhsnorm/bnorm > tolerance/2) {
        z = r :/ diagonal_rest
        direction = z
        rho = ppmltalo__dot(r,z)
        out.status = "MAXITER"
        best_relres = rhsnorm/bnorm
        stale = 0
        for (iteration=1; iteration<=maxiter; iteration++) {
            Sd = ppmltalo__hdfe_schur_Hm(D, direction)
            denom = ppmltalo__dot(direction,Sd)
            if (denom <= 0 | denom >= .) {
                out.status = "BREAKDOWN_NONPOSITIVE_CURVATURE"
                break
            }
            alpha = rho/denom
            xrest = xrest + alpha:*direction
            r = r - alpha:*Sd
            relres = ppmltalo__norm2(r)/bnorm
            if (relres <= tolerance/2) {
                out.status = "CONVERGED"
                break
            }
            if (relres < best_relres) {
                best_relres = relres
                stale = 0
            }
            else stale++
            if (stale >= 500) {
                out.status = "STAGNATED"
                break
            }
            z = r :/ diagonal_rest
            rho_new = ppmltalo__dot(r,z)
            if (rho_new <= 0 | rho_new >= .) {
                out.status = "BREAKDOWN_PRECONDITIONER"
                break
            }
            if (mod(iteration,100) == 0) {
                r = rhs - ppmltalo__hdfe_schur_Hm(D,xrest)
                relres = ppmltalo__norm2(r)/bnorm
                if (relres <= tolerance/2) {
                    out.status = "CONVERGED"
                    break
                }
                z = r :/ diagonal_rest
                rho_new = ppmltalo__dot(r,z)
                if (rho_new <= 0 | rho_new >= .) {
                    out.status = "BREAKDOWN_PRECONDITIONER"
                    break
                }
                direction = z + (rho_new/rho):*direction
            }
            else direction = z + (rho_new/rho):*direction
            rho = rho_new
        }
    }
    cross_first = panelsum(
        (D.mu :* ppmltalo__hdfe_rest_xm(D,xrest))[D.order[,1]],
        *D.panelinfo[1])
    out.x = ((b1-cross_first):/D.diagH[(1::p1)]) \ xrest
    r = bwork - ppmltalo__hdfe_H(D,out.x)
    out.relres = ppmltalo__norm2(r)/bnorm
    out.iterations = min((iteration,maxiter))
    if (out.relres <= tolerance) out.status = "CONVERGED"
    else if (out.status == "CONVERGED") {
        out.status = "FAILED_RESIDUAL_CERTIFICATE"
    }
    out.x = bscale:*out.x
    if (hasmissing(out.x)) out.status = "NONFINITE_SOLUTION"
    return(out)
}

struct ppmltalo_pcg_matrix_result scalar ppmltalo__hdfe_schur_pcg_matrix(
    struct ppmltalo_hdfe_design scalar D,
    real matrix B,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_pcg_matrix_result scalar out
    real matrix Bwork, B1, Br, rhs, Xrest, R, Z, direction, Sd, diagonal_rest
    real matrix first_base, cross_first
    real rowvector bnorm, rhsnorm, rho, rho_new, denom, alpha, beta, active
    real rowvector bscale, zero_column, best_relres, stale
    real colvector use, bad, inactive, rest_index
    real scalar p1, m, iteration, column

    p1 = D.params[1]
    rest_index = (p1+1::D.p)
    m = cols(B)
    bscale = ppmltalo__colmaxabs(B)
    zero_column = (bscale:==0)
    bscale = bscale + (bscale:==0)
    Bwork = B :/ (J(rows(B),1,1)*bscale)
    bnorm = ppmltalo__colnorm2(Bwork)
    bnorm = bnorm + (bnorm:==0)
    B1 = Bwork[(1::p1),.]
    Br = Bwork[rest_index,.]
    first_base = ppmltalo__hdfe_first_xm(D,
        B1 :/ (D.diagH[(1::p1)]*J(1,m,1)))
    rhs = Br - ppmltalo__hdfe_rest_xtm(D,
        (D.mu*J(1,m,1)):*first_base)
    Xrest = J(D.p-p1,m,0)
    R = rhs
    rhsnorm = ppmltalo__colnorm2(rhs)
    active = (rhsnorm:/bnorm:>tolerance/2) :& (zero_column:==0)
    out.column_status = J(1,m,"MAXITER")
    out.column_iterations = J(1,m,0)
    for (column=1; column<=m; column++) {
        if (zero_column[column]) {
            out.column_status[column] = "CONVERGED_ZERO_RHS"
        }
        else if (!active[column]) {
            out.column_status[column] = "CONVERGED_INITIAL"
        }
    }
    inactive = selectindex((active:==0)')
    if (rows(inactive)) R[,inactive] = J(D.p-p1,rows(inactive),0)
    if (D.k == 2 & D.q == 0) diagonal_rest = D.schur_diag*J(1,m,1)
    else diagonal_rest = D.diagH[rest_index]*J(1,m,1)
    Z = R:/diagonal_rest
    direction = Z
    rho = ppmltalo__coldot(R,Z)
    best_relres = rhsnorm:/bnorm
    stale = J(1,m,0)
    iteration = 0
    for (iteration=1; iteration<=maxiter & sum(active)>0; iteration++) {
        Sd = ppmltalo__hdfe_schur_Hm(D,direction)
        denom = ppmltalo__coldot(direction,Sd)
        bad = selectindex((active :& ((denom:<=0):|(denom:>=.)))')
        if (rows(bad)) {
            for (column=1; column<=rows(bad); column++) {
                out.column_status[bad[column]] =
                    "BREAKDOWN_NONPOSITIVE_CURVATURE"
                out.column_iterations[bad[column]] = iteration
            }
            active[bad'] = J(1,rows(bad),0)
            R[,bad] = J(D.p-p1,rows(bad),0)
            direction[,bad] = J(D.p-p1,rows(bad),0)
        }
        use = selectindex(active')
        if (!rows(use)) break
        alpha = J(1,m,0)
        alpha[use] = rho[use]:/denom[use]
        Xrest = Xrest + direction*diag(alpha)
        R = R - Sd*diag(alpha)
        out.relres = ppmltalo__colnorm2(R):/bnorm
        for (column=1; column<=rows(use); column++) {
            if (out.relres[use[column]] <= tolerance/2) {
                active[use[column]] = 0
                out.column_status[use[column]] = "CONVERGED"
                out.column_iterations[use[column]] = iteration
            }
            else if (out.relres[use[column]] < best_relres[use[column]]) {
                best_relres[use[column]] = out.relres[use[column]]
                stale[use[column]] = 0
            }
            else {
                stale[use[column]] = stale[use[column]] + 1
                if (stale[use[column]] >= 500) {
                    active[use[column]] = 0
                    out.column_status[use[column]] = "STAGNATED"
                    out.column_iterations[use[column]] = iteration
                }
            }
        }
        inactive = selectindex((active:==0)')
        if (rows(inactive)) R[,inactive] = J(D.p-p1,rows(inactive),0)
        if (sum(active) == 0) break
        if (mod(iteration,100) == 0) {
            R = rhs - ppmltalo__hdfe_schur_Hm(D,Xrest)
            inactive = selectindex((active:==0)')
            if (rows(inactive)) R[,inactive] = J(D.p-p1,rows(inactive),0)
            out.relres = ppmltalo__colnorm2(R):/bnorm
            use = selectindex((active :& (out.relres:<=tolerance/2))')
            if (rows(use)) {
                for (column=1; column<=rows(use); column++) {
                    active[use[column]] = 0
                    out.column_status[use[column]] = "CONVERGED_REPLACED"
                    out.column_iterations[use[column]] = iteration
                }
                R[,use] = J(D.p-p1,rows(use),0)
            }
            if (sum(active) == 0) break
        }
        Z = R:/diagonal_rest
        rho_new = ppmltalo__coldot(R,Z)
        bad = selectindex((active :& ((rho_new:<=0):|(rho_new:>=.)))')
        if (rows(bad)) {
            for (column=1; column<=rows(bad); column++) {
                out.column_status[bad[column]] = "BREAKDOWN_PRECONDITIONER"
                out.column_iterations[bad[column]] = iteration
            }
            active[bad'] = J(1,rows(bad),0)
            R[,bad] = J(D.p-p1,rows(bad),0)
            direction[,bad] = J(D.p-p1,rows(bad),0)
        }
        use = selectindex(active')
        if (!rows(use)) break
        beta = J(1,m,0)
        beta[use] = rho_new[use]:/rho[use]
        direction = Z + direction*diag(beta)
        if (rows(inactive)) direction[,inactive] = J(D.p-p1,rows(inactive),0)
        rho = rho_new
    }
    use = selectindex(active')
    if (rows(use)) {
        for (column=1; column<=rows(use); column++) {
            out.column_status[use[column]] = "MAXITER"
            out.column_iterations[use[column]] = maxiter
        }
    }
    cross_first = panelsum(
        ((D.mu*J(1,m,1)):*ppmltalo__hdfe_rest_xm(D,Xrest))[D.order[,1],.],
        *D.panelinfo[1])
    out.x = ((B1-cross_first):/
        (D.diagH[(1::p1)]*J(1,m,1))) \ Xrest
    R = Bwork - ppmltalo__hdfe_Hm(D,out.x)
    out.relres = ppmltalo__colnorm2(R):/bnorm
    for (column=1; column<=m; column++) {
        if (out.relres[column] <= tolerance) {
            if (!ppmltalo__pcg_status_ok(out.column_status[column])) {
                out.column_status[column] = "CONVERGED_RECOMPUTED"
            }
        }
        else if (ppmltalo__pcg_status_ok(out.column_status[column])) {
            out.column_status[column] = "FAILED_RESIDUAL_CHECK"
        }
    }
    out.x = out.x :* (J(rows(out.x),1,1)*bscale)
    for (column=1; column<=m; column++) {
        if (hasmissing(out.x[,column])) {
            out.column_status[column] = "NONFINITE_SOLUTION"
        }
    }
    out.iterations = max(out.column_iterations)
    out.status = ppmltalo__pcg_matrix_status(out.column_status)
    return(out)
}

real colvector ppmltalo__hdfe_sgs(
    struct ppmltalo_hdfe_design scalar D,
    real colvector r)
{
    real colvector y, out, fitted, ord, cross, take, coefficient
    real scalar j

    if (rows(r) != D.p | cols(r) != 1) _error(3200, "preconditioner input has wrong dimension")
    y = J(D.p, 1, 0)
    fitted = J(D.n, 1, 0)
    for (j=1; j<=D.k; j++) {
        ord = D.order[,j]
        cross = panelsum((D.mu :* fitted)[ord], *D.panelinfo[j])
        take = (D.offsets[j]+1::D.offsets[j]+D.params[j])
        coefficient = (r[take] :- cross[(1::D.params[j])]) :/ D.diagH[take]
        y[take] = coefficient
        fitted = fitted + ppmltalo__hdfe_block_x(D, y, j)
    }
    out = J(D.p, 1, 0)
    fitted = J(D.n, 1, 0)
    for (j=D.k; j>=1; j--) {
        ord = D.order[,j]
        cross = panelsum((D.mu :* fitted)[ord], *D.panelinfo[j])
        take = (D.offsets[j]+1::D.offsets[j]+D.params[j])
        coefficient = (D.diagH[take] :* y[take] :-
            cross[(1::D.params[j])]) :/ D.diagH[take]
        out[take] = coefficient
        fitted = fitted + ppmltalo__hdfe_block_x(D, out, j)
    }
    return(out)
}

struct ppmltalo_pcg_result scalar ppmltalo__hdfe_pcg(
    struct ppmltalo_hdfe_design scalar D,
    real colvector b,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_pcg_result scalar out
    real colvector bwork, r, z, direction, Hd
    real scalar bnorm, rho, rho_new, denom, alpha, relres, iteration, bscale
    real scalar best_relres, stale

    if (rows(b) != D.p | cols(b) != 1) _error(3200, "PCG right-hand side has wrong dimension")
    if (hasmissing(b)) _error(3200, "PCG right-hand side must be finite")
    if (tolerance < 1e-15 | tolerance >= 1 | tolerance >= .) _error(3498, "PCG tolerance must lie in [1e-15,1)")
    if (maxiter < 1 | maxiter >= . | maxiter != floor(maxiter)) _error(3498, "PCG maxiter must be a finite positive integer")
    if (hasmissing(D.diagH) | min(D.diagH) <= 0) _error(3498, "PCG preconditioner must have a finite positive diagonal")
    if (D.k >= 2 & D.p > D.params[1]) {
        return(ppmltalo__hdfe_schur_pcg(D,b,tolerance,maxiter))
    }
    bscale = max(abs(b))
    if (bscale == 0) {
        out.x = J(D.p, 1, 0)
        out.relres = 0
        out.iterations = 0
        out.status = "CONVERGED_ZERO_RHS"
        return(out)
    }
    bwork = b:/bscale
    out.x = J(D.p, 1, 0)
    r = bwork
    bnorm = ppmltalo__norm2(bwork)
    z = r :/ D.diagH
    direction = z
    rho = ppmltalo__dot(r,z)
    out.status = "MAXITER"
    best_relres = 1
    stale = 0
    for (iteration=1; iteration<=maxiter; iteration++) {
        Hd = ppmltalo__hdfe_H(D, direction)
        denom = ppmltalo__dot(direction,Hd)
        if (denom <= 0 | denom >= .) {
            out.status = "BREAKDOWN_NONPOSITIVE_CURVATURE"
            break
        }
        alpha = rho / denom
        out.x = out.x + alpha :* direction
        r = r - alpha :* Hd
        relres = ppmltalo__norm2(r) / bnorm
        if (relres <= tolerance) {
            out.status = "CONVERGED"
            break
        }
        if (relres < best_relres) {
            best_relres = relres
            stale = 0
        }
        else stale++
        if (stale >= 500) {
            out.status = "STAGNATED"
            break
        }
        z = r :/ D.diagH
        rho_new = ppmltalo__dot(r,z)
        if (rho_new <= 0 | rho_new >= .) {
            out.status = "BREAKDOWN_PRECONDITIONER"
            break
        }
        if (mod(iteration,100) == 0) {
            r = bwork - ppmltalo__hdfe_H(D,out.x)
            relres = ppmltalo__norm2(r)/bnorm
            if (relres <= tolerance) {
                out.status = "CONVERGED"
                break
            }
            z = r :/ D.diagH
            rho_new = ppmltalo__dot(r,z)
            if (rho_new <= 0 | rho_new >= .) {
                out.status = "BREAKDOWN_PRECONDITIONER"
                break
            }
            direction = z + (rho_new/rho):*direction
        }
        else direction = z + (rho_new/rho) :* direction
        rho = rho_new
    }
    out.iterations = min((iteration, maxiter))
    r = bwork - ppmltalo__hdfe_H(D, out.x)
    out.relres = ppmltalo__norm2(r) / bnorm
    if (out.relres <= tolerance) out.status = "CONVERGED"
    else if (out.status == "CONVERGED") {
        out.status = "FAILED_RESIDUAL_CERTIFICATE"
    }
    out.x = bscale:*out.x
    if (hasmissing(out.x)) out.status = "NONFINITE_SOLUTION"
    return(out)
}

struct ppmltalo_pcg_matrix_result scalar ppmltalo__hdfe_pcg_matrix(
    struct ppmltalo_hdfe_design scalar D,
    real matrix B,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_pcg_matrix_result scalar out
    struct ppmltalo_pcg_result scalar single
    real scalar column, m

    if (rows(B) != D.p) _error(3200, "PCG right-hand-side matrix has wrong row dimension")
    if (cols(B) < 1) _error(3200, "PCG right-hand-side matrix must have at least one column")
    if (hasmissing(B)) _error(3200, "PCG right-hand sides must be finite")
    if (tolerance < 1e-15 | tolerance >= 1 | tolerance >= .) _error(3498, "PCG tolerance must lie in [1e-15,1)")
    if (maxiter < 1 | maxiter >= . | maxiter != floor(maxiter)) _error(3498, "PCG maxiter must be a finite positive integer")
    if (hasmissing(D.diagH) | min(D.diagH) <= 0) _error(3498, "PCG preconditioner must have a finite positive diagonal")
    m = cols(B)
    if (m == 1) {
        single = ppmltalo__hdfe_pcg(D, B, tolerance, maxiter)
        out.x = single.x
        out.relres = single.relres
        out.column_iterations = single.iterations
        out.column_status = single.status
        out.iterations = single.iterations
        if (ppmltalo__pcg_status_ok(single.status)) out.status = "CONVERGED"
        else out.status = single.status
        return(out)
    }
    if (D.k >= 2 & D.p > D.params[1]) {
        return(ppmltalo__hdfe_schur_pcg_matrix(D,B,tolerance,maxiter))
    }
    out.x = J(D.p, m, 0)
    out.relres = J(1,m,.)
    out.column_iterations = J(1,m,.)
    out.column_status = J(1,m,"")
    for (column=1; column<=m; column++) {
        single = ppmltalo__hdfe_pcg(D,B[,column],tolerance,maxiter)
        out.x[,column] = single.x
        out.relres[column] = single.relres
        out.column_iterations[column] = single.iterations
        out.column_status[column] = single.status
    }
    out.iterations = max(out.column_iterations)
    out.status = ppmltalo__pcg_matrix_status(out.column_status)
    return(out)
}

real matrix ppmltalo__joint_nuisance_xm(
    struct ppmltalo_joint_design scalar J,
    real matrix C)
{
    if (rows(C) != J.nuisance_parameters) {
        _error(3200,"joint nuisance coefficient matrix has wrong dimension")
    }
    if (J.nuisance_parameters == 0) return(J(J.full.n,cols(C),0))
    return(ppmltalo__hdfe_xm(J.full,
        J(J.base.p,cols(C),0) \ C))
}

real matrix ppmltalo__joint_nuisance_xtm(
    struct ppmltalo_joint_design scalar J,
    real matrix R)
{
    real matrix out

    if (rows(R) != J.full.n) {
        _error(3200,"joint nuisance row matrix has wrong dimension")
    }
    if (J.nuisance_parameters == 0) return(J(0,cols(R),.))
    out = ppmltalo__hdfe_xtm(J.full,R)
    return(out[(J.base.p+1::J.full.p),.])
}

real colvector ppmltalo__tree_lev_upper(
    real matrix id,
    real colvector information_weight,
    real colvector deletion_weight)
{
    real colvector ord, cell_weight, cell_worker, cell_firm
    real colvector parent_set, set_size, tree_u, tree_v, tree_r
    real colvector degree, offset, cursor, neighbor, edge_r
    real colvector tree_parent, depth, prefix, queue, row_distance
    real colvector cell_distance, cell_index_sorted, out
    real matrix panel, up
    real scalar workers, firms, nodes, cells, c, u, v, ru, rv, edges
    real scalar pos, head, tail, child, level, levels, delta, lca
    real scalar left, right

    if (cols(id) != 2 | rows(id) == 0 | rows(information_weight) != rows(id) |
        rows(deletion_weight) != rows(id) | hasmissing(id) |
        hasmissing(information_weight) | hasmissing(deletion_weight) |
        min(information_weight) <= 0 | min(deletion_weight) < 0 |
        max(deletion_weight:-information_weight) >
            1e-12*max((1,max(information_weight)))) {
        _error(3200,"tree leverage inputs are invalid")
    }
    workers = max(id[,1])
    firms = max(id[,2])
    nodes = workers+firms
    // Keep the pair key as two columns so distinct large identifiers cannot
    // alias through floating-point multiplication.
    ord = order(id,(1,2))
    panel = ppmltalo__lexicographic_panel(id,ord)
    cells = rows(panel)
    cell_weight = panelsum(information_weight[ord],panel)
    cell_worker = id[ord[panel[,1]],1]
    cell_firm = workers:+id[ord[panel[,1]],2]

    parent_set = (1::nodes)
    set_size = J(nodes,1,1)
    tree_u = J(nodes-1,1,.)
    tree_v = J(nodes-1,1,.)
    tree_r = J(nodes-1,1,.)
    edges = 0
    for (c=1; c<=cells; c++) {
        u = cell_worker[c]
        v = cell_firm[c]
        ru = u
        while (parent_set[ru] != ru) ru = parent_set[ru]
        rv = v
        while (parent_set[rv] != rv) rv = parent_set[rv]
        if (ru != rv) {
            edges++
            tree_u[edges] = u
            tree_v[edges] = v
            tree_r[edges] = 1/cell_weight[c]
            if (set_size[ru] < set_size[rv]) {
                parent_set[ru] = rv
                set_size[rv] = set_size[rv]+set_size[ru]
            }
            else {
                parent_set[rv] = ru
                set_size[ru] = set_size[ru]+set_size[rv]
            }
        }
    }
    if (edges != nodes-1) _error(3498,"two-FE tree certificate graph is disconnected")

    degree = J(nodes,1,0)
    for (c=1; c<=edges; c++) {
        degree[tree_u[c]] = degree[tree_u[c]]+1
        degree[tree_v[c]] = degree[tree_v[c]]+1
    }
    offset = J(nodes+1,1,1)
    for (u=1; u<=nodes; u++) offset[u+1] = offset[u]+degree[u]
    cursor = offset[(1::nodes)]
    neighbor = J(2*edges,1,.)
    edge_r = J(2*edges,1,.)
    for (c=1; c<=edges; c++) {
        u = tree_u[c]
        v = tree_v[c]
        pos = cursor[u]
        neighbor[pos] = v
        edge_r[pos] = tree_r[c]
        cursor[u] = cursor[u]+1
        pos = cursor[v]
        neighbor[pos] = u
        edge_r[pos] = tree_r[c]
        cursor[v] = cursor[v]+1
    }
    tree_parent = J(nodes,1,0)
    depth = J(nodes,1,0)
    prefix = J(nodes,1,0)
    queue = J(nodes,1,.)
    head = 1
    tail = 1
    queue[1] = 1
    tree_parent[1] = 1
    while (head <= tail) {
        u = queue[head]
        head = head+1
        left = offset[u]
        right = offset[u+1]-1
        for (pos=left; pos<=right; pos++) {
            child = neighbor[pos]
            if (tree_parent[child] == 0) {
                tree_parent[child] = u
                depth[child] = depth[u]+1
                prefix[child] = prefix[u]+edge_r[pos]
                tail = tail+1
                queue[tail] = child
            }
        }
    }
    if (tail != nodes) _error(3498,"tree certificate traversal is incomplete")
    levels = ceil(ln(nodes)/ln(2))+1
    up = J(nodes,levels,1)
    up[,1] = tree_parent
    for (level=2; level<=levels; level++) {
        up[,level] = up[up[,level-1],level-1]
    }
    cell_distance = J(cells,1,.)
    for (c=1; c<=cells; c++) {
        u = cell_worker[c]
        v = cell_firm[c]
        if (depth[u] < depth[v]) {
            pos = u
            u = v
            v = pos
        }
        delta = depth[u]-depth[v]
        for (level=levels; level>=1; level--) {
            if (delta >= 2^(level-1)) {
                u = up[u,level]
                delta = delta-2^(level-1)
            }
        }
        if (u == v) lca = u
        else {
            for (level=levels; level>=1; level--) {
                if (up[u,level] != up[v,level]) {
                    u = up[u,level]
                    v = up[v,level]
                }
            }
            lca = tree_parent[u]
        }
        cell_distance[c] = prefix[cell_worker[c]]+
            prefix[cell_firm[c]]-2*prefix[lca]
    }
    cell_index_sorted = J(rows(id),1,.)
    for (c=1; c<=cells; c++) {
        left = panel[c,1]
        right = panel[c,2]
        cell_index_sorted[(left::right)] = J(right-left+1,1,c)
    }
    row_distance = J(rows(id),1,.)
    row_distance[ord] = cell_distance[cell_index_sorted]
    out = deletion_weight:*row_distance
    if (hasmissing(out) | min(out) < -1e-12) {
        _error(3498,"tree leverage certificate is nonfinite")
    }
    return(out)
}

real colvector ppmltalo__twofe_exact_lev(
    struct ppmltalo_hdfe_design scalar D,
    real colvector deletion_weight)
{
    struct ppmltalo_scaled_inverse_result scalar information_inverse
    real matrix A
    real colvector out, worker_index, firm_index, use
    real scalar position, row

    if (D.k != 2 | D.q != 0 | rows(deletion_weight) != D.n |
        cols(deletion_weight) != 1 | hasmissing(deletion_weight) |
        min(deletion_weight) < 0) {
        _error(3200,"exact two-FE leverage inputs are invalid")
    }
    information_inverse = ppmltalo__scaled_sym_inverse(
        ppmltalo__hdfe_dense_H(D,D.mu),1e-9)
    if (information_inverse.status != "CONVERGED") {
        _error(3498,"exact two-FE information inversion failed")
    }
    A = information_inverse.inverse
    worker_index = D.id[,1]
    out = deletion_weight:*diagonal(A)[worker_index]
    use = D.firm_use
    if (rows(use)) {
        firm_index = D.firm_coef_index
        for (position=1; position<=rows(use); position++) {
            row = use[position]
            out[row] = out[row]+deletion_weight[row]*
                (A[firm_index[position],firm_index[position]]+
                 2*A[worker_index[row],firm_index[position]])
        }
    }
    return(out)
}

real colvector ppmltalo__joint_lev_cert(
    struct ppmltalo_joint_design scalar JD,
    real colvector deletion_weight)
{
    real colvector base

    if (JD.base.p <= 500) {
        base = ppmltalo__twofe_exact_lev(JD.base,deletion_weight)
    }
    else {
        base = ppmltalo__tree_lev_upper(JD.base.id,JD.base.mu,
            deletion_weight)
    }
    return(base+ppmltalo__joint_nuis_lev(JD,deletion_weight))
}

real colvector ppmltalo__joint_nuis_lev(
    struct ppmltalo_joint_design scalar JD,
    real colvector deletion_weight)
{
    real matrix L
    real colvector coefficient, transformed, out
    real scalar column

    if (rows(deletion_weight) != JD.full.n | cols(deletion_weight) != 1 |
        hasmissing(deletion_weight) | min(deletion_weight) < 0) {
        _error(3200,"nuisance leverage weights are invalid")
    }
    out = J(JD.full.n,1,0)
    if (JD.nuisance_parameters == 0) return(out)
    L = cholesky(0.5:*(JD.S_inverse+JD.S_inverse'))
    if (hasmissing(L)) _error(3498,"nuisance leverage factorization failed")
    for (column=1; column<=cols(L); column++) {
        coefficient = L[,column]
        transformed = ppmltalo__joint_nuisance_xm(JD,coefficient)-
            ppmltalo__hdfe_x(JD.base,JD.V*coefficient)
        out = out+transformed:^2
    }
    return(deletion_weight:*out)
}

struct ppmltalo_joint_design scalar ppmltalo__joint_build(
    real matrix id,
    real matrix controls,
    real colvector information_weight,
    real scalar tolerance,
    real scalar maxiter,
    real scalar batch_size,
    real scalar rank_tolerance)
{
    struct ppmltalo_joint_design scalar J
    struct ppmltalo_pcg_matrix_result scalar solve
    struct ppmltalo_scaled_inverse_result scalar small_inverse
    real matrix C, Z, HZ, quadratic, cross_term, HV, base_values, S
    real colvector take, conditional_share
    real scalar completed, actual, prep_tolerance, schur_gap

    if (cols(id) < 2) _error(3498,"joint information needs two target fixed effects")
    if (batch_size < 1 | batch_size != floor(batch_size)) {
        _error(3498,"joint information batch size must be a positive integer")
    }
    if (rank_tolerance <= 0 | rank_tolerance >= 1) {
        _error(3498,"joint information rank tolerance must lie in (0,1)")
    }
    J.full = ppmltalo__hdfe_build_x(id,controls,information_weight)
    J.base = ppmltalo__hdfe_build(id[,(1,2)],information_weight)
    J.nuisance_parameters = J.full.p-J.base.p
    J.V = J(J.base.p,J.nuisance_parameters,0)
    J.S_inverse = J(J.nuisance_parameters,J.nuisance_parameters,0)
    J.preparation_relres = 0
    J.small_inverse_relres = 0
    J.schur_rcond = .
    J.status = "CONVERGED"
    if (J.nuisance_parameters == 0) return(J)

    // The Schur preparation is reused by every later inverse action and by
    // the leverage certificate.  Solve it more tightly than a looser public
    // action tolerance, then compare two algebraically identical cross forms.
    prep_tolerance = min((tolerance,1e-12))
    HZ = J(J.nuisance_parameters,J.nuisance_parameters,0)
    for (completed=0; completed<J.nuisance_parameters; completed=completed+actual) {
        actual = min((batch_size,J.nuisance_parameters-completed))
        take = (completed+1::completed+actual)
        C = J(J.nuisance_parameters,actual,0)
        C[take,.] = I(actual)
        Z = ppmltalo__joint_nuisance_xm(J,C)
        solve = ppmltalo__hdfe_pcg_matrix(J.base,
            ppmltalo__hdfe_xtm(J.base,
                (information_weight*J(1,actual,1)):*Z),
            prep_tolerance,maxiter)
        if (solve.status != "CONVERGED") {
            J.status = "BASE_CROSS_"+solve.status
            J.preparation_relres = max(solve.relres)
            return(J)
        }
        J.V[,take] = solve.x
        J.preparation_relres = max((J.preparation_relres,max(solve.relres)))
        HZ[,take] = ppmltalo__joint_nuisance_xtm(J,
            (information_weight*J(1,actual,1)):*Z)
    }
    quadratic = J(J.nuisance_parameters,J.nuisance_parameters,0)
    cross_term = J(J.nuisance_parameters,J.nuisance_parameters,0)
    for (completed=0; completed<J.nuisance_parameters; completed=completed+actual) {
        actual = min((batch_size,J.nuisance_parameters-completed))
        take = (completed+1::completed+actual)
        HV = ppmltalo__hdfe_Hm(J.base,J.V[,take])
        quadratic[,take] = J.V'*HV
        base_values = ppmltalo__hdfe_xm(J.base,J.V[,take])
        cross_term[,take] = ppmltalo__joint_nuisance_xtm(J,
            (information_weight*J(1,actual,1)):*base_values)
    }
    schur_gap = ppmltalo__norm2(vec(cross_term-quadratic))/
        (1+ppmltalo__norm2(vec(cross_term))+
           ppmltalo__norm2(vec(quadratic)))
    J.preparation_relres = max((J.preparation_relres,schur_gap))
    if (hasmissing(schur_gap) |
        schur_gap > max((1e-10,100*prep_tolerance))) {
        J.status = "SCHUR_COHERENCE_FAILURE"
        return(J)
    }
    S = HZ-0.5:*(cross_term+cross_term')
    S = 0.5:*(S+S')
    conditional_share = diagonal(S):/diagonal(HZ)
    if (hasmissing(conditional_share) |
        min(conditional_share) <= rank_tolerance) {
        J.status = "NUISANCE_COLLINEAR"
        return(J)
    }
    small_inverse = ppmltalo__scaled_sym_inverse(S,max((1e-10,10*tolerance)))
    J.small_inverse_relres = small_inverse.relres
    J.schur_rcond = small_inverse.rcond
    if (small_inverse.status == "SINGULAR") {
        J.status = "NUISANCE_COLLINEAR"
        return(J)
    }
    if (small_inverse.status != "CONVERGED") {
        J.status = "NUISANCE_INVERSE_"+small_inverse.status
        return(J)
    }
    J.S_inverse = small_inverse.inverse
    return(J)
}

struct ppmltalo_pcg_matrix_result scalar ppmltalo__joint_pcg_matrix(
    struct ppmltalo_joint_design scalar J,
    real matrix B,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_pcg_matrix_result scalar out
    struct ppmltalo_pcg_matrix_result scalar base_solve
    real matrix BF, BZ, XF, XZ, residual
    real rowvector bnorm
    real scalar column

    if (J.status != "CONVERGED") {
        out.status = "PREPARATION_"+J.status
        out.x = J(J.full.p,cols(B),.)
        out.relres = J(1,cols(B),.)
        out.column_iterations = J(1,cols(B),0)
        out.column_status = J(1,cols(B),out.status)
        out.iterations = 0
        return(out)
    }
    if (rows(B) != J.full.p | cols(B) < 1 | hasmissing(B)) {
        _error(3200,"joint PCG right-hand sides are invalid")
    }
    BF = B[(1::J.base.p),.]
    base_solve = ppmltalo__hdfe_pcg_matrix(J.base,BF,tolerance,maxiter)
    if (base_solve.status != "CONVERGED") {
        out = base_solve
        out.status = "BASE_"+base_solve.status
        out.x = J(J.full.p,cols(B),.)
        return(out)
    }
    if (J.nuisance_parameters) {
        BZ = B[(J.base.p+1::J.full.p),.]
        XZ = J.S_inverse*(BZ-J.V'*BF)
        XF = base_solve.x-J.V*XZ
        out.x = XF \ XZ
    }
    else out.x = base_solve.x
    residual = B-ppmltalo__hdfe_Hm(J.full,out.x)
    bnorm = ppmltalo__colnorm2(B)
    bnorm = bnorm+(bnorm:==0)
    out.relres = ppmltalo__colnorm2(residual):/bnorm
    out.column_iterations = base_solve.column_iterations
    out.column_status = base_solve.column_status
    for (column=1; column<=cols(B); column++) {
        if (out.relres[column] <= tolerance) {
            out.column_status[column] = "CONVERGED_JOINT"
        }
        else out.column_status[column] = "FAILED_JOINT_RESIDUAL"
    }
    out.iterations = base_solve.iterations
    out.status = ppmltalo__pcg_matrix_status(out.column_status)
    if (hasmissing(out.x)) out.status = "NONFINITE_JOINT_SOLUTION"
    return(out)
}

struct ppmltalo_pcg_result scalar ppmltalo__joint_pcg(
    struct ppmltalo_joint_design scalar J,
    real colvector b,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_pcg_result scalar out
    struct ppmltalo_pcg_matrix_result scalar solve

    solve = ppmltalo__joint_pcg_matrix(J,b,tolerance,maxiter)
    out.x = solve.x
    out.relres = solve.relres[1]
    out.iterations = solve.iterations
    out.status = solve.column_status[1]
    return(out)
}

real colvector ppmltalo__hdfe_block_x(
    struct ppmltalo_hdfe_design scalar D,
    real colvector v,
    real scalar block)
{
    real colvector out, use, idx

    if (block < 1 | block > D.k | block != floor(block)) _error(3498, "invalid target block")
    if (rows(v) != D.p | cols(v) != 1) _error(3200, "target vector has wrong dimension")
    out = J(D.n, 1, 0)
    if (block == 1) {
        idx = D.offsets[block] :+ D.id[,block]
        out = v[idx]
    }
    else {
        if (D.k == 2 & block == 2) {
            use = D.firm_use
            idx = D.firm_coef_index
        }
        else {
            use = selectindex(D.id[,block] :< D.levels[block])
            idx = D.offsets[block] :+ D.id[use,block]
        }
        if (rows(use)) {
            out[use] = v[idx]
        }
    }
    return(out)
}

real matrix ppmltalo__hdfe_block_xm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V,
    real scalar block)
{
    real matrix out
    real colvector use, idx

    if (block < 1 | block > D.k | block != floor(block)) _error(3498, "invalid target block")
    if (rows(V) != D.p) _error(3200, "target matrix has wrong row dimension")
    out = J(D.n, cols(V), 0)
    if (block == 1) {
        idx = D.offsets[block] :+ D.id[,block]
        out = V[idx,.]
    }
    else {
        if (D.k == 2 & block == 2) {
            use = D.firm_use
            idx = D.firm_coef_index
        }
        else {
            use = selectindex(D.id[,block] :< D.levels[block])
            idx = D.offsets[block] :+ D.id[use,block]
        }
        if (rows(use)) {
            out[use,.] = V[idx,.]
        }
    }
    return(out)
}

real colvector ppmltalo__hdfe_block_xt(
    struct ppmltalo_hdfe_design scalar D,
    real colvector z,
    real scalar block)
{
    real colvector out, ord, sums, take
    real matrix info

    if (rows(z) != D.n | cols(z) != 1) _error(3200, "target row vector has wrong dimension")
    if (block < 1 | block > D.k | block != floor(block)) _error(3498, "invalid target block")
    out = J(D.p, 1, 0)
    ord = D.order[,block]
    sums = panelsum(z[ord], *D.panelinfo[block])
    take = (D.offsets[block]+1::D.offsets[block]+D.params[block])
    out[take] = sums[(1::D.params[block])]
    return(out)
}

real matrix ppmltalo__hdfe_block_xtm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix Z,
    real scalar block)
{
    real matrix out, sums
    real colvector ord, take

    if (rows(Z) != D.n) _error(3200, "target row matrix has wrong row dimension")
    if (block < 1 | block > D.k | block != floor(block)) _error(3498, "invalid target block")
    out = J(D.p, cols(Z), 0)
    ord = D.order[,block]
    sums = panelsum(Z[ord,.], *D.panelinfo[block])
    take = (D.offsets[block]+1::D.offsets[block]+D.params[block])
    out[take,.] = sums[(1::D.params[block]),.]
    return(out)
}

real colvector ppmltalo__target_var(
    struct ppmltalo_hdfe_design scalar D,
    real colvector v,
    real colvector pi,
    real scalar block)
{
    real colvector values
    real scalar total, mean

    if (rows(pi) != D.n | cols(pi) != 1 | min(pi) < 0 | hasmissing(pi)) {
        _error(3200, "target weights must be a finite nonnegative n by 1 vector")
    }
    total = sum(pi)
    if (total <= 0) _error(3498, "target weights have zero mass")
    values = ppmltalo__hdfe_block_x(D, v, block)
    mean = (pi' * values)[1,1] / total
    return(ppmltalo__hdfe_block_xt(D, (pi:/total) :* (values :- mean), block))
}

real matrix ppmltalo__target_varm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V,
    real colvector pi,
    real scalar block)
{
    real matrix values, centered
    real rowvector mean
    real scalar total

    if (rows(pi) != D.n | cols(pi) != 1 | min(pi) < 0 | hasmissing(pi)) {
        _error(3200, "target weights must be a finite nonnegative n by 1 vector")
    }
    total = sum(pi)
    if (total <= 0) _error(3498, "target weights have zero mass")
    values = ppmltalo__hdfe_block_xm(D, V, block)
    mean = (pi' * values) :/ total
    centered = values - J(D.n,1,1) * mean
    return(ppmltalo__hdfe_block_xtm(D,
        ((pi:/total)*J(1,cols(V),1)) :* centered, block))
}

real colvector ppmltalo__target_cov(
    struct ppmltalo_hdfe_design scalar D,
    real colvector v,
    real colvector pi,
    real scalar left_block,
    real scalar right_block)
{
    real colvector left, right, out
    real scalar total, left_mean, right_mean

    if (left_block == right_block) {
        return(ppmltalo__target_var(D, v, pi, left_block))
    }
    if (rows(pi) != D.n | cols(pi) != 1 | min(pi) < 0 | hasmissing(pi)) {
        _error(3200, "target weights must be a finite nonnegative n by 1 vector")
    }
    total = sum(pi)
    if (total <= 0) _error(3498, "target weights have zero mass")
    left = ppmltalo__hdfe_block_x(D, v, left_block)
    right = ppmltalo__hdfe_block_x(D, v, right_block)
    left_mean = (pi' * left)[1,1] / total
    right_mean = (pi' * right)[1,1] / total
    out = 0.5 :* ppmltalo__hdfe_block_xt(
        D, (pi:/total) :* (right :- right_mean), left_block)
    out = out + 0.5 :* ppmltalo__hdfe_block_xt(
        D, (pi:/total) :* (left :- left_mean), right_block)
    return(out)
}

real matrix ppmltalo__target_covm(
    struct ppmltalo_hdfe_design scalar D,
    real matrix V,
    real colvector pi,
    real scalar left_block,
    real scalar right_block)
{
    real matrix left, right, out, weight_matrix
    real rowvector left_mean, right_mean
    real scalar total

    if (left_block == right_block) {
        return(ppmltalo__target_varm(D, V, pi, left_block))
    }
    if (rows(pi) != D.n | cols(pi) != 1 | min(pi) < 0 | hasmissing(pi)) {
        _error(3200, "target weights must be a finite nonnegative n by 1 vector")
    }
    total = sum(pi)
    if (total <= 0) _error(3498, "target weights have zero mass")
    left = ppmltalo__hdfe_block_xm(D, V, left_block)
    right = ppmltalo__hdfe_block_xm(D, V, right_block)
    left_mean = (pi' * left) :/ total
    right_mean = (pi' * right) :/ total
    weight_matrix = (pi:/total) * J(1,cols(V),1)
    out = 0.5 :* ppmltalo__hdfe_block_xtm(D,
        weight_matrix :* (right - J(D.n,1,1)*right_mean), left_block)
    out = out + 0.5 :* ppmltalo__hdfe_block_xtm(D,
        weight_matrix :* (left - J(D.n,1,1)*left_mean), right_block)
    return(out)
}

real colvector ppmltalo__rademacher(real scalar n)
{
    return(2 :* (runiform(n, 1) :>= 0.5) :- 1)
}

real matrix ppmltalo__rademacher_batch(real scalar n, real scalar batch_size)
{
    real matrix out
    real scalar column

    out = J(n, batch_size, .)
    for (column=1; column<=batch_size; column++) {
        out[,column] = ppmltalo__rademacher(n)
    }
    return(out)
}

struct ppmltalo_leverage_result scalar ppmltalo__leverage_sketch(
    struct ppmltalo_hdfe_design scalar D,
    real scalar probes,
    real scalar batch_size,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_leverage_result scalar out
    struct ppmltalo_pcg_matrix_result scalar solve
    real matrix z, rhs, projected
    real colvector mean, M2, delta
    real scalar draw, completed, actual, column

    if (probes < 2 | probes != floor(probes)) _error(3498, "leverage probes must be an integer of at least two")
    if (batch_size < 1 | batch_size != floor(batch_size)) _error(3498, "batch size must be a positive integer")
    rseed(seed)
    mean = J(D.n, 1, 0)
    M2 = J(D.n, 1, 0)
    out.max_solve_relres = 0
    out.max_solve_iterations = 0
    out.status = "CONVERGED"
    draw = 0
    for (completed=0; completed<probes; completed=completed+actual) {
        actual = min((batch_size, probes-completed))
        z = ppmltalo__rademacher_batch(D.n, actual)
        rhs = ppmltalo__hdfe_xtm(D,
            (sqrt(D.mu)*J(1,actual,1)) :* z)
        solve = ppmltalo__hdfe_pcg_matrix(D, rhs, tolerance, maxiter)
        if (solve.status != "CONVERGED") {
            out.status = "SOLVE_" + solve.status
            out.max_solve_relres = max(solve.relres)
            out.max_solve_iterations = solve.iterations
            out.leverage = J(D.n, 1, .)
            out.mcse = J(D.n, 1, .)
            out.probes = completed
            return(out)
        }
        out.max_solve_relres = max((out.max_solve_relres, max(solve.relres)))
        out.max_solve_iterations = max((out.max_solve_iterations, solve.iterations))
        projected = (sqrt(D.mu)*J(1,actual,1)) :*
            ppmltalo__hdfe_xm(D, solve.x)
        projected = projected :^ 2
        for (column=1; column<=actual; column++) {
            draw++
            delta = projected[,column] :- mean
            mean = mean + delta :/ draw
            M2 = M2 + delta :* (projected[,column] :- mean)
        }
    }
    out.leverage = mean
    out.mcse = sqrt((M2 :/ (probes-1)) :/ probes)
    out.probes = probes
    if (hasmissing(out.leverage) | hasmissing(out.mcse) |
        hasmissing(out.max_solve_relres) |
        hasmissing(out.max_solve_iterations)) {
        out.status = "NONFINITE"
    }
    return(out)
}

struct ppmltalo_correction_result scalar ppmltalo__row_correction(
    struct ppmltalo_hdfe_design scalar D,
    real colvector y,
    real colvector beta,
    real colvector eta,
    real colvector pi,
    real colvector leverage,
    real scalar worker_block,
    real scalar firm_block,
    real scalar probes,
    real scalar batch_size,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar leverage_limit)
{
    struct ppmltalo_correction_result scalar out
    struct ppmltalo_pcg_result scalar solve_s
    struct ppmltalo_pcg_matrix_result scalar solve_probe
    real colvector residual, shift, moment
    real colvector qbeta_w, qbeta_f, qbeta_c, d_w, d_f, d_c
    real matrix z, kz, rhs, U, V, xu, xv, crossrow
    real matrix qv_w, qv_f, qv_c, values
    real rowvector mean, M2, delta
    real scalar draw, completed, actual, column

    if (rows(y) != D.n | cols(y) != 1 | hasmissing(y) | min(y) < 0) {
        _error(3200, "outcome must be a finite nonnegative n by 1 vector")
    }
    if (rows(beta) != D.p | cols(beta) != 1 | hasmissing(beta)) {
        _error(3200, "fitted coefficient vector has wrong dimension")
    }
    if (rows(eta) != D.n | cols(eta) != 1 | hasmissing(eta)) {
        _error(3200, "fitted index vector has wrong dimension")
    }
    if (rows(leverage) != D.n | cols(leverage) != 1 | hasmissing(leverage)) {
        _error(3200, "leverage vector has wrong dimension")
    }
    if (min(leverage) < 0 | max(leverage) >= leverage_limit) {
        _error(3498, "leverage violates the configured safety limit")
    }
    if (probes < 2 | probes != floor(probes)) _error(3498, "correction probes must be an integer of at least two")
    if (batch_size < 1 | batch_size != floor(batch_size)) _error(3498, "batch size must be a positive integer")

    residual = y :- D.mu
    shift = -(leverage :/ (1 :- leverage)) :* (residual :/ D.mu)
    out.deleted_mu = exp(eta + shift)
    if (hasmissing(out.deleted_mu) | min(out.deleted_mu) <= 0) {
        _error(3498, "analytic deleted prediction is nonfinite")
    }
    moment = y :* (y :- out.deleted_mu)

    qbeta_w = ppmltalo__target_var(D, beta, pi, worker_block)
    qbeta_f = ppmltalo__target_var(D, beta, pi, firm_block)
    qbeta_c = ppmltalo__target_cov(D, beta, pi, worker_block, firm_block)
    solve_s = ppmltalo__hdfe_pcg(D, qbeta_w, tolerance, maxiter)
    if (solve_s.status != "CONVERGED" & solve_s.status != "CONVERGED_ZERO_RHS") {
        out.status = "WORKER_TARGET_" + solve_s.status
        out.max_solve_relres = solve_s.relres
        out.max_solve_iterations = solve_s.iterations
        return(out)
    }
    d_w = D.mu :* ppmltalo__hdfe_x(D, solve_s.x)
    out.max_solve_relres = solve_s.relres
    out.max_solve_iterations = solve_s.iterations
    solve_s = ppmltalo__hdfe_pcg(D, qbeta_f, tolerance, maxiter)
    if (solve_s.status != "CONVERGED" & solve_s.status != "CONVERGED_ZERO_RHS") {
        out.status = "FIRM_TARGET_" + solve_s.status
        out.max_solve_relres = max((out.max_solve_relres,
            solve_s.relres))
        out.max_solve_iterations = max((out.max_solve_iterations,
            solve_s.iterations))
        return(out)
    }
    d_f = D.mu :* ppmltalo__hdfe_x(D, solve_s.x)
    out.max_solve_relres = max((out.max_solve_relres, solve_s.relres))
    out.max_solve_iterations = max((out.max_solve_iterations, solve_s.iterations))
    solve_s = ppmltalo__hdfe_pcg(D, qbeta_c, tolerance, maxiter)
    if (solve_s.status != "CONVERGED" & solve_s.status != "CONVERGED_ZERO_RHS") {
        out.status = "COVARIANCE_TARGET_" + solve_s.status
        out.max_solve_relres = max((out.max_solve_relres,
            solve_s.relres))
        out.max_solve_iterations = max((out.max_solve_iterations,
            solve_s.iterations))
        return(out)
    }
    d_c = D.mu :* ppmltalo__hdfe_x(D, solve_s.x)
    out.max_solve_relres = max((out.max_solve_relres, solve_s.relres))
    out.max_solve_iterations = max((out.max_solve_iterations, solve_s.iterations))

    out.plugin = ((beta' * qbeta_w)[1,1],
                  (beta' * qbeta_f)[1,1],
                  (beta' * qbeta_c)[1,1], 0)
    out.plugin[4] = out.plugin[1] + out.plugin[2] + 2*out.plugin[3]

    rseed(seed)
    mean = J(1, 4, 0)
    M2 = J(1, 4, 0)
    out.status = "CONVERGED"
    draw = 0
    for (completed=0; completed<probes; completed=completed+actual) {
        actual = min((batch_size, probes-completed))
        z = ppmltalo__rademacher_batch(D.n, actual)
        kz = (moment*J(1,actual,1)) :* z
        rhs = ppmltalo__hdfe_xtm(D, (z, kz))
        solve_probe = ppmltalo__hdfe_pcg_matrix(D, rhs, tolerance, maxiter)
        if (solve_probe.status != "CONVERGED") {
            out.status = "PROBE_" + solve_probe.status
            out.max_solve_relres = max((out.max_solve_relres,
                max(solve_probe.relres)))
            out.max_solve_iterations = max((out.max_solve_iterations,
                solve_probe.iterations))
            out.probes = completed
            out.correction = J(1,4,.)
            out.mcse = J(1,4,.)
            out.talo = J(1,4,.)
            return(out)
        }
        out.max_solve_relres = max((out.max_solve_relres, max(solve_probe.relres)))
        out.max_solve_iterations = max((out.max_solve_iterations, solve_probe.iterations))
        U = solve_probe.x[,(1::actual)]
        V = solve_probe.x[,(actual+1::2*actual)]
        xu = ppmltalo__hdfe_xm(D, U)
        xv = ppmltalo__hdfe_xm(D, V)
        crossrow = xu :* xv
        qv_w = ppmltalo__target_varm(D, V, pi, worker_block)
        qv_f = ppmltalo__target_varm(D, V, pi, firm_block)
        qv_c = ppmltalo__target_covm(D, V, pi, worker_block, firm_block)
        values = J(actual,4,0)
        values[,1] = (colsum(U:*qv_w) :-
            colsum((d_w*J(1,actual,1)):*crossrow))'
        values[,2] = (colsum(U:*qv_f) :-
            colsum((d_f*J(1,actual,1)):*crossrow))'
        values[,3] = (colsum(U:*qv_c) :-
            colsum((d_c*J(1,actual,1)):*crossrow))'
        values[,4] = values[,1] + values[,2] + 2:*values[,3]
        for (column=1; column<=actual; column++) {
            draw++
            delta = values[column,.] - mean
            mean = mean + delta :/ draw
            M2 = M2 + delta :* (values[column,.] - mean)
        }
    }
    out.correction = mean
    out.mcse = sqrt((M2 :/ (probes-1)) :/ probes)
    out.correction[4] = out.correction[1]+out.correction[2]+2*out.correction[3]
    out.talo = out.plugin - out.correction
    out.talo[4] = out.talo[1]+out.talo[2]+2*out.talo[3]
    out.probes = probes
    out.max_leverage = max(leverage)
    if (hasmissing(out.plugin) | hasmissing(out.correction) |
        hasmissing(out.mcse) | hasmissing(out.talo) |
        hasmissing(out.deleted_mu) | hasmissing(out.max_leverage) |
        hasmissing(out.max_solve_relres) |
        hasmissing(out.max_solve_iterations)) {
        out.status = "TRACE_NONFINITE"
    }
    return(out)
}

struct ppmltalo_leverage_result scalar ppmltalo__joint_leverage_sketch(
    struct ppmltalo_joint_design scalar JD,
    real colvector deletion_weight,
    real scalar probes,
    real scalar batch_size,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter)
{
    struct ppmltalo_leverage_result scalar out
    struct ppmltalo_pcg_matrix_result scalar solve
    real matrix z, rhs, projected
    real colvector mean, M2, delta
    real scalar draw, completed, actual, column

    if (rows(deletion_weight) != JD.full.n | cols(deletion_weight) != 1 |
        hasmissing(deletion_weight) | min(deletion_weight) <= 0 |
        max(deletion_weight:-JD.full.mu) > 1e-12*max((1,max(JD.full.mu)))) {
        _error(3200,"deletion curvature must be positive and no larger than information curvature")
    }
    if (probes < 2 | probes != floor(probes)) {
        _error(3498,"leverage probes must be an integer of at least two")
    }
    if (batch_size < 1 | batch_size != floor(batch_size)) {
        _error(3498,"batch size must be a positive integer")
    }
    rseed(seed)
    mean = J(JD.full.n,1,0)
    M2 = J(JD.full.n,1,0)
    out.max_solve_relres = max((JD.preparation_relres,JD.small_inverse_relres))
    out.max_solve_iterations = 0
    out.status = "CONVERGED"
    draw = 0
    for (completed=0; completed<probes; completed=completed+actual) {
        actual = min((batch_size,probes-completed))
        z = ppmltalo__rademacher_batch(JD.full.n,actual)
        rhs = ppmltalo__hdfe_xtm(JD.full,
            (sqrt(JD.full.mu)*J(1,actual,1)):*z)
        solve = ppmltalo__joint_pcg_matrix(JD,rhs,tolerance,maxiter)
        if (solve.status != "CONVERGED") {
            out.status = "SOLVE_"+solve.status
            out.max_solve_relres = max((out.max_solve_relres,max(solve.relres)))
            out.max_solve_iterations = solve.iterations
            out.leverage = J(JD.full.n,1,.)
            out.mcse = J(JD.full.n,1,.)
            out.probes = completed
            return(out)
        }
        out.max_solve_relres = max((out.max_solve_relres,max(solve.relres)))
        out.max_solve_iterations = max((out.max_solve_iterations,solve.iterations))
        projected = (sqrt(JD.full.mu)*J(1,actual,1)):*
            ppmltalo__hdfe_xm(JD.full,solve.x)
        projected = (deletion_weight:/JD.full.mu)*J(1,actual,1):*
            (projected:^2)
        for (column=1; column<=actual; column++) {
            draw++
            delta = projected[,column]-mean
            mean = mean+delta:/draw
            M2 = M2+delta:*(projected[,column]-mean)
        }
    }
    out.leverage = mean
    out.mcse = sqrt((M2:/(probes-1)):/probes)
    out.probes = probes
    if (hasmissing(out.leverage) | hasmissing(out.mcse) |
        hasmissing(out.max_solve_relres) |
        hasmissing(out.max_solve_iterations)) {
        out.status = "NONFINITE"
    }
    return(out)
}

struct ppmltalo_correction_result scalar ppmltalo__joint_row_correction(
    struct ppmltalo_joint_design scalar JD,
    real colvector y,
    real colvector fit_mu,
    real colvector frequency,
    real colvector beta,
    real colvector eta,
    real colvector pi,
    real colvector leverage,
    real scalar worker_block,
    real scalar firm_block,
    real scalar probes,
    real scalar batch_size,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar leverage_limit)
{
    struct ppmltalo_correction_result scalar out
    struct ppmltalo_pcg_result scalar solve_s
    struct ppmltalo_pcg_matrix_result scalar solve_probe
    struct ppmltalo_hdfe_design scalar D
    real colvector residual, shift, moment
    real colvector qbeta_w, qbeta_f, qbeta_c, d_w, d_f, d_c
    real matrix z, kz, rhs, U, V, xu, xv, crossrow
    real matrix qv_w, qv_f, qv_c, values
    real rowvector mean, M2, delta
    real scalar draw, completed, actual, column

    D = JD.full
    if (rows(y) != D.n | rows(fit_mu) != D.n | rows(frequency) != D.n |
        cols(y) != 1 | cols(fit_mu) != 1 | cols(frequency) != 1 |
        hasmissing(y) | hasmissing(fit_mu) | hasmissing(frequency) |
        min(y) < 0 | min(fit_mu) <= 0 | min(frequency) < 1 |
        max(abs(frequency-floor(frequency))) != 0) {
        _error(3200,"joint observation inputs are invalid")
    }
    if (rows(beta) != D.p | cols(beta) != 1 | hasmissing(beta)) {
        _error(3200,"fitted coefficient vector has wrong dimension")
    }
    if (rows(eta) != D.n | cols(eta) != 1 | hasmissing(eta)) {
        _error(3200,"fitted index vector has wrong dimension")
    }
    if (rows(leverage) != D.n | cols(leverage) != 1 | hasmissing(leverage)) {
        _error(3200,"leverage vector has wrong dimension")
    }
    if (min(leverage) < 0 | max(leverage) >= leverage_limit) {
        _error(3498,"leverage violates the configured safety limit")
    }
    if (probes < 2 | probes != floor(probes)) {
        _error(3498,"correction probes must be an integer of at least two")
    }
    residual = y-fit_mu
    shift = -(leverage:/(1:-leverage)):*(residual:/fit_mu)
    out.deleted_mu = exp(eta+shift)
    if (hasmissing(out.deleted_mu) | min(out.deleted_mu) <= 0) {
        _error(3498,"analytic deleted prediction is nonfinite")
    }
    moment = frequency:*y:*(y-out.deleted_mu)

    qbeta_w = ppmltalo__target_var(D,beta,pi,worker_block)
    qbeta_f = ppmltalo__target_var(D,beta,pi,firm_block)
    qbeta_c = ppmltalo__target_cov(D,beta,pi,worker_block,firm_block)
    solve_s = ppmltalo__joint_pcg(JD,qbeta_w,tolerance,maxiter)
    if (!ppmltalo__pcg_status_ok(solve_s.status)) {
        out.status = "WORKER_TARGET_"+solve_s.status
        out.max_solve_relres = solve_s.relres
        out.max_solve_iterations = solve_s.iterations
        return(out)
    }
    d_w = D.mu:*ppmltalo__hdfe_x(D,solve_s.x)
    out.max_solve_relres = max((JD.preparation_relres,
        JD.small_inverse_relres,solve_s.relres))
    out.max_solve_iterations = solve_s.iterations
    solve_s = ppmltalo__joint_pcg(JD,qbeta_f,tolerance,maxiter)
    if (!ppmltalo__pcg_status_ok(solve_s.status)) {
        out.status = "FIRM_TARGET_"+solve_s.status
        out.max_solve_relres = max((out.max_solve_relres,solve_s.relres))
        out.max_solve_iterations = max((out.max_solve_iterations,solve_s.iterations))
        return(out)
    }
    d_f = D.mu:*ppmltalo__hdfe_x(D,solve_s.x)
    out.max_solve_relres = max((out.max_solve_relres,solve_s.relres))
    out.max_solve_iterations = max((out.max_solve_iterations,solve_s.iterations))
    solve_s = ppmltalo__joint_pcg(JD,qbeta_c,tolerance,maxiter)
    if (!ppmltalo__pcg_status_ok(solve_s.status)) {
        out.status = "COVARIANCE_TARGET_"+solve_s.status
        out.max_solve_relres = max((out.max_solve_relres,solve_s.relres))
        out.max_solve_iterations = max((out.max_solve_iterations,solve_s.iterations))
        return(out)
    }
    d_c = D.mu:*ppmltalo__hdfe_x(D,solve_s.x)
    out.max_solve_relres = max((out.max_solve_relres,solve_s.relres))
    out.max_solve_iterations = max((out.max_solve_iterations,solve_s.iterations))

    out.plugin = ((beta'*qbeta_w)[1,1],(beta'*qbeta_f)[1,1],
        (beta'*qbeta_c)[1,1],0)
    out.plugin[4] = out.plugin[1]+out.plugin[2]+2*out.plugin[3]
    rseed(seed)
    mean = J(1,4,0)
    M2 = J(1,4,0)
    out.status = "CONVERGED"
    draw = 0
    for (completed=0; completed<probes; completed=completed+actual) {
        actual = min((batch_size,probes-completed))
        z = ppmltalo__rademacher_batch(D.n,actual)
        kz = (moment*J(1,actual,1)):*z
        rhs = ppmltalo__hdfe_xtm(D,(z,kz))
        solve_probe = ppmltalo__joint_pcg_matrix(JD,rhs,tolerance,maxiter)
        if (solve_probe.status != "CONVERGED") {
            out.status = "PROBE_"+solve_probe.status
            out.max_solve_relres = max((out.max_solve_relres,max(solve_probe.relres)))
            out.max_solve_iterations = max((out.max_solve_iterations,solve_probe.iterations))
            out.probes = completed
            out.correction = J(1,4,.)
            out.mcse = J(1,4,.)
            out.talo = J(1,4,.)
            return(out)
        }
        out.max_solve_relres = max((out.max_solve_relres,max(solve_probe.relres)))
        out.max_solve_iterations = max((out.max_solve_iterations,solve_probe.iterations))
        U = solve_probe.x[,(1::actual)]
        V = solve_probe.x[,(actual+1::2*actual)]
        xu = ppmltalo__hdfe_xm(D,U)
        xv = ppmltalo__hdfe_xm(D,V)
        crossrow = xu:*xv
        qv_w = ppmltalo__target_varm(D,V,pi,worker_block)
        qv_f = ppmltalo__target_varm(D,V,pi,firm_block)
        qv_c = ppmltalo__target_covm(D,V,pi,worker_block,firm_block)
        values = J(actual,4,0)
        values[,1] = (colsum(U:*qv_w)-
            colsum((d_w*J(1,actual,1)):*crossrow))'
        values[,2] = (colsum(U:*qv_f)-
            colsum((d_f*J(1,actual,1)):*crossrow))'
        values[,3] = (colsum(U:*qv_c)-
            colsum((d_c*J(1,actual,1)):*crossrow))'
        values[,4] = values[,1]+values[,2]+2:*values[,3]
        for (column=1; column<=actual; column++) {
            draw++
            delta = values[column,.]-mean
            mean = mean+delta:/draw
            M2 = M2+delta:*(values[column,.]-mean)
        }
    }
    out.correction = mean
    out.mcse = sqrt((M2:/(probes-1)):/probes)
    out.correction[4] = out.correction[1]+out.correction[2]+2*out.correction[3]
    out.talo = out.plugin-out.correction
    out.talo[4] = out.talo[1]+out.talo[2]+2*out.talo[3]
    out.probes = probes
    out.max_leverage = max(leverage)
    if (hasmissing(out.plugin) | hasmissing(out.correction) |
        hasmissing(out.mcse) | hasmissing(out.talo) |
        hasmissing(out.deleted_mu) | hasmissing(out.max_leverage) |
        hasmissing(out.max_solve_relres) |
        hasmissing(out.max_solve_iterations)) {
        out.status = "TRACE_NONFINITE"
    }
    return(out)
}

struct ppmltalo_local_design scalar ppmltalo__block_local_design(
    struct ppmltalo_hdfe_design scalar D,
    real colvector idx)
{
    struct ppmltalo_local_design scalar out
    real colvector observed
    real matrix block_X
    real scalar block, level, control

    out.index = J(0,1,.)
    out.X = J(rows(idx),0,.)
    for (block=1; block<=D.k; block++) {
        observed = uniqrows(sort(D.id[idx,block],1))
        if (block > 1) observed = select(observed,
            observed :< D.levels[block])
        if (rows(observed)) {
            block_X = J(rows(idx),rows(observed),0)
            for (level=1; level<=rows(observed); level++) {
                block_X[,level] = (D.id[idx,block] :== observed[level])
            }
            out.X = out.X,block_X
            out.index = out.index \ (D.offsets[block] :+ observed)
        }
    }
    for (control=1; control<=D.q; control++) {
        if (max(abs(D.controls[idx,control])) > 0) {
            out.X = out.X,D.controls[idx,control]
            out.index = out.index \ (D.control_offset+control)
        }
    }
    return(out)
}

real matrix ppmltalo__match_Km(
    real matrix V,
    real colvector y_mass,
    real colvector e_mass,
    real colvector match_order,
    real matrix match_panel)
{
    real matrix out
    real colvector idx
    real scalar panel, left, right

    if (rows(V) != rows(y_mass) | rows(V) != rows(e_mass)) {
        _error(3200,"match-kernel action has incompatible dimensions")
    }
    out = J(rows(V),cols(V),0)
    for (panel=1; panel<=rows(match_panel); panel++) {
        left = match_panel[panel,1]
        right = match_panel[panel,2]
        idx = match_order[(left::right)]
        out[idx,.] = 0.5:*(e_mass[idx]*(y_mass[idx]'*V[idx,.]) +
            y_mass[idx]*(e_mass[idx]'*V[idx,.]))
    }
    return(out)
}

struct ppmltalo_match_result scalar ppmltalo__block_dense(
    struct ppmltalo_hdfe_design scalar D,
    real colvector y,
    real colvector fit_mu,
    real colvector frequency,
    real colvector match,
    real colvector beta,
    real colvector eta,
    real colvector pi,
    real scalar exact_trace,
    real scalar probes,
    real scalar batch_size,
    real scalar seed,
    real scalar block_limit,
    real scalar rank_tolerance,
    real scalar one_copy,
    real scalar require_constant,
    real scalar block_size_limit,
    real scalar local_dimension_limit,
    string scalar status_prefix)
{
    struct ppmltalo_match_result scalar out
    struct ppmltalo_scaled_inverse_result scalar information_inverse
    struct ppmltalo_scaled_inverse_result scalar positive_inverse
    struct ppmltalo_scaled_inverse_result scalar local_kernel_inverse
    struct ppmltalo_scaled_inverse_result scalar local_system_inverse
    real matrix H, A, Hpositive, Apositive, match_panel
    struct ppmltalo_local_design scalar local_design
    real matrix local_X, K, Kpositive, Z, Zpositive, R, Rpositive
    real matrix Kscaled, Kpositive_scaled
    real matrix small_system, z, kz, rhs, U, V, xu, xv, basis
    real matrix qv_w, qv_f, qv_c, values
    real colvector match_order, idx, residual, ac, b, gamma, delta
    real colvector local_scale, positive_scale, deletion_weight
    real colvector positive_deletion_weight
    real colvector y_mass, e_mass, qbeta_w, qbeta_f, qbeta_c
    real colvector d_w, d_f, d_c, mean_vec, M2_vec, delta_vec
    real colvector expected_mu, expected_eta
    real rowvector eig, eigpositive, mean, M2, update
    real matrix correction_state
    real scalar panel, left, right, m, local_rank, eigmax, eigmaxpositive
    real scalar completed, actual, column, draw

    out.status = "INVALID_INPUT"
    out.blocks = 0
    out.expanded_n = .
    out.max_block_patterns = 0
    out.max_local_rank = 0
    out.max_block_eigen = 0
    out.max_positive_block_eigen = 0
    out.information_inverse_relres = .
    out.positive_inverse_relres = .
    out.information_rcond = .
    out.positive_rcond = .
    out.min_local_kernel_rcond = .
    out.min_local_system_rcond = .
    out.trace_directions = 0
    out.failure_block_index = .
    out.failure_stage = 0
    out.failure_metric_min = .
    out.failure_metric_max = .
    if (D.k < 2) return(out)
    if (rows(y) != D.n | rows(fit_mu) != D.n | rows(frequency) != D.n |
        rows(match) != D.n | rows(eta) != D.n | rows(pi) != D.n |
        rows(beta) != D.p | cols(y) != 1 | cols(fit_mu) != 1 |
        cols(frequency) != 1 | cols(match) != 1 | cols(eta) != 1 |
        cols(pi) != 1 | cols(beta) != 1 | rows(D.mu) != D.n |
        cols(D.mu) != 1 | hasmissing(y) |
        hasmissing(fit_mu) | hasmissing(frequency) | hasmissing(match) |
        hasmissing(beta) | hasmissing(eta) | hasmissing(pi) |
        hasmissing(D.mu) |
        min(y) < 0 | min(fit_mu) <= 0 | min(frequency) <= 0 |
        max(abs(frequency-floor(frequency))) != 0 | min(pi) < 0 |
        sum(pi) <= 0 | min(D.mu) <= 0) return(out)
    if (!anyof((0,1),exact_trace) | probes < 1 | probes != floor(probes) |
        (!exact_trace & probes < 2) |
        batch_size < 1 |
        batch_size != floor(batch_size) | block_limit <= 0 |
        block_limit >= 1 | rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        !anyof((0,1),one_copy) | !anyof((0,1),require_constant) |
        block_size_limit < 1 | local_dimension_limit < 1 |
        !anyof(("MATCH","CLUSTER","OBSERVATION"),status_prefix)) {
        return(out)
    }
    expected_mu = frequency:*fit_mu
    if (ppmltalo__norm2(D.mu-expected_mu) /
        (1+ppmltalo__norm2(D.mu)+ppmltalo__norm2(expected_mu)) > 1e-12) {
        out.status = status_prefix+"_WEIGHT_COHERENCE_FAILURE"
        return(out)
    }
    expected_eta = log(fit_mu)
    if (ppmltalo__norm2(eta-expected_eta) /
        (1+ppmltalo__norm2(eta)+ppmltalo__norm2(expected_eta)) > 1e-12) {
        out.status = status_prefix+"_LINEAR_PREDICTOR_COHERENCE_FAILURE"
        return(out)
    }
    H = ppmltalo__hdfe_dense_H(D,D.mu)
    information_inverse = ppmltalo__scaled_sym_inverse(H,1e-8)
    if (information_inverse.status == "SINGULAR") {
        out.status = "FULL_INFORMATION_SINGULAR"
        return(out)
    }
    if (information_inverse.status != "CONVERGED") {
        out.status = "FULL_INFORMATION_INVERSE_FAILED"
        return(out)
    }
    A = information_inverse.inverse
    out.information_inverse_relres = information_inverse.relres
    out.information_rcond = information_inverse.rcond
    Hpositive = ppmltalo__hdfe_dense_H(D,frequency:*y)
    positive_inverse = ppmltalo__scaled_sym_inverse(Hpositive,1e-8)
    if (positive_inverse.status == "SINGULAR") {
        out.status = "POSITIVE_INFORMATION_SINGULAR"
        return(out)
    }
    if (positive_inverse.status != "CONVERGED") {
        out.status = "POSITIVE_INFORMATION_INVERSE_FAILED"
        return(out)
    }
    Apositive = positive_inverse.inverse
    out.positive_inverse_relres = positive_inverse.relres
    out.positive_rcond = positive_inverse.rcond
    residual = y:-fit_mu
    out.deleted_mu = J(D.n,1,.)
    out.expanded_n = sum(frequency)
    match_order = order(match,1)
    match_panel = panelsetup(match[match_order],1)
    out.blocks = rows(match_panel)
    for (panel=1; panel<=rows(match_panel); panel++) {
        left = match_panel[panel,1]
        right = match_panel[panel,2]
        idx = match_order[(left::right)]
        if (require_constant &
            (min(D.id[idx,1]) != max(D.id[idx,1]) |
             min(D.id[idx,2]) != max(D.id[idx,2]))) {
            out.status = status_prefix+"_IDENTIFIER_INCONSISTENT"
            return(out)
        }
        m = rows(idx)
        out.max_block_patterns = max((out.max_block_patterns,m))
        if (m > block_size_limit) {
            out.status = status_prefix+"_BLOCK_SIZE_LIMIT"
            return(out)
        }
        local_design = ppmltalo__block_local_design(D,idx)
        local_X = local_design.X
        if (cols(local_X) > local_dimension_limit) {
            out.status = status_prefix+"_LOCAL_DIMENSION_LIMIT"
            return(out)
        }
        K = A[local_design.index,local_design.index]
        K = 0.5:*(K+K')
        if (hasmissing(K) | min(diagonal(K)) <= 0) {
            out.status = "LOCAL_KERNEL_SINGULAR"
            return(out)
        }
        local_scale = sqrt(diagonal(K))
        Kscaled = ((1:/local_scale)*(1:/local_scale)'):*K
        local_kernel_inverse = ppmltalo__scaled_sym_inverse(Kscaled,1e-8)
        if (local_kernel_inverse.status != "CONVERGED") {
            out.status = "LOCAL_KERNEL_SINGULAR"
            return(out)
        }
        if (missing(out.min_local_kernel_rcond)) {
            out.min_local_kernel_rcond = local_kernel_inverse.rcond
        }
        else out.min_local_kernel_rcond = min((out.min_local_kernel_rcond,
            local_kernel_inverse.rcond))
        if (one_copy) deletion_weight = fit_mu[idx]
        else deletion_weight = frequency[idx]:*fit_mu[idx]
        Z = (sqrt(deletion_weight)*J(1,cols(local_X),1)):*local_X
        Z = Z:*(J(rows(Z),1,1)*local_scale')
        local_rank = rank(Z)
        out.max_local_rank = max((out.max_local_rank,local_rank))
        R = Z'*Z
        eig = Re(eigenvalues(Kscaled*R))'
        eigmax = max(eig)
        if (min(eig) < -rank_tolerance | eigmax >= 1-rank_tolerance) {
            out.failure_block_index = panel
            out.failure_stage = 4
            out.failure_metric_min = min(eig)
            out.failure_metric_max = eigmax
            if (one_copy) out.status = "ROW_DELETION_RANK_FAILURE"
            else out.status = status_prefix+"_DELETION_RANK_FAILURE"
            return(out)
        }
        out.max_block_eigen = max((out.max_block_eigen,eigmax))
        if (eigmax >= block_limit) {
            out.failure_block_index = panel
            out.failure_stage = 5
            out.failure_metric_min = min(eig)
            out.failure_metric_max = eigmax
            if (one_copy) out.status = "LEVERAGE_LIMIT"
            else out.status = status_prefix+"_BLOCK_LIMIT"
            return(out)
        }
        Kpositive = Apositive[local_design.index,local_design.index]
        Kpositive = 0.5:*(Kpositive+Kpositive')
        if (hasmissing(Kpositive) | min(diagonal(Kpositive)) <= 0) {
            out.failure_block_index = panel
            out.failure_stage = 1
            if (!hasmissing(diagonal(Kpositive))) {
                out.failure_metric_min = min(diagonal(Kpositive))
                out.failure_metric_max = max(diagonal(Kpositive))
            }
            if (one_copy) out.status = "ROW_DELETION_FACE_FAILURE"
            else out.status = status_prefix+"_POSITIVE_FACE_FAILURE"
            return(out)
        }
        positive_scale = sqrt(diagonal(Kpositive))
        Kpositive_scaled = ((1:/positive_scale)*(1:/positive_scale)') :*
            Kpositive
        if (rank(Kpositive_scaled) != rows(Kpositive_scaled)) {
            out.failure_block_index = panel
            out.failure_stage = 2
            if (one_copy) out.status = "ROW_DELETION_FACE_FAILURE"
            else out.status = status_prefix+"_POSITIVE_FACE_FAILURE"
            return(out)
        }
        if (one_copy) positive_deletion_weight = y[idx]
        else positive_deletion_weight = frequency[idx]:*y[idx]
        Zpositive = (sqrt(positive_deletion_weight)*
            J(1,cols(local_X),1)):*local_X
        Zpositive = Zpositive:*(J(rows(Zpositive),1,1)*positive_scale')
        Rpositive = Zpositive'*Zpositive
        eigpositive = Re(eigenvalues(Kpositive_scaled*Rpositive))'
        eigmaxpositive = max(eigpositive)
        if (min(eigpositive) < -rank_tolerance |
            eigmaxpositive >= 1-rank_tolerance) {
            out.failure_block_index = panel
            out.failure_stage = 3
            out.failure_metric_min = min(eigpositive)
            out.failure_metric_max = eigmaxpositive
            if (one_copy) out.status = "ROW_DELETION_FACE_FAILURE"
            else out.status = status_prefix+"_POSITIVE_FACE_FAILURE"
            return(out)
        }
        out.max_positive_block_eigen = max((out.max_positive_block_eigen,
            eigmaxpositive))
        if (one_copy) ac = residual[idx]:/sqrt(fit_mu[idx])
        else ac = sqrt(frequency[idx]:/fit_mu[idx]):*residual[idx]
        b = Z'*ac
        small_system = local_kernel_inverse.inverse-R
        local_system_inverse = ppmltalo__scaled_sym_inverse(
            small_system,1e-8)
        if (local_system_inverse.status != "CONVERGED") {
            out.status = status_prefix+"_LOCAL_INVERSE_SINGULAR"
            return(out)
        }
        if (missing(out.min_local_system_rcond)) {
            out.min_local_system_rcond = local_system_inverse.rcond
        }
        else out.min_local_system_rcond = min((out.min_local_system_rcond,
            local_system_inverse.rcond))
        gamma = local_system_inverse.inverse*b
        delta = Z*gamma
        out.deleted_mu[idx] = exp(eta[idx]-delta:/sqrt(deletion_weight))
        if (hasmissing(out.deleted_mu[idx]) | min(out.deleted_mu[idx]) <= 0) {
            out.status = status_prefix+"_DELETED_MEAN_NONFINITE"
            return(out)
        }
    }
    qbeta_w = ppmltalo__target_var(D,beta,pi,1)
    qbeta_f = ppmltalo__target_var(D,beta,pi,2)
    qbeta_c = ppmltalo__target_cov(D,beta,pi,1,2)
    d_w = D.mu:*ppmltalo__hdfe_x(D,A*qbeta_w)
    d_f = D.mu:*ppmltalo__hdfe_x(D,A*qbeta_f)
    d_c = D.mu:*ppmltalo__hdfe_x(D,A*qbeta_c)
    if (hasmissing(qbeta_w) | hasmissing(qbeta_f) |
        hasmissing(qbeta_c) | hasmissing(d_w) | hasmissing(d_f) |
        hasmissing(d_c)) {
        out.status = status_prefix+"_TARGET_NONFINITE"
        return(out)
    }
    out.plugin = ((beta'*qbeta_w)[1,1],(beta'*qbeta_f)[1,1],
                  (beta'*qbeta_c)[1,1],0)
    out.plugin[4] = out.plugin[1]+out.plugin[2]+2*out.plugin[3]
    if (hasmissing(out.plugin)) {
        out.status = status_prefix+"_TARGET_NONFINITE"
        return(out)
    }
    y_mass = frequency:*y
    if (one_copy) e_mass = y:-out.deleted_mu
    else e_mass = frequency:*(y:-out.deleted_mu)
    if (hasmissing(y_mass) | hasmissing(e_mass)) {
        out.status = status_prefix+"_TRACE_NONFINITE"
        return(out)
    }
    if (exact_trace) {
        correction_state = J(2,4,0)
        for (completed=0; completed<D.p; completed=completed+actual) {
            actual = min((batch_size,D.p-completed))
            basis = J(D.p,actual,0)
            for (column=1; column<=actual; column++) {
                basis[completed+column,column] = 1
            }
            z = ppmltalo__hdfe_xm(D,basis)
            kz = ppmltalo__match_Km(z,y_mass,e_mass,match_order,match_panel)
            rhs = ppmltalo__hdfe_xtm(D,kz)
            U = A*basis
            V = A*rhs
            xu = ppmltalo__hdfe_xm(D,U)
            xv = ppmltalo__hdfe_xm(D,V)
            qv_w = ppmltalo__target_varm(D,V,pi,1)
            qv_f = ppmltalo__target_varm(D,V,pi,2)
            qv_c = ppmltalo__target_covm(D,V,pi,1,2)
            if (hasmissing(z) | hasmissing(kz) | hasmissing(rhs) |
                hasmissing(U) | hasmissing(V) | hasmissing(xu) |
                hasmissing(xv) | hasmissing(qv_w) |
                hasmissing(qv_f) | hasmissing(qv_c)) {
                out.status = status_prefix+"_TRACE_NONFINITE"
                return(out)
            }
            values = J(actual,4,0)
            values[,1] = (ppmltalo__coldot(U,qv_w) -
                ppmltalo__coltriple(xu,d_w,xv))'
            values[,2] = (ppmltalo__coldot(U,qv_f) -
                ppmltalo__coltriple(xu,d_f,xv))'
            values[,3] = (ppmltalo__coldot(U,qv_c) -
                ppmltalo__coltriple(xu,d_c,xv))'
            values[,4] = values[,1]+values[,2]+2:*values[,3]
            if (hasmissing(values)) {
                out.status = status_prefix+"_TRACE_NONFINITE"
                return(out)
            }
            correction_state = ppmltalo__compensated_update(
                correction_state,values)
        }
        out.correction = correction_state[1,.]+correction_state[2,.]
        out.correction[4] = out.correction[1]+out.correction[2]+
            2*out.correction[3]
        out.mcse = J(1,4,0)
        out.talo = out.plugin-out.correction
        out.talo[4] = out.talo[1]+out.talo[2]+2*out.talo[3]
        if (hasmissing(out.correction) | hasmissing(out.talo) |
            hasmissing((out.information_rcond,out.positive_rcond,
                out.min_local_kernel_rcond,out.min_local_system_rcond)) |
            min((out.information_rcond,out.positive_rcond,
                out.min_local_kernel_rcond,out.min_local_system_rcond)) <= 0) {
            out.status = status_prefix+"_TRACE_NONFINITE"
            return(out)
        }
        out.probes = 0
        out.trace_directions = D.p
        out.status = "CONVERGED"
        return(out)
    }
    rseed(seed)
    mean = J(1,4,0)
    M2 = J(1,4,0)
    draw = 0
    for (completed=0; completed<probes; completed=completed+actual) {
        actual = min((batch_size,probes-completed))
        z = ppmltalo__rademacher_batch(D.n,actual)
        kz = ppmltalo__match_Km(z,y_mass,e_mass,match_order,match_panel)
        rhs = ppmltalo__hdfe_xtm(D,(z,kz))
        U = A*rhs[,(1::actual)]
        V = A*rhs[,(actual+1::2*actual)]
        xu = ppmltalo__hdfe_xm(D,U)
        xv = ppmltalo__hdfe_xm(D,V)
        qv_w = ppmltalo__target_varm(D,V,pi,1)
        qv_f = ppmltalo__target_varm(D,V,pi,2)
        qv_c = ppmltalo__target_covm(D,V,pi,1,2)
        if (hasmissing(z) | hasmissing(kz) | hasmissing(rhs) |
            hasmissing(U) | hasmissing(V) | hasmissing(xu) |
            hasmissing(xv) | hasmissing(qv_w) | hasmissing(qv_f) |
            hasmissing(qv_c)) {
            out.status = status_prefix+"_TRACE_NONFINITE"
            return(out)
        }
        values = J(actual,4,0)
        values[,1] = (ppmltalo__coldot(U,qv_w) -
            ppmltalo__coltriple(xu,d_w,xv))'
        values[,2] = (ppmltalo__coldot(U,qv_f) -
            ppmltalo__coltriple(xu,d_f,xv))'
        values[,3] = (ppmltalo__coldot(U,qv_c) -
            ppmltalo__coltriple(xu,d_c,xv))'
        values[,4] = values[,1]+values[,2]+2:*values[,3]
        if (hasmissing(values)) {
            out.status = status_prefix+"_TRACE_NONFINITE"
            return(out)
        }
        for (column=1; column<=actual; column++) {
            draw++
            update = values[column,.]-mean
            mean = mean+update:/draw
            M2 = M2+update:*(values[column,.]-mean)
        }
    }
    out.correction = mean
    out.mcse = sqrt((M2:/(probes-1)):/probes)
    out.correction[4] = out.correction[1]+out.correction[2]+
        2*out.correction[3]
    out.talo = out.plugin-out.correction
    out.talo[4] = out.talo[1]+out.talo[2]+2*out.talo[3]
    if (hasmissing(out.correction) | hasmissing(out.mcse) |
        hasmissing(out.talo) |
        hasmissing((out.information_rcond,out.positive_rcond,
            out.min_local_kernel_rcond,out.min_local_system_rcond)) |
        min((out.information_rcond,out.positive_rcond,
            out.min_local_kernel_rcond,out.min_local_system_rcond)) <= 0) {
        out.status = status_prefix+"_TRACE_NONFINITE"
        return(out)
    }
    out.probes = probes
    out.trace_directions = 0
    out.status = "CONVERGED"
    return(out)
}

struct ppmltalo_match_result scalar ppmltalo__match_dense(
    struct ppmltalo_hdfe_design scalar D,
    real colvector y,
    real colvector fit_mu,
    real colvector frequency,
    real colvector match,
    real colvector beta,
    real colvector eta,
    real colvector pi,
    real scalar exact_trace,
    real scalar probes,
    real scalar batch_size,
    real scalar seed,
    real scalar block_limit,
    real scalar rank_tolerance)
{
    return(ppmltalo__block_dense(D,y,fit_mu,frequency,match,beta,eta,pi,
        exact_trace,probes,batch_size,seed,block_limit,rank_tolerance,
        0,1,10000,500,"MATCH"))
}

real colvector ppmltalo__beta_from_fe(
    struct ppmltalo_hdfe_design scalar D,
    real matrix fe)
{
    real colvector beta, ord, values, counts, take
    real matrix info
    real scalar j, anchor_shift

    if (rows(fe) != D.n | cols(fe) != D.k | hasmissing(fe)) {
        _error(3200, "saved fixed effects must be a finite n by k matrix")
    }
    beta = J(D.p, 1, 0)
    anchor_shift = 0
    for (j=1; j<=D.k; j++) {
        ord = D.order[,j]
        values = panelsum(fe[ord,j], *D.panelinfo[j])
        counts = panelsum(J(D.n,1,1)[ord], *D.panelinfo[j])
        values = values :/ counts
        take = (D.offsets[j]+1::D.offsets[j]+D.params[j])
        if (j == 1) beta[take] = values
        else {
            anchor_shift = anchor_shift + values[D.levels[j]]
            beta[take] = values[(1::D.params[j])] :- values[D.levels[j]]
        }
    }
    beta[(1::D.params[1])] = beta[(1::D.params[1])] :+ anchor_shift
    return(beta)
}

struct ppmltalo_correction_result scalar ppmltalo__row_exact_targets(
    struct ppmltalo_hdfe_design scalar D,
    real colvector y,
    real colvector beta,
    real colvector eta,
    real colvector pi,
    real scalar worker_block,
    real scalar firm_block,
    real scalar leverage_limit)
{
    struct ppmltalo_correction_result scalar out
    struct ppmltalo_dense_result scalar worker, firm, covariance
    struct ppmltalo_dense_inverse_result scalar information_inverse
    real matrix X, identity, Qworker, Qfirm, Qcovariance

    identity = I(D.p)
    X = ppmltalo__hdfe_xm(D,identity)
    information_inverse = ppmltalo__dense_inverse(X,D.mu)
    if (information_inverse.status != "CONVERGED") {
        _error(3498,"exact dense information inversion failed its residual gate")
    }
    Qworker = ppmltalo__target_varm(D,identity,pi,worker_block)
    Qfirm = ppmltalo__target_varm(D,identity,pi,firm_block)
    Qcovariance = ppmltalo__target_covm(D,identity,pi,
        worker_block,firm_block)
    worker = ppmltalo__dense_row(y,X,beta,Qworker,eta,D.mu)
    firm = ppmltalo__dense_row(y,X,beta,Qfirm,eta,D.mu)
    covariance = ppmltalo__dense_row(y,X,beta,Qcovariance,eta,D.mu)
    if (max(worker.leverage) >= leverage_limit) {
        _error(3498, "exact leverage exceeds the configured safety limit")
    }
    out.plugin = (worker.plugin,firm.plugin,covariance.plugin,0)
    out.correction = (worker.correction,firm.correction,
        covariance.correction,0)
    out.talo = (worker.talo,firm.talo,covariance.talo,0)
    out.plugin[4] = out.plugin[1] + out.plugin[2] + 2*out.plugin[3]
    out.correction[4] = out.correction[1] + out.correction[2] +
        2*out.correction[3]
    out.talo[4] = out.talo[1] + out.talo[2] + 2*out.talo[3]
    out.mcse = J(1,4,0)
    out.deleted_mu = worker.deleted_mu
    out.probes = 0
    out.max_leverage = max(worker.leverage)
    out.max_solve_relres = 0
    out.max_solve_iterations = 0
    out.information_inverse_relres = information_inverse.relres
    out.information_rcond = information_inverse.rcond
    out.status = "CONVERGED"
    if (hasmissing(out.plugin) | hasmissing(out.correction) |
        hasmissing(out.talo) | hasmissing(out.mcse) |
        hasmissing(out.information_inverse_relres) |
        hasmissing(out.information_rcond) | out.information_rcond <= 0) {
        _error(3498,"exact dense target output is nonfinite")
    }
    return(out)
}

void ppmltalo__stata_run(
    string scalar yvar,
    string scalar muvar,
    string scalar idvars,
    string scalar fevars,
    string scalar pivar,
    string scalar touse,
    real scalar exact_mode,
    real scalar exact_limit,
    real scalar leverage_probes,
    real scalar correction_probes,
    real scalar batch_size,
    real scalar seed,
    real scalar tolerance,
    real scalar score_tolerance,
    real scalar maxiter,
    real scalar leverage_limit)
{
    struct ppmltalo_hdfe_design scalar D
    struct ppmltalo_deletion_gate_result scalar deletion_gate
    struct ppmltalo_leverage_result scalar lev
    struct ppmltalo_correction_result scalar result
    real matrix ids, fe
    real colvector y, mu, pi, beta, eta, upper, canonical_order
    real colvector score, score_y, score_mu
    real scalar score_relres

    y = st_data(., yvar, touse)
    mu = st_data(., muvar, touse)
    ids = st_data(., tokens(idvars), touse)
    fe = st_data(., tokens(fevars), touse)
    pi = st_data(., pivar, touse)
    if (rows(y) == 0) _error(2000, "no observations")
    if (cols(ids) != 2) {
        st_local("PPMLTALO_failure_status","UNSUPPORTED_FE_DIMENSIONS")
        _error(3498, "production deletion certificate requires exactly two fixed-effect blocks")
    }
    if (max(ids[,2]) == 1) {
        st_local("PPMLTALO_failure_status","UNSUPPORTED_ONE_FIRM_LEVEL")
        _error(3498, "one-level firm quotient is not supported")
    }
    canonical_order = order((ids, y, mu, pi), (1..cols(ids)+3))
    y = y[canonical_order]
    mu = mu[canonical_order]
    ids = ids[canonical_order,.]
    fe = fe[canonical_order,.]
    pi = pi[canonical_order]
    D = ppmltalo__hdfe_build(ids, mu)
    beta = ppmltalo__beta_from_fe(D, fe)
    eta = log(mu)
    score = ppmltalo__hdfe_xt(D, y :- mu)
    score_y = ppmltalo__hdfe_xt(D, y)
    score_mu = ppmltalo__hdfe_xt(D, mu)
    score_relres = ppmltalo__norm2(score) /
        (1 + ppmltalo__norm2(score_y) + ppmltalo__norm2(score_mu))
    st_numscalar("PPMLTALO_score_relres", score_relres)
    if (score_relres > score_tolerance) {
        st_local("PPMLTALO_failure_status","PPML_SCORE_RESIDUAL")
        _error(3498, "PPML score residual exceeds score_tolerance()")
    }
    deletion_gate = ppmltalo__row_deletion_gate(D,y)
    st_numscalar("PPMLTALO_bridge_rows",deletion_gate.bridge_rows)
    st_numscalar("PPMLTALO_margin_rows",deletion_gate.positive_margin_rows)
    st_numscalar("PPMLTALO_positive_missing",
        deletion_gate.positive_graph_missing_nodes)
    st_numscalar("PPMLTALO_positive_components",
        deletion_gate.positive_graph_components)
    st_numscalar("PPMLTALO_positive_bridges",
        deletion_gate.positive_graph_bridge_rows)
    if (deletion_gate.status != "PASSED_TWO_WAY_DELETION_CERTIFICATE") {
        st_local("PPMLTALO_failure_status",deletion_gate.status)
        if (deletion_gate.bridge_rows > 0) {
            _error(3498, "row deletion disconnects the worker-firm quotient graph")
        }
        if (deletion_gate.positive_margin_rows > 0) {
            _error(3498, "row deletion removes a positive worker or firm outcome margin")
        }
        _error(3498, "positive-outcome graph does not certify every row-deleted PPML face")
    }
    if (exact_mode) {
        if (D.n > exact_limit | D.p > exact_limit) {
            st_local("PPMLTALO_failure_status","EXACT_LIMIT")
            _error(3498, "exact mode exceeds exact_limit() in rows or quotient parameters")
        }
        result = ppmltalo__row_exact_targets(D,y,beta,eta,pi,1,2,
            leverage_limit)
        st_matrix("PPMLTALO_plugin", result.plugin)
        st_matrix("PPMLTALO_correction", result.correction)
        st_matrix("PPMLTALO_mcse", result.mcse)
        st_matrix("PPMLTALO_talo", result.talo)
        st_numscalar("PPMLTALO_max_h", result.max_leverage)
        st_numscalar("PPMLTALO_max_h_mcse", 0)
        st_numscalar("PPMLTALO_max_h_upper", result.max_leverage)
        st_numscalar("PPMLTALO_max_relres",
            result.information_inverse_relres)
        st_numscalar("PPMLTALO_info_inv_relres",
            result.information_inverse_relres)
        st_numscalar("PPMLTALO_info_rcond",result.information_rcond)
        st_numscalar("PPMLTALO_max_iterations", 0)
        st_numscalar("PPMLTALO_score_relres", score_relres)
        return
    }
    lev = ppmltalo__leverage_sketch(D, leverage_probes, batch_size, seed,
        tolerance, maxiter)
    if (lev.status != "CONVERGED") {
        st_local("PPMLTALO_failure_status","LEVERAGE_" + lev.status)
        errprintf("leverage sketch information solve failed: %s; max residual %21.15e; max iterations %g\n",
            lev.status, lev.max_solve_relres, lev.max_solve_iterations)
        _error(3498)
    }
    upper = lev.leverage + 3.290527 :* lev.mcse
    if (max(upper) >= leverage_limit) {
        st_local("PPMLTALO_failure_status","LEVERAGE_LIMIT")
        _error(3498, "leverage upper diagnostic exceeds the configured limit")
    }
    result = ppmltalo__row_correction(D, y, beta, eta, pi, lev.leverage,
        1, 2, correction_probes, batch_size, seed+1, tolerance, maxiter,
        leverage_limit)
    if (result.status != "CONVERGED") {
        st_local("PPMLTALO_failure_status","CORRECTION_" + result.status)
        errprintf("correction probe information solve failed: %s; max residual %21.15e; max iterations %g\n",
            result.status, result.max_solve_relres,
            result.max_solve_iterations)
        _error(3498)
    }
    st_matrix("PPMLTALO_plugin", result.plugin)
    st_matrix("PPMLTALO_correction", result.correction)
    st_matrix("PPMLTALO_mcse", result.mcse)
    st_matrix("PPMLTALO_talo", result.talo)
    st_numscalar("PPMLTALO_max_h", max(lev.leverage))
    st_numscalar("PPMLTALO_max_h_mcse", max(lev.mcse))
    st_numscalar("PPMLTALO_max_h_upper", max(upper))
    st_numscalar("PPMLTALO_max_relres", max((lev.max_solve_relres,
        result.max_solve_relres)))
    st_numscalar("PPMLTALO_max_iterations", max((lev.max_solve_iterations,
        result.max_solve_iterations)))
    st_numscalar("PPMLTALO_score_relres", score_relres)
}

void ppmltalo__stata_joint_run(
    string scalar yvar,
    string scalar muvar,
    string scalar frequencyvar,
    string scalar idvars,
    string scalar fevars,
    string scalar controlvars,
    string scalar pivar,
    string scalar touse,
    real scalar leverage_probes,
    real scalar correction_probes,
    real scalar batch_size,
    real scalar seed,
    real scalar tolerance,
    real scalar score_tolerance,
    real scalar maxiter,
    real scalar leverage_limit,
    real scalar rank_tolerance)
{
    struct ppmltalo_joint_design scalar JD, JDpositive
    struct ppmltalo_deletion_gate_result scalar deletion_gate
    struct ppmltalo_leverage_result scalar lev
    struct ppmltalo_correction_result scalar result
    real matrix ids, fe, controls
    real colvector y, mu, frequency, pi, beta, eta, upper, canonical_order
    real colvector score, score_y, score_mu, positive_rows
    real colvector full_certificate, positive_certificate
    real scalar score_relres, full_certificate_max, positive_certificate_max
    real scalar certificate_guard

    y = st_data(.,yvar,touse)
    mu = st_data(.,muvar,touse)
    frequency = st_data(.,frequencyvar,touse)
    ids = st_data(.,tokens(idvars),touse)
    fe = st_data(.,tokens(fevars),touse)
    if (strtrim(controlvars) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controlvars),touse)
    pi = st_data(.,pivar,touse)
    if (rows(y) == 0) _error(2000,"no observations")
    if (cols(ids) < 2) {
        st_local("PPMLTALO_failure_status","UNSUPPORTED_FE_DIMENSIONS")
        _error(3498,"joint observation deletion needs worker and firm effects")
    }
    canonical_order = order((ids,controls,y,mu,frequency,pi),
        (1..(cols(ids)+cols(controls)+4)))
    y = y[canonical_order]
    mu = mu[canonical_order]
    frequency = frequency[canonical_order]
    ids = ids[canonical_order,.]
    fe = fe[canonical_order,.]
    controls = controls[canonical_order,.]
    pi = pi[canonical_order]
    JD = ppmltalo__joint_build(ids,controls,frequency:*mu,
        tolerance,maxiter,batch_size,rank_tolerance)
    if (JD.status != "CONVERGED") {
        if (JD.status == "NUISANCE_COLLINEAR") {
            st_local("PPMLTALO_failure_status","NUISANCE_COLLINEARITY")
        }
        else st_local("PPMLTALO_failure_status","JOINT_INFORMATION_"+JD.status)
        _error(3498,"joint information preparation failed")
    }
    beta = ppmltalo__beta_from_fe(JD.full,fe)
    eta = log(mu)
    score = ppmltalo__hdfe_xt(JD.full,frequency:*(y-mu))
    score_y = ppmltalo__hdfe_xt(JD.full,frequency:*y)
    score_mu = ppmltalo__hdfe_xt(JD.full,frequency:*mu)
    score_relres = ppmltalo__norm2(score)/
        (1+ppmltalo__norm2(score_y)+ppmltalo__norm2(score_mu))
    st_numscalar("PPMLTALO_score_relres",score_relres)
    if (score_relres > score_tolerance) {
        st_local("PPMLTALO_failure_status","PPML_SCORE_RESIDUAL")
        _error(3498,"weighted PPML score residual exceeds score_tolerance()")
    }

    deletion_gate = ppmltalo__row_gate_weighted(JD.base,y,frequency)
    st_numscalar("PPMLTALO_bridge_rows",deletion_gate.bridge_rows)
    st_numscalar("PPMLTALO_margin_rows",deletion_gate.positive_margin_rows)
    st_numscalar("PPMLTALO_positive_missing",
        deletion_gate.positive_graph_missing_nodes)
    st_numscalar("PPMLTALO_positive_components",
        deletion_gate.positive_graph_components)
    st_numscalar("PPMLTALO_positive_bridges",
        deletion_gate.positive_graph_bridge_rows)
    if (deletion_gate.status != "PASSED_TWO_WAY_DELETION_CERTIFICATE") {
        st_local("PPMLTALO_failure_status",deletion_gate.status)
        _error(3498,"weighted row-deletion graph or positive face failed")
    }

    full_certificate_max = .
    positive_certificate_max = .
    certificate_guard = max((1e-10,100*JD.preparation_relres,
        100*JD.small_inverse_relres))
    full_certificate = ppmltalo__joint_lev_cert(JD,mu)
    full_certificate_max = max(full_certificate)+certificate_guard
    if (full_certificate_max >= leverage_limit) {
        st_numscalar("PPMLTALO_joint_full_cert",full_certificate_max)
        st_local("PPMLTALO_failure_status","JOINT_LEVERAGE_CERTIFICATE_LIMIT")
        _error(3498,"joint leverage cannot be certified below leverage_limit()")
    }
    if (JD.nuisance_parameters) {
        JDpositive = ppmltalo__joint_build(ids,controls,frequency:*y,
            tolerance,maxiter,batch_size,rank_tolerance)
        if (JDpositive.status != "CONVERGED") {
            if (JDpositive.status == "NUISANCE_COLLINEAR") {
                st_local("PPMLTALO_failure_status","POSITIVE_NUISANCE_COLLINEARITY")
            }
            else st_local("PPMLTALO_failure_status","POSITIVE_JOINT_INFORMATION_"+
                JDpositive.status)
            _error(3498,"positive joint information preparation failed")
        }
        if (JDpositive.full.p != JD.full.p |
            JDpositive.nuisance_parameters != JD.nuisance_parameters) {
            st_local("PPMLTALO_failure_status","POSITIVE_NUISANCE_QUOTIENT_CHANGED")
            _error(3498,"positive information changed the full-sample nuisance quotient")
        }
        certificate_guard = max((certificate_guard,
            100*JDpositive.preparation_relres,
            100*JDpositive.small_inverse_relres))
        positive_certificate = ppmltalo__joint_lev_cert(JDpositive,y)
        positive_certificate_max = max(positive_certificate)+certificate_guard
        if (positive_certificate_max >= 1-rank_tolerance) {
            st_numscalar("PPMLTALO_joint_pos_cert",
                positive_certificate_max)
            st_local("PPMLTALO_failure_status","ROW_DELETION_FACE_CERTIFICATE_FAILURE")
            _error(3498,"joint positive face cannot be certified after deletion")
        }
    }
    st_numscalar("PPMLTALO_joint_full_cert",full_certificate_max)
    st_numscalar("PPMLTALO_joint_pos_cert",positive_certificate_max)
    st_numscalar("PPMLTALO_joint_cert_guard",certificate_guard)
    st_numscalar("PPMLTALO_joint_prep_relres",JD.preparation_relres)
    st_numscalar("PPMLTALO_joint_small_relres",JD.small_inverse_relres)
    st_numscalar("PPMLTALO_joint_schur_rcond",JD.schur_rcond)

    lev = ppmltalo__joint_leverage_sketch(JD,mu,leverage_probes,
        batch_size,seed,tolerance,maxiter)
    if (lev.status != "CONVERGED") {
        st_local("PPMLTALO_failure_status","LEVERAGE_"+lev.status)
        _error(3498,"joint leverage sketch information solve failed")
    }
    upper = lev.leverage+3.290527:*lev.mcse
    if (max(upper) >= leverage_limit) {
        st_local("PPMLTALO_failure_status","LEVERAGE_LIMIT")
        _error(3498,"leverage upper diagnostic exceeds the configured limit")
    }
    result = ppmltalo__joint_row_correction(JD,y,mu,frequency,beta,eta,pi,
        lev.leverage,1,2,correction_probes,batch_size,seed+1,tolerance,
        maxiter,leverage_limit)
    if (result.status != "CONVERGED") {
        st_local("PPMLTALO_failure_status","CORRECTION_"+result.status)
        _error(3498,"joint correction information solve failed")
    }
    st_matrix("PPMLTALO_plugin",result.plugin)
    st_matrix("PPMLTALO_correction",result.correction)
    st_matrix("PPMLTALO_mcse",result.mcse)
    st_matrix("PPMLTALO_talo",result.talo)
    st_numscalar("PPMLTALO_max_h",max(lev.leverage))
    st_numscalar("PPMLTALO_max_h_mcse",max(lev.mcse))
    st_numscalar("PPMLTALO_max_h_upper",max(upper))
    st_numscalar("PPMLTALO_max_relres",max((lev.max_solve_relres,
        result.max_solve_relres,JD.preparation_relres,JD.small_inverse_relres)))
    st_numscalar("PPMLTALO_max_iterations",max((lev.max_solve_iterations,
        result.max_solve_iterations)))
    st_numscalar("PPMLTALO_score_relres",score_relres)
}

void ppmltalo__stata_block_run(
    string scalar yvar,
    string scalar muvar,
    string scalar frequencyvar,
    string scalar matchvar,
    string scalar idvars,
    string scalar fevars,
    string scalar controlvars,
    string scalar pivar,
    string scalar touse,
    real scalar exact_trace,
    real scalar correction_probes,
    real scalar batch_size,
    real scalar seed,
    real scalar score_tolerance,
    real scalar block_limit,
    real scalar rank_tolerance,
    real scalar dense_limit,
    real scalar one_copy,
    real scalar require_constant,
    real scalar block_size_limit,
    real scalar local_dimension_limit,
    string scalar status_prefix)
{
    struct ppmltalo_hdfe_design scalar D
    struct ppmltalo_match_result scalar result
    real matrix ids, fe, controls
    real colvector y, mu, frequency, match, pi, beta, eta, canonical_order
    real colvector score, score_y, score_mu
    real scalar score_relres

    y = st_data(.,yvar,touse)
    mu = st_data(.,muvar,touse)
    frequency = st_data(.,frequencyvar,touse)
    match = st_data(.,matchvar,touse)
    ids = st_data(.,tokens(idvars),touse)
    fe = st_data(.,tokens(fevars),touse)
    if (strtrim(controlvars) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controlvars),touse)
    pi = st_data(.,pivar,touse)
    if (rows(y) == 0) _error(2000,"no observations")
    if (cols(ids) < 2) {
        st_local("PPMLTALO_failure_status","UNSUPPORTED_FE_DIMENSIONS")
        _error(3498,"dense deletion requires worker and firm fixed effects")
    }
    if (max(ids[,2]) == 1) {
        st_local("PPMLTALO_failure_status","UNSUPPORTED_ONE_FIRM_LEVEL")
        _error(3498,"one-level firm quotient is not supported")
    }
    canonical_order = order((ids,controls,y,mu,frequency,pi,match),
        (1..(cols(ids)+cols(controls)+5)))
    y = y[canonical_order]
    mu = mu[canonical_order]
    frequency = frequency[canonical_order]
    match = match[canonical_order]
    ids = ids[canonical_order,.]
    fe = fe[canonical_order,.]
    controls = controls[canonical_order,.]
    pi = pi[canonical_order]
    D = ppmltalo__hdfe_build_x(ids,controls,frequency:*mu)
    st_numscalar("PPMLTALO_quotient_parameters",D.p)
    st_numscalar("PPMLTALO_worker_levels",D.levels[1])
    st_numscalar("PPMLTALO_firm_levels",D.levels[2])
    st_numscalar("PPMLTALO_nuisance_levels",
        (D.k == 3 ? D.levels[3] : 0))
    st_numscalar("PPMLTALO_control_count",D.q)
    if (D.p > dense_limit) {
        st_local("PPMLTALO_failure_status",status_prefix+"_DENSE_PARAMETER_LIMIT")
        _error(3498,"dense deletion exceeds dense_limit() quotient parameters")
    }
    beta = ppmltalo__beta_from_fe(D,fe)
    eta = log(mu)
    score = ppmltalo__hdfe_xt(D,frequency:*(y:-mu))
    score_y = ppmltalo__hdfe_xt(D,frequency:*y)
    score_mu = ppmltalo__hdfe_xt(D,frequency:*mu)
    score_relres = ppmltalo__norm2(score) /
        (1+ppmltalo__norm2(score_y)+ppmltalo__norm2(score_mu))
    st_numscalar("PPMLTALO_score_relres",score_relres)
    if (score_relres > score_tolerance) {
        st_local("PPMLTALO_failure_status","PPML_SCORE_RESIDUAL")
        _error(3498,"weighted PPML score residual exceeds score_tolerance()")
    }
    result = ppmltalo__block_dense(D,y,mu,frequency,match,beta,eta,pi,
        exact_trace,correction_probes,batch_size,seed+1,block_limit,
        rank_tolerance,one_copy,require_constant,block_size_limit,
        local_dimension_limit,status_prefix)
    st_numscalar("PPMLTALO_score_relres",score_relres)
    st_numscalar("PPMLTALO_match_blocks",result.blocks)
    st_numscalar("PPMLTALO_expanded_n",result.expanded_n)
    st_numscalar("PPMLTALO_max_block_patterns",result.max_block_patterns)
    st_numscalar("PPMLTALO_max_local_rank",result.max_local_rank)
    st_numscalar("PPMLTALO_max_block_eigen",result.max_block_eigen)
    st_numscalar("PPMLTALO_max_pos_block_eigen",
        result.max_positive_block_eigen)
    st_numscalar("PPMLTALO_info_inv_relres",
        result.information_inverse_relres)
    st_numscalar("PPMLTALO_pos_inv_relres",
        result.positive_inverse_relres)
    st_numscalar("PPMLTALO_info_rcond",result.information_rcond)
    st_numscalar("PPMLTALO_pos_rcond",result.positive_rcond)
    st_numscalar("PPMLTALO_local_kernel_rcond",
        result.min_local_kernel_rcond)
    st_numscalar("PPMLTALO_local_system_rcond",
        result.min_local_system_rcond)
    st_numscalar("PPMLTALO_trace_directions",result.trace_directions)
    st_numscalar("PPMLTALO_failure_block_index",result.failure_block_index)
    st_numscalar("PPMLTALO_failure_stage",result.failure_stage)
    st_numscalar("PPMLTALO_failure_metric_min",result.failure_metric_min)
    st_numscalar("PPMLTALO_failure_metric_max",result.failure_metric_max)
    if (result.status != "CONVERGED") {
        st_local("PPMLTALO_failure_status",result.status)
        _error(3498,"weighted match TALO engine withheld the correction")
    }
    st_matrix("PPMLTALO_plugin",result.plugin)
    st_matrix("PPMLTALO_correction",result.correction)
    st_matrix("PPMLTALO_mcse",result.mcse)
    st_matrix("PPMLTALO_talo",result.talo)
}

void ppmltalo__stata_match_run(
    string scalar yvar,
    string scalar muvar,
    string scalar frequencyvar,
    string scalar matchvar,
    string scalar idvars,
    string scalar fevars,
    string scalar controlvars,
    string scalar pivar,
    string scalar touse,
    real scalar exact_trace,
    real scalar correction_probes,
    real scalar batch_size,
    real scalar seed,
    real scalar score_tolerance,
    real scalar block_limit,
    real scalar rank_tolerance,
    real scalar dense_limit)
{
    ppmltalo__stata_block_run(yvar,muvar,frequencyvar,matchvar,idvars,
        fevars,controlvars,pivar,touse,exact_trace,correction_probes,
        batch_size,seed,score_tolerance,block_limit,rank_tolerance,
        dense_limit,0,1,10000,500,"MATCH")
}

end
