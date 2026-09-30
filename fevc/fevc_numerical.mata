*! fevc numerical kernels and native validation API 4 30sep2026
version 18.0
mata:
mata set matastrict on
mata set matalnum off
real scalar vckss_nmc__module_api()
{
    return(4)
}
string scalar vckss_nmc__build_id()
{
    return("vckss-numerical-api4-vector-replay8")
}
real scalar vckss_nmc__norm2(real matrix value)
{
    if (rows(value)==0 | cols(value)==0) return(0)
    return(sqrt(quadcross(vec(value),vec(value))))
}
real rowvector vckss_nmc__column_sum(real matrix values)
{
    if (rows(values)==0 | cols(values)==0 | hasmissing(values)) return(J(1,0,.))
    return(quadcolsum(values))
}

/* Reduce all panels at a common offset with Neumaier compensation. Wide
   panels with few groups use quad sums instead of a long offset loop. Both
   routes preserve small addends, including (1e16,1,-1e16). */
real matrix vckss_nmc__panelsum(real matrix values, real matrix panel,
    | real colvector row_order)
{
    real scalar first, last, offset, width, g
    real colvector start, length, active, index
    real matrix out, subtotal, correction, add, current, next
    if (rows(panel)==0) return(J(0,cols(values),0))
    if (hasmissing(values)) return(J(rows(panel),cols(values),.))
    if (args()<3) row_order = (1..rows(values))'
    out = J(rows(panel),cols(values),0)
    width = max(panel[.,2]-panel[.,1]:+1)
    if (width > max((32,2*rows(panel)))) {
        for (g=1; g<=rows(panel); g++) {
            index = row_order[|panel[g,1]\panel[g,2]|]
            out[g,.] = quadcolsum(values[index,.],1)
        }
        return(out)
    }
    for (first=1; first<=rows(panel); first=first+65536) {
        last = min((rows(panel),first+65535))
        start = panel[first..last,1]
        length = panel[first..last,2]:-start:+1
        subtotal = correction = J(last-first+1,cols(values),0)
        for (offset=0; offset<max(length); offset++) {
            active = selectindex(length:>offset)
            add = values[row_order[start[active]:+offset],.]
            current = subtotal[active,.]
            next = current+add
            correction[active,.] = correction[active,.] +
                (abs(current):>=abs(add)):*((current-next)+add) +
                (abs(current):<abs(add)):*((add-next)+current)
            subtotal[active,.] = next
        }
        out[first..last,.] = subtotal+correction
    }
    return(out)
}

/* A short-panel gather has at most twice as many entries as its source.
   This admits the common small matches without a groups-by-maximum-width
   allocation on skewed or frequency-heavy panels. Zero padding addresses
   the extra zero row supplied by the contraction. */
real matrix vckss_nmc__panel_plan(real matrix panel, real colvector row_order)
{
    real scalar width, offset
    real colvector length, active
    real matrix plan
    if (rows(panel)==0) return(J(0,0,.))
    length = panel[.,2]-panel[.,1]:+1
    width = max(length)
    if (width>16 | width*rows(panel)>2*rows(row_order)) return(J(0,0,.))
    plan = J(rows(panel),width,rows(row_order)+1)
    for (offset=0; offset<width; offset++) {
        active = selectindex(length:>offset)
        plan[active,offset+1] = row_order[panel[active,1]:+offset]
    }
    return(plan)
}

real matrix vckss_nmc__planned_sum(real matrix values, real matrix panel,
    real matrix plan)
{
    real scalar column
    real matrix padded, out
    if (cols(plan)==0) return(vckss_nmc__panelsum(values,panel))
    padded = values\J(1,cols(values),0)
    out = J(rows(plan),cols(values),.)
    for (column=1; column<=cols(values); column++) {
        out[.,column] = quadrowsum(rowshape(padded[vec(plan),column],cols(plan))',1)
    }
    return(out)
}

real colvector vckss_nmc__block_score(real matrix gradients,
    real colvector p, real colvector m)
{
    real colvector p2, m2
    p2 = p:^2; m2 = m:^2
    return(quadrowsum(gradients[.,1..5]:*
        (p2,m2,p2:^2,m2:^2,p2:*m2),1)-gradients[.,6])
}
/* Package-owned numerical-probe kernels. The public point runtime does not
   request these kernels until the versioned attachment preflight is wired. */
struct vckss_nmc__finite
{
    string scalar status
    real colvector h
    real colvector m
    real colvector bias
    real colvector variance
    real matrix dh
    real matrix db
    real matrix dv
}

struct vckss_nmc__covariance
{
    string scalar status
    real matrix conditional
    real matrix leverage
    real matrix raw
    real matrix usable
    real rowvector mcse
    real scalar psd_adjustment
}

struct vckss_nmc__attachment
{
    struct vckss_nmc__covariance scalar covariance
    real scalar leverage_probes
    real scalar target_probes
    real rowvector folds
    real matrix replay_rhs
    string scalar replay_failure
    real scalar failed_replay_probe
    real scalar replay_attempted_rhs
    real scalar replay_seconds
    real scalar replay_generator_evaluations
    real scalar allocation_bound_bytes
    real scalar minimum_constrained
    real scalar minimum_margin
    real scalar sensitivity_ratio
}

struct vckss_nmc__state
{
    string scalar status
    real scalar R
    real scalar T
    real matrix copy
    real matrix row
    real colvector observation_index
    real matrix observation_panel
    real colvector physical_row
    real colvector copy_order
    real colvector copy_group
    real matrix copy_fold
    real matrix block
    real colvector wt
    real matrix observation_fold
    real matrix observation_compensation
    real matrix block_fold
    real matrix block_compensation
    real colvector observation_weight
    real colvector block_y
    real colvector block_wt
    real colvector block_frequency
    real colvector block_root
    real matrix block_plan
    real scalar minimum_constrained
    real scalar minimum_margin
    real scalar sensitivity_ratio
    real scalar allocation_bound_bytes
}

real scalar vckss_nmc__api_level()
{
    return(1)
}

struct vckss_nmc__finite scalar vckss_nmc__finite_derivative(
    real matrix u, real scalar R)
{
    struct vckss_nmc__finite scalar out
    real colvector s, h, m, a, b, c
    real rowvector da, db, dc

    out.status = "nonfinite_derivative"
    out.dh = out.db = out.dv = J(rows(u),5,.)
    if (rows(u)==0 | cols(u) != 5 | hasmissing(u) | missing(R) | R < 2 |
        R != floor(R)) return(out)
    s = u[.,1]+u[.,2]
    if (hasmissing(s) | min(s) <= 0) return(out)
    h = u[.,1]:/s
    m = u[.,2]:/s
    a = u[.,3]; b = u[.,4]; c = u[.,5]
    out.h = h; out.m = m
    out.bias = (m:*a-h:*b+(m-h):*c)/R
    out.variance = (m:^2:*a+h:^2:*b-2*h:*m:*c)/R
    out.dh = (m:/s,-h:/s,J(rows(u),3,0))
    da = (0,0,1,0,0); db = (0,0,0,1,0); dc = (0,0,0,0,1)
    out.db = (-(a+b+2*c):*out.dh+m*da-h*db+(m-h)*dc)/R
    out.dv = ((-2*m:*a+2*h:*b-2*c:*(m-h)):*out.dh+
        (m:^2)*da+(h:^2)*db-(2*h:*m)*dc)/R
    if (hasmissing((out.bias,out.variance,out.dh,out.db,out.dv))) return(out)
    out.status = min(out.variance) < 0 ? "nonsmooth_adjustment" : "ok_local"
    return(out)
}

real matrix vckss_nmc__observation_gradient(
    struct vckss_nmc__finite scalar finite, real colvector control)
{
    real colvector ell, inverse, factor
    if (finite.status != "ok_local" | hasmissing(control)) return(J(rows(finite.m),5,.))
    ell = finite.m:-control
    if (min(ell) <= 0 | hasmissing(ell)) return(J(rows(ell),5,.))
    inverse = 1:/ell
    factor = -inverse:^2-2*finite.bias:*inverse:^3+3*finite.variance:*inverse:^4
    return(-factor:*finite.dh+inverse:^2:*finite.db-inverse:^3:*finite.dv)
}

real rowvector vckss_nmc__block_gradient(
    struct vckss_nmc__finite scalar finite, real scalar k)
{
    if (finite.status != "ok_local" | missing(k) | k <= 0) return(J(1,5,.))
    return((1+2*finite.bias*k-3*finite.variance*k^2):*finite.dh+
        finite.db-k:*finite.dv)
}

real matrix vckss_nmc__copy_pullback(real matrix lambda)
{
    if (cols(lambda) != 5 | hasmissing(lambda)) return(J(rows(lambda),4,.))
    return((lambda[.,1]+lambda[.,2]+6*lambda[.,4]+lambda[.,5],
        lambda[.,3]+lambda[.,4]+lambda[.,5],-2*lambda[.,2]-4*lambda[.,4],
        -4*lambda[.,4]-2*lambda[.,5]))
}

real matrix vckss_nmc__cross_covariance(real matrix a, real matrix b)
{
    real scalar R, scale, i, j
    real rowvector ma, mb
    real matrix out, aa, bb
    if (rows(a) != rows(b) | rows(a) < 2 | cols(a) != 3 |
        cols(b) != 3 | hasmissing(a) | hasmissing(b)) return(J(3,3,.))
    R = rows(a)
    scale = max((max(abs(a)),max(abs(b))))
    if (scale == 0) return(J(3,3,0))
    aa = a:/scale; bb = b:/scale
    ma = vckss_nmc__column_sum(aa:/R)
    mb = vckss_nmc__column_sum(bb:/R)
    aa = aa-J(R,1,1)*ma; bb = bb-J(R,1,1)*mb
    out = J(3,3,0)
    for (i=1; i<=3; i++) {
        for (j=i; j<=3; j++) {
            out[i,j] = vckss_nmc__column_sum(
                (aa[.,i]:*bb[.,j]+bb[.,i]:*aa[.,j]):/(2*R*(R-1)))[1]
            out[i,j] = (out[i,j]*scale)*scale
            out[j,i] = out[i,j]
        }
    }
    return(out)
}

struct vckss_nmc__covariance scalar vckss_nmc__finalize(
    real matrix conditional, real matrix leverage)
{
    struct vckss_nmc__covariance scalar out
    real scalar scale, tau
    real matrix raw, Q, usable, A
    real rowvector eigenvalues
    out.status = "nonfinite_derivative"
    out.conditional = conditional; out.leverage = leverage
    out.raw = out.usable = J(3,3,.)
    out.mcse = J(1,4,.)
    out.psd_adjustment = 0
    if (rows(conditional) != 3 | cols(conditional) != 3 |
        rows(leverage) != 3 | cols(leverage) != 3 |
        hasmissing(conditional) | hasmissing(leverage)) return(out)
    if (max(abs(conditional-conditional')) != 0 |
        max(abs(leverage-leverage')) != 0) return(out)
    out.raw = conditional+leverage
    if (hasmissing(out.raw)) return(out)
    scale = max((max(abs(conditional)),max(abs(leverage))))
    if (scale == 0) {
        out.status = "ok_local"
        out.usable = J(3,3,0); out.mcse = J(1,4,0)
        return(out)
    }
    tau = 1e-12*(vckss_nmc__norm2(conditional:/scale)+vckss_nmc__norm2(leverage:/scale))
    raw = (out.raw:/scale+(out.raw:/scale)')/2
    symeigensystem(raw,Q,eigenvalues)
    if (hasmissing(Q) | hasmissing(eigenvalues)) return(out)
    /* Certify the small eigensystem in its actual Mata orientation. */
    if (vckss_nmc__norm2(Q*diag(eigenvalues)*Q'-raw) > tau) Q = Q'
    if (vckss_nmc__norm2(Q*diag(eigenvalues)*Q'-raw) > tau) return(out)
    if (min(eigenvalues) < -tau) {
        out.status = "unstable_nonpsd"
        return(out)
    }
    out.status = "ok_local"
    usable = raw
    if (min(eigenvalues) < 0) {
        out.status = "ok_local_psd_adjusted"
        eigenvalues = eigenvalues:*(eigenvalues:>0)
        usable = Q*diag(eigenvalues)*Q'
        out.psd_adjustment = vckss_nmc__norm2(usable-raw)*scale
    }
    out.usable = usable:*scale
    A = (1,0,0\0,1,0\0,0,1\1,1,2)
    out.mcse = diagonal(A*usable*A')'
    out.mcse = sqrt(out.mcse:*(out.mcse:>0))*sqrt(scale)
    if (hasmissing(out.usable) | hasmissing(out.mcse)) {
        out.status = "nonfinite_derivative"
        out.usable = J(3,3,.); out.mcse = J(1,4,.)
    }
    return(out)
}



struct vckss_nmc__state scalar vckss_nmc__new(
    real scalar n, real scalar copies, real scalar groups, real scalar R)
{
    struct vckss_nmc__state scalar out
    out.status = "ok_local"
    out.R = out.T = R
    out.copy = J(0,2,0); out.row = J(0,3,0)
    out.observation_index = out.physical_row = J(0,1,.)
    out.observation_panel = J(0,2,.)
    out.block = J(groups,6,0); out.wt = J(n,1,0)
    out.copy_order = out.copy_group = out.observation_weight = J(0,1,.)
    out.copy_fold = out.block_plan = J(0,0,.)
    out.block_y = out.block_wt = out.block_frequency = out.block_root = J(0,1,.)
    out.observation_fold = out.observation_compensation = J(0,6,0)
    out.block_fold = out.block_compensation = J(groups,6,0)
    out.minimum_constrained = out.minimum_margin = .
    out.sensitivity_ratio = 0
    // Includes score/RHS export and temporary replay contractions of at most eight columns.
    out.allocation_bound_bytes = 192*copies+640*n+512*R+4096
    return(out)
}

/* Apply the same derivative kernel to bounded copy tiles. No moments are
   averaged before nonlinear inversion. Refuse the fast path on any diagnostic
   boundary so the scalar path retains its per-copy status/receipt ordering. */
real scalar vckss_nmc__obs_prepare_batch(
    struct vckss_nmc__state scalar state, real colvector a2sum,
    real colvector a4sum, real colvector c1sum, real colvector c3sum,
    real colvector control)
{
    struct vckss_nmc__finite scalar finite
    real scalar first, last, width, constrained, margin, sensitivity
    real colvector index, a2, a4, c1, c3, ell
    real matrix u, pullbacks, add, totals, plan
    constrained = state.minimum_constrained
    margin = state.minimum_margin
    sensitivity = state.sensitivity_ratio
    pullbacks = J(rows(state.copy),4,.)
    // Tile scratch fits the existing copy reserve even when frequency >> rows.
    width = min((1024,max((1,floor(rows(state.copy)/8)))))
    for (first=1; first<=rows(state.copy); first=first+width) {
        last = min((rows(state.copy),first+width-1))
        index = state.physical_row[|first\last|]
        a2 = a2sum[index]/state.R; a4 = a4sum[index]/state.R
        c1 = c1sum[|first\last|]/state.R; c3 = c3sum[|first\last|]/state.R
        u = (a2,1:+a2-2*c1,a4,1:+6*a2+a4-4*c1-4*c3,a2+a4-2*c3)
        finite = vckss_nmc__finite_derivative(u,state.R)
        if (finite.status!="ok_local") return(0)
        ell = finite.m-control[index]
        if (hasmissing(ell) | min(ell)<=0) return(0)
        add = vckss_nmc__copy_pullback(
            vckss_nmc__observation_gradient(finite,control[index]))
        if (hasmissing(add)) return(0)
        pullbacks[first..last,.] = add
        constrained = min((constrained,min(u[.,1]+u[.,2])))
        margin = min((margin,min(ell)))
        sensitivity = max((sensitivity,max(sqrt(finite.variance):/ell)))
    }
    state.copy = pullbacks[.,3..4]
    add = (pullbacks[.,1..2],pullbacks[.,3]:*(c1sum/state.R)+
        pullbacks[.,4]:*(c3sum/state.R))
    plan = vckss_nmc__panel_plan(state.observation_panel,state.copy_order)
    totals = vckss_nmc__planned_sum(add,state.observation_panel,plan)
    index = state.observation_index
    state.row = (totals[.,1..2],totals[.,1]:*(a2sum[index]/state.R)+
        totals[.,2]:*(a4sum[index]/state.R)+totals[.,3])
    state.minimum_constrained = constrained; state.minimum_margin = margin
    state.sensitivity_ratio = sensitivity
    return(1)
}

void vckss_nmc__observation_prepare(
    struct vckss_nmc__state scalar state,
    real colvector index, real matrix panel, real colvector physical_row,
    real colvector a2sum, real colvector a4sum,
    real colvector c1sum, real colvector c3sum, real colvector control)
{
    struct vckss_nmc__finite scalar finite
    real scalar i, j, row, a2, a4, c1, c3, ell
    real rowvector u, gradient, pullback, sums, compensation, add, next
    state.observation_index = index; state.observation_panel = panel
    state.physical_row = physical_row
    state.copy_order = (1..rows(physical_row))'
    state.copy_group = J(rows(physical_row),1,.)
    state.copy = J(rows(physical_row),2,0)
    state.row = J(rows(index),3,0)
    state.observation_fold = state.observation_compensation = J(rows(index),6,0)
    for (i=1; i<=rows(index); i++) {
        state.copy_group[|panel[i,1]\panel[i,2]|] = J(panel[i,2]-panel[i,1]+1,1,i)
    }
    if (vckss_nmc__obs_prepare_batch(state,a2sum,a4sum,
        c1sum,c3sum,control)) return
    for (i=1; i<=rows(index); i++) {
        row = index[i]
        a2 = a2sum[row]/state.R; a4 = a4sum[row]/state.R
        sums = compensation = J(1,3,0)
        for (j=panel[i,1]; j<=panel[i,2]; j++) {
            c1 = c1sum[j]/state.R; c3 = c3sum[j]/state.R
            u = (a2,1+a2-2*c1,a4,1+6*a2+a4-4*c1-4*c3,a2+a4-2*c3)
            finite = vckss_nmc__finite_derivative(u,state.R)
            state.minimum_constrained = min((state.minimum_constrained,u[1]+u[2]))
            ell = finite.m-control[row]
            state.minimum_margin = min((state.minimum_margin,ell))
            if (finite.status != "ok_local") {
                state.status = finite.status
                continue
            }
            state.sensitivity_ratio = max((state.sensitivity_ratio,
                sqrt(max((finite.variance,0)))/ell))
            gradient = vckss_nmc__observation_gradient(finite,control[row])
            pullback = vckss_nmc__copy_pullback(gradient)
            if (hasmissing(pullback)) {
                state.status = "nonfinite_derivative"
                continue
            }
            state.copy[j,.] = pullback[3..4]
            add = (pullback[1..2],pullback[3]*c1+pullback[4]*c3)-compensation
            next = sums+add
            compensation = (next-sums)-add
            sums = next
        }
        state.row[i,.] = (sums[1..2],sums[1]*a2+sums[2]*a4+sums[3])
    }
}

void vckss_nmc__block_prepare(
    struct vckss_nmc__state scalar state, real scalar group,
    real colvector index, real rowvector u, real colvector w,
    real scalar t, real scalar k, real scalar margin)
{
    struct vckss_nmc__finite scalar finite
    real rowvector beta
    finite = vckss_nmc__finite_derivative(u,state.R)
    state.minimum_constrained = min((state.minimum_constrained,u[1]+u[2]))
    state.minimum_margin = min((state.minimum_margin,margin))
    if (finite.status != "ok_local") {
        state.status = finite.status
        return
    }
    beta = vckss_nmc__block_gradient(finite,k)
    if (hasmissing(beta)) {
        state.status = "nonfinite_derivative"
        return
    }
    state.block[group,.] = (beta,vckss_nmc__column_sum((beta:*u)')[1])
    state.wt[index] = w:*t
    state.sensitivity_ratio = max((state.sensitivity_ratio,
        sqrt(max((finite.variance,0)))*k))
}

void vckss_nmc__fold_add(real matrix sums, real matrix compensation,
    real matrix values, real scalar start)
{
    real matrix y, next
    y = values-compensation[.,start..(start+2)]
    next = sums[.,start..(start+2)]+y
    compensation[.,start..(start+2)] =
        (next-sums[.,start..(start+2)])-y
    sums[.,start..(start+2)] = next
}

void vckss_nmc__target_prepare(struct vckss_nmc__state scalar state,
    real colvector working_y, real colvector residual,
    real colvector frequency, real colvector row_order, real matrix panel)
{
    state.observation_weight = working_y[state.observation_index]:*
        residual[state.observation_index]
    if (rows(panel)>0) {
        state.block_frequency = frequency[row_order]
        state.block_y = state.block_frequency:*working_y[row_order]
        state.block_wt = sqrt(state.block_frequency):*state.wt[row_order]
        state.block_root = sqrt(vckss_nmc__panelsum(state.block_frequency,panel))
        state.block_plan = vckss_nmc__panel_plan(panel,(1..rows(row_order))')
    }
}

void vckss_nmc__target(struct vckss_nmc__state scalar state,
    real scalar probe, real colvector sw, real colvector sf,
    real colvector row_order, real matrix panel)
{
    real scalar start, count
    real colvector ow, of
    real matrix values, prediction, first, second
    start = mod(probe,2) ? 1 : 4
    count = mod(probe,2) ? ceil(state.T/2) : floor(state.T/2)
    if (rows(state.observation_index)>0) {
        ow = sw[state.observation_index]; of = sf[state.observation_index]
        values = state.observation_weight:*(ow:^2,of:^2,ow:*of):/count
        vckss_nmc__fold_add(state.observation_fold,
            state.observation_compensation,values,start)
    }
    if (rows(panel)>0) {
        prediction = (sw[row_order],sf[row_order])
        first = vckss_nmc__planned_sum(state.block_y:*prediction,panel,state.block_plan)
        second = vckss_nmc__planned_sum(state.block_wt:*prediction,panel,state.block_plan)
        values = (first[.,1]:*second[.,1],first[.,2]:*second[.,2],
            (first[.,1]:*second[.,2]+first[.,2]:*second[.,1]):/2):/count
        vckss_nmc__fold_add(state.block_fold,state.block_compensation,values,start)
    }
}

real rowvector vckss_nmc__score(struct vckss_nmc__state scalar state,
    real colvector projected, real colvector physical_q,
    real colvector random_sum, real colvector row_order, real matrix panel,
    | real matrix contributions)
{
    real scalar observations
    real colvector p, p2, m, xi, copy_weight
    real matrix response, observation_score
    observations = rows(state.observation_index)
    if (args()<7) contributions = J(rows(panel),6,0)
    if (observations>0) {
        p = projected[state.observation_index]; p2 = p:^2
        xi = state.row[.,1]:*p2+state.row[.,2]:*p2:^2-state.row[.,3]
        // quadcross omits missing rows; a failed influence must be withheld.
        if (hasmissing(xi)) return(J(1,6,.))
        observation_score = -quadcross(xi,state.observation_fold)
        p = projected[state.physical_row]
        copy_weight = state.copy[.,1]:*physical_q:*p
        if (hasmissing(copy_weight)) return(J(1,6,.))
        observation_score = observation_score\-quadcross(copy_weight,state.copy_fold)
        copy_weight = state.copy[.,2]:*physical_q:*p:^3
        if (hasmissing(copy_weight)) return(J(1,6,.))
        observation_score = observation_score\-quadcross(copy_weight,state.copy_fold)
        observation_score = quadcolsum(observation_score,1)
    }
    if (rows(panel)>0) {
        response = vckss_nmc__planned_sum(
            (state.block_frequency:*projected[row_order],random_sum[row_order]),
            panel,state.block_plan)
        p = response[.,1]:/state.block_root
        m = response[.,2]:/state.block_root-p
        xi = vckss_nmc__block_score(state.block,p,m)
        contributions = -xi:*state.block_fold
    }
    if (observations==0) return(vckss_nmc__column_sum(contributions))
    if (rows(panel)==0) return(observation_score)
    return(quadcolsum(observation_score\quadcolsum(contributions,1),1))
}


real scalar vckss_nmc__schema()
{
    return(1)
}

/* The command owns these temporary destinations. Nothing is retained between
   invocations; point and replay receipts keep separate result families. */
void vckss_nmc__stata_export(struct vckss_nmc__attachment scalar a)
{
    real scalar n, attempted, residual, failed
    real matrix rhs
    n = rows(a.replay_rhs)
    attempted = a.replay_attempted_rhs
    residual = 0
    if (n > 0) residual = max(a.replay_rhs[.,3])
    st_matrix(st_global("VCKSS_NMC_COND"),a.covariance.conditional)
    st_matrix(st_global("VCKSS_NMC_LEV"),a.covariance.leverage)
    st_matrix(st_global("VCKSS_NMC_RAW"),a.covariance.raw)
    st_matrix(st_global("VCKSS_NMC_ALL"),a.covariance.usable)
    st_matrix(st_global("VCKSS_NMC_SE"),a.covariance.mcse)
    rhs = a.replay_rhs
    if (n>0) {
        rhs[.,2] = rhs[.,1]:-1
        rhs[.,1] = J(n,1,1)
    }
    failed = a.failed_replay_probe
    if (!missing(failed)) failed = failed-1
    st_matrix(st_global("VCKSS_NMC_RHS"),rhs)
    st_matrix(st_global("VCKSS_NMC_META"),(a.leverage_probes,
        a.target_probes,a.folds,n,attempted,a.replay_generator_evaluations,
        a.allocation_bound_bytes,a.covariance.psd_adjustment,
        a.minimum_constrained,a.minimum_margin,a.sensitivity_ratio,
        residual,a.replay_seconds,failed))
    st_global("VCKSS_NMC_STATUS",a.covariance.status)
    st_global("VCKSS_NMC_FAILURE",a.replay_failure)
}

real scalar vckss_nmc__relative_equal(real matrix a, real matrix b)
{
    real scalar scale
    if (rows(a)!=rows(b) | cols(a)!=cols(b) | hasmissing(a) | hasmissing(b)) return(0)
    scale = max((abs(vec(a))\abs(vec(b))))
    if (scale == 0) return(1)
    return(max(abs(vec(a:/scale-b:/scale))) <= 1e-11)
}

/* Transport validation is independent of the status supplied by a backend.
   The maintained dense tests separately verify the covariance mathematics. */
real scalar vckss_nmc__stata_validate(real scalar probes, real scalar gate)
{
    real matrix cond, lev, raw, usable, rhs, A
    real rowvector se, meta, old
    struct vckss_nmc__covariance scalar checked
    real scalar n, successful, i
    string scalar status
    cond=st_matrix(st_global("VCKSS_NMC_COND"))
    lev=st_matrix(st_global("VCKSS_NMC_LEV"))
    raw=st_matrix(st_global("VCKSS_NMC_RAW"))
    usable=st_matrix(st_global("VCKSS_NMC_ALL"))
    se=st_matrix(st_global("VCKSS_NMC_SE"))
    meta=st_matrix(st_global("VCKSS_NMC_META"))
    rhs=st_matrix(st_global("VCKSS_NMC_RHS"))
    status=st_global("VCKSS_NMC_STATUS")
    if (rows(cond)!=3 | cols(cond)!=3 | rows(lev)!=3 | cols(lev)!=3 |
        rows(raw)!=3 | cols(raw)!=3 | rows(usable)!=3 | cols(usable)!=3 |
        rows(se)!=1 | cols(se)!=4 | rows(meta)!=1 | cols(meta)!=15 |
        (rows(rhs)>0 & cols(rhs)!=3)) return(0)
    if (status=="exact_zero") {
        if (st_global("e(algorithm)")!="exact") return(0)
        return(!hasmissing((cond,lev,raw,usable)) & !hasmissing(se) &
            max(abs(vec((cond,lev,raw,usable))))==0 & max(abs(se))==0 &
            all(meta[1..9]:==0) & meta[13]==0 & rows(rhs)==0 &
            missing(meta[15]) & st_global("VCKSS_NMC_FAILURE")=="")
    }
    if (!anyof(("ok_local","ok_local_psd_adjusted","unstable_nonpsd",
        "nonsmooth_adjustment","nonfinite_derivative","replay_failed"),status)) return(0)
    if (probes<2 | probes!=floor(probes) | meta[1]!=probes | meta[2]!=probes |
        meta[3]!=ceil(probes/2) | meta[4]!=floor(probes/2) |
        hasmissing(meta[1..8]) | min(meta[1..8])<0 |
        any(meta[1..8]:!=floor(meta[1..8])) | meta[5]!=rows(rhs) |
        meta[5]>probes | meta[6]>probes | meta[6]<meta[5] |
        missing(meta[13]) | meta[13]<0 | meta[13]>gate) return(0)
    n=rows(rhs)
    for (i=1;i<=n;i++) {
        if (rhs[i,1]!=1 | rhs[i,2]!=i-1 | missing(rhs[i,3]) |
            rhs[i,3]<0 | rhs[i,3]>gate) return(0)
    }
    if (n>0) {
        if (meta[13]!=max(rhs[.,3])) return(0)
    }
    else if (meta[13]!=0) return(0)
    if (hasmissing(cond) | !vckss_nmc__relative_equal(cond,cond')) return(0)
    old=st_matrix("e(numerical_mcse)")
    A=(1,0,0\0,1,0\0,0,1\1,1,2)
    if (!vckss_nmc__relative_equal(diagonal(A*cond*A')',old:^2)) return(0)
    successful=anyof(("ok_local","ok_local_psd_adjusted","unstable_nonpsd"),status)
    if (successful) {
        if (meta[5]!=probes | meta[6]!=probes | !missing(meta[15]) |
            st_global("VCKSS_NMC_FAILURE")!="" |
            hasmissing(meta[9..12]) | meta[9]<0 | min(meta[10..11])<=0 |
            meta[12]<0 | hasmissing((lev,raw)) |
            !vckss_nmc__relative_equal(lev,lev') |
            !vckss_nmc__relative_equal(raw,cond+lev)) return(0)
        checked=vckss_nmc__finalize(cond,lev)
        if (checked.status!=status |
            !vckss_nmc__relative_equal(checked.raw,raw) |
            !vckss_nmc__relative_equal(meta[9],checked.psd_adjustment)) return(0)
        if (status=="unstable_nonpsd") {
            return(all(vec(usable):==.) & all(se:==.))
        }
        return(vckss_nmc__relative_equal(usable,checked.usable) &
            vckss_nmc__relative_equal(se,checked.mcse) &
            vckss_nmc__relative_equal(meta[9],checked.psd_adjustment))
    }
    if (!all(vec(usable):==.) | !all(se:==.)) return(0)
    if (status=="replay_failed") {
        return(meta[6]>meta[5] & meta[6]<=min((meta[5]+4,probes)) & meta[15]==meta[5] &
            anyof(("PCG_BREAKDOWN","PCG_NONCONVERGENCE","SOLVER_RESIDUAL_FAILED",
                "PRECONDITIONER_BREAKDOWN","PULLBACK_BREAKDOWN",
                "CMG_APPLY_FAILED","PCG_BREAKDOWN_CURVATURE","PCG_BREAKDOWN_PRECONDITIONER",
                "PCG_STAGNATION","PCG_MAXITER","FULL_RESIDUAL_FAILED"),st_global("VCKSS_NMC_FAILURE")))
    }
    if (status=="nonfinite_derivative") {
        if (!(meta[5]==0 | meta[5]==probes) | meta[6]!=meta[5] |
            !missing(meta[15]) | st_global("VCKSS_NMC_FAILURE")!="") return(0)
        if (!hasmissing((lev,raw))) {
            if (!vckss_nmc__relative_equal(lev,lev') |
                !vckss_nmc__relative_equal(raw,cond+lev)) return(0)
        }
        return(1)
    }
    return(meta[5]==0 & meta[6]==0 & missing(meta[15]) &
        st_global("VCKSS_NMC_FAILURE")=="")
}


end
