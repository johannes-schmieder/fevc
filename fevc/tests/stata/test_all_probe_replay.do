version 18.0

/* Small dense test oracle. Physical-copy-by-probe caches exist only here.
   It builds full design/maker matrices and differentiates the complete point
   expression independently; production uses bounded reverse contractions. */
mata:
struct fevc_test_nmc_reference
{
    real matrix conditional
    real matrix leverage
    real rowvector correction
}

real colvector fevc_test_nmc_deleted(real rowvector u, real scalar R,
    real matrix C, real colvector e)
{
    real scalar h, m, B, V, ell
    real matrix K
    real colvector v, a, w
    h = u[1]/sum(u[1..2]); m = u[2]/sum(u[1..2])
    B = (m*u[3]-h*u[4]+(m-h)*u[5])/R
    V = (m^2*u[3]+h^2*u[4]-2*h*m*u[5])/R
    if (rows(e) == 1) {
        ell = m-C[1,1]
        return(e:*(1/ell+B/ell^2-V/ell^3))
    }
    v = J(rows(e),1,1/sqrt(rows(e)))
    K = luinv(I(rows(e))-C-h*v*v')
    a = K*e; w = K*v
    return(a+(B-V*(v'*w)[1]):*w*(v'*a)[1])
}

struct fevc_test_nmc_reference scalar fevc_test_nmc_dense(
    real colvector y, real colvector worker, real colvector firm,
    real matrix controls, real colvector frequency, real colvector mass,
    real colvector deletion_id, string scalar deletion, string scalar nuisance,
    real colvector stayer, real scalar R, real scalar seed)
{
    struct fevc_test_nmc_reference scalar out
    real scalar n, physical, i, j, g, r, t, count, step, mover_count
    real colvector row, wi, fi, di, yi, tm, e, q, order, index, stayer_copies
    real colvector sw, sf, d, fit, working_y, yw
    real matrix Xw, Xf, Xfe, X, C, P, Qc, inverse, panel, qcache, pcache
    real matrix draws, scoreA, scoreB, U, D, L, foldA, foldB, Cg
    real rowvector u, up, down, first
    row = J(sum(frequency),1,.)
    j = 1
    for (i=1; i<=rows(y); i++) {
        row[|j\j+frequency[i]-1|] = J(frequency[i],1,i)
        j = j+frequency[i]
    }
    physical = rows(row); wi = worker[row]; fi = firm[row]; yi = y[row]
    tm = (mass:/frequency)[row]; tm = tm:/sum(tm)
    Xw = J(physical,max(worker),0); Xf = J(physical,max(firm)-1,0)
    for (i=1; i<=physical; i++) {
        Xw[i,wi[i]] = 1
        if (fi[i]<max(firm)) Xf[i,fi[i]] = 1
    }
    Xfe = Xw,Xf
    P = Xfe*invsym(Xfe'*Xfe)*Xfe'
    X = Xfe,controls[row,.]
    fit = invsym(X'*X)*X'*yi
    working_y = yi
    C = J(physical,physical,0)
    if (cols(controls)>0) {
        Qc = controls[row,.]-P*controls[row,.]
        if (nuisance == "joint") C = Qc*invsym(Qc'*Qc)*Qc'
        else {
            working_y = yi-controls[row,.]*fit[(cols(Xfe)+1)..rows(fit)]
            X = Xfe
        }
    }
    inverse = invsym(X'*X)
    e = working_y-X*inverse*X'*working_y
    if (deletion == "observation") di = (1..physical)'
    else {
        di = deletion_id[row]
        if (rows(stayer)>0) {
            stayer_copies = selectindex(stayer[row]:==1)
            di[stayer_copies] = max(di):+(1..rows(stayer_copies))'
        }
    }
    order = order(di,1); panel = panelsetup(di[order],1)
    qcache = pcache = J(physical,R,.)
    assert(vckss_rng__set_stream_seed(1,seed) == 0)
    for (r=1; r<=R; r++) {
        if (rows(stayer)>0 & deletion == "match") {
            mover_count = sum(frequency[selectindex(stayer:==0)])
            q = (2:*rbinomial(mover_count,1,1,.5):-1) \
                (2:*rbinomial(physical-mover_count,1,1,.5):-1)
        }
        else q = 2:*rbinomial(physical,1,1,.5):-1
        qcache[.,r] = q; pcache[.,r] = P*q
    }
    draws = J(R,3,0); scoreA = scoreB = J(R,3,0)
    // Test-only group-by-target operators and full forward derivatives.
    for (g=1; g<=rows(panel); g++) {
        index = order[|panel[g,1]\panel[g,2]|]
        U = J(R,5,.)
        for (r=1; r<=R; r++) {
            first = (sum(pcache[index,r]),sum(qcache[index,r])):/sqrt(rows(index))
            first[2] = first[2]-first[1]
            U[r,.] = (first[1]^2,first[2]^2,first[1]^4,first[2]^4,
                first[1]^2*first[2]^2)
        }
        u = mean(U); Cg = C[index,index]
        D = J(rows(index),5,.)
        for (j=1; j<=5; j++) {
            step = 1e-5*abs(u[j])
            assert(step>0)
            up = down = u; up[j] = up[j]+step; down[j] = down[j]-step
            D[.,j] = (fevc_test_nmc_deleted(up,R,Cg,e[index])-
                fevc_test_nmc_deleted(down,R,Cg,e[index]))/(2*step)
        }
        foldA = foldB = J(3,rows(index),0)
        // Resetting the independent target domain reproduces each whole L_t.
        assert(vckss_rng__set_stream_seed(2,seed) == 0)
        for (t=1; t<=R; t++) {
            d = sqrt(tm):*(2:*rbinomial(physical,1,1,.5):-1)
            d = d-tm:*sum(d)
            sw = X*inverse*(Xw'*d\J(cols(X)-cols(Xw),1,0))
            sf = X*inverse*(J(cols(Xw),1,0)\Xf'*d\J(cols(X)-cols(Xfe),1,0))
            if (rows(index) == 1) L = working_y[index]:*(sw[index]^2,sf[index]^2,sw[index]*sf[index])'
            else {
                first = (sum(working_y[index]:*sw[index]),sum(working_y[index]:*sf[index]))
                L = (first[1]:*sw[index],first[2]:*sf[index],
                    (first[1]:*sf[index]+first[2]:*sw[index]):/2)'
            }
            draws[t,.] = draws[t,.]+(L*fevc_test_nmc_deleted(u,R,Cg,e[index]))'
            if (mod(t,2)) foldA = foldA+L:/ceil(R/2)
            else foldB = foldB+L:/floor(R/2)
        }
        scoreA = scoreA-(foldA*D*(U:-u)')'
        scoreB = scoreB-(foldB*D*(U:-u)')'
    }
    out.correction = mean(draws)
    out.conditional = (draws:-mean(draws))'*(draws:-mean(draws))/(R*(R-1))
    out.leverage = ((scoreA:-mean(scoreA))'*(scoreB:-mean(scoreB))+
        (scoreB:-mean(scoreB))'*(scoreA:-mean(scoreA)))/(2*R*(R-1))
    return(out)
}

struct fevc_test_nmc_failure_context
{
    real scalar calls
    real scalar fail_after
    string scalar failure
}

struct vckss_preconditioner_result scalar fevc_test_nmc_exact_apply(
    pointer scalar context, struct vckss_fe_design scalar design, real matrix residual)
{
    pointer(struct fevc_test_nmc_failure_context scalar) scalar counter
    struct vckss_preconditioner_result scalar out
    real matrix S
    counter = context
    (*counter).calls = (*counter).calls+1
    out.status = "CONVERGED"; out.message = "test exact quotient action"
    out.value = J(0,0,.)
    if ((*counter).calls > (*counter).fail_after) {
        out.status = (*counter).failure
        return(out)
    }
    S = vckss__fe_schur_action(design,I(design.firm_levels))
    out.value = invsym(S+J(design.firm_levels,design.firm_levels,
        1/design.firm_levels))*residual
    return(out)
}

void fevc_test_nmc_replay()
{
    struct vckss_fe_design scalar base
    struct vckss_solver_backend scalar backend
    struct vckss_result scalar ordinary, enabled
    struct vckss_nmc__attachment scalar attachment
    struct fevc_test_nmc_reference scalar reference
    struct vckss_rng__stream_snapshot scalar before, after
    struct vckss_rng__full_snapshot scalar caller
    real colvector worker, firm, y, frequency, mass, deletion_id, stayer, empty
    real matrix controls, original_controls
    struct fevc_test_nmc_failure_context scalar counter
    string rowvector failures
    real scalar i, d, nu, hybrid, R, seed, point_calls
    string scalar deletion, nuisance
    R = 33; seed = 2026092907
    empty = J(0,1,.)
    caller = vckss_rng__capture_full()
    for (hybrid=0; hybrid<=1; hybrid++) {
        worker = firm = deletion_id = y = frequency = mass = J(36+6*hybrid,1,.)
        controls = J(rows(y),2,.)
        for (i=1; i<=36; i++) {
            worker[i] = floor((i-1)/9)+1
            firm[i] = floor(mod(i-1,9)/3)+1
            deletion_id[i] = 3*(worker[i]-1)+firm[i]
        }
        if (hybrid) {
            worker[37..42] = J(6,1,5); firm[37..42] = J(6,1,1)
            deletion_id[37..42] = J(6,1,13)
            stayer = J(36,1,0)\J(6,1,1)
        }
        else stayer = empty
        for (i=1; i<=rows(y); i++) {
            controls[i,.] = (sin(1.7*i),cos(.31*i))
            y[i] = .4*worker[i]-.2*firm[i]+.8*sin(1.7*i)+.17*cos(2.1*i)
            frequency[i] = mod(i,3)+1
            mass[i] = frequency[i]*(.4+mod(i,5)/3)
        }
        original_controls = controls
        base = vckss__fe_prepare(worker,firm,frequency,1e-10)
        assert(base.status == "CONVERGED")
        backend = vckss__diagonal_backend()
        for (d=1; d<=2; d++) {
            deletion = d==1 ? "observation" : "match"
            for (nu=1; nu<=3; nu++) {
                nuisance = nu==2 ? "fixedoffset" : "joint"
                controls = nu==3 ? J(rows(y),0,.) : original_controls
                ordinary = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
                    deletion_id,deletion,nuisance,R,7,seed,1e-12,2000,1e-10,1e-10,
                    100,base,backend,0,empty,0,
                    hybrid & deletion == "match" ? stayer : empty)
                assert(ordinary.status == "CONVERGED")
                before = vckss_rng__capture_streams((1\2))
                controls = nu==3 ? J(rows(y),0,.) : original_controls
                enabled = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
                    deletion_id,deletion,nuisance,R,7,seed,1e-12,2000,1e-10,1e-10,
                    100,base,backend,0,empty,0,
                    hybrid & deletion == "match" ? stayer : empty,&attachment)
                after = vckss_rng__capture_streams((1\2))
                assert(enabled.status == "CONVERGED")
                assert(vckss__norm2(ordinary.corrected-enabled.corrected)/
                    vckss__norm2(ordinary.corrected) < 1e-11)
                assert(vckss__norm2(ordinary.numerical_mcse-enabled.numerical_mcse)/
                    vckss__norm2(ordinary.numerical_mcse) < 1e-11)
                assert(before.active.algorithm == after.active.algorithm &
                    before.active.stream == after.active.stream &
                    before.active.state == after.active.state & all(before.state:==after.state))
                assert(rows(attachment.replay_rhs)==R & attachment.replay_failure == "")
                assert(max(attachment.replay_rhs[.,3])<=1e-11)
                assert(attachment.replay_generator_evaluations == R*sum(frequency))
                reference = fevc_test_nmc_dense(y,worker,firm,
                    nu==3 ? J(rows(y),0,.) : original_controls,
                    frequency,mass,deletion_id,deletion,nuisance,
                    hybrid & deletion == "match" ? stayer : empty,R,seed)
                assert(vckss__norm2(reference.correction-enabled.correction[1..3])/
                    vckss__norm2(reference.correction) < 1e-11)
                assert(vckss__norm2(reference.conditional-attachment.covariance.conditional)/
                    vckss__norm2(reference.conditional) < 1e-11)
                assert(vckss__norm2(reference.leverage-attachment.covariance.leverage)/
                    vckss__norm2(reference.leverage) < 1e-9)
                // A wider admitted point batch exercises the eight-direction replay cap.
                controls = nu==3 ? J(rows(y),0,.) : original_controls
                enabled = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
                    deletion_id,deletion,nuisance,R,11,seed,1e-12,2000,1e-10,1e-10,
                    100,base,backend,0,empty,0,
                    hybrid & deletion == "match" ? stayer : empty,&attachment)
                after = vckss_rng__capture_streams((1\2))
                assert(enabled.status == "CONVERGED" & rows(attachment.replay_rhs)==R)
                assert(before.active.state == after.active.state & all(before.state:==after.state))
                assert(vckss__norm2(ordinary.corrected-enabled.corrected)/
                    vckss__norm2(ordinary.corrected) < 1e-11)
                assert(vckss__norm2(ordinary.numerical_mcse-enabled.numerical_mcse)/
                    vckss__norm2(ordinary.numerical_mcse) < 1e-11)
                assert(vckss__norm2(reference.leverage-attachment.covariance.leverage)/
                    vckss__norm2(reference.leverage) < 1e-9)
                controls = nu==3 ? J(rows(y),0,.) : original_controls
                enabled = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
                    deletion_id,deletion,nuisance,R,1,seed,1e-12,2000,1e-10,1e-10,
                    100,base,backend,0,empty,0,
                    hybrid & deletion == "match" ? stayer : empty,&attachment)
                assert(vckss__norm2(reference.leverage-attachment.covariance.leverage)/
                    vckss__norm2(reference.leverage) < 1e-9)
            }
        }
    }
    // Inject failures only after every valid point RHS has completed.
    counter.calls = 0; counter.fail_after = .; counter.failure = ""
    backend.context = &counter; backend.apply = &fevc_test_nmc_exact_apply()
    backend.exact_inverse = 1
    controls = original_controls
    ordinary = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
        deletion_id,"observation","joint",R,7,seed,1e-12,2000,1e-10,1e-10,
        100,base,backend,0)
    assert(ordinary.status == "CONVERGED")
    point_calls = counter.calls
    before = vckss_rng__capture_streams((1\2))
    failures = ("PCG_BREAKDOWN","PCG_NONCONVERGENCE","SOLVER_RESIDUAL_FAILED",
        "PRECONDITIONER_BREAKDOWN","PULLBACK_BREAKDOWN","INVALID_PRECONDITIONER_ACTION")
    for (i=1; i<=cols(failures); i++) {
        counter.calls = 0; counter.fail_after = point_calls; counter.failure = failures[i]
        controls = original_controls
        enabled = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
            deletion_id,"observation","joint",R,7,seed,1e-12,2000,1e-10,1e-10,
            100,base,backend,0,empty,0,empty,&attachment)
        after = vckss_rng__capture_streams((1\2))
        assert(before.active.state == after.active.state & all(before.state:==after.state))
        assert(attachment.failed_replay_probe==1 & attachment.replay_failure==failures[i])
        if (i<=5) {
            assert(enabled.status == "CONVERGED" & attachment.covariance.status=="replay_failed")
            assert(hasmissing(attachment.covariance.usable) & rows(attachment.replay_rhs)==0)
            assert(vckss__norm2(enabled.corrected-ordinary.corrected)<1e-11*vckss__norm2(ordinary.corrected))
        }
        else assert(enabled.status == failures[i])
    }
    counter.calls = 0; counter.fail_after = .
    controls = original_controls
    enabled = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
        deletion_id,"observation","joint",R,7,seed,1e-12,2000,1e-10,1e-10,
        100,base,backend,0,empty,0,empty,&attachment)
    assert(enabled.status == "CONVERGED" & rows(attachment.replay_rhs)==R)
    // A frequency-heavy observation profile exercises O(copies) scratch.
    frequency = J(rows(y),1,1001)
    mass = frequency:*(.4:+mod((1..rows(y))',5):/3)
    base = vckss__fe_prepare(worker,firm,frequency,1e-10)
    backend = vckss__diagonal_backend()
    controls = original_controls
    enabled = vckss__jla_backend(y,worker,firm,controls,frequency,mass,
        deletion_id,"observation","joint",R,7,seed,1e-12,2000,1e-10,1e-10,
        100,base,backend,0,empty,0,empty,&attachment)
    assert(enabled.status == "CONVERGED" & rows(attachment.replay_rhs)==R)
    assert(attachment.allocation_bound_bytes>=128*sum(frequency)+512*rows(y)+512*R+4096)
    assert(vckss_rng__restore_full(caller)==0)
}
fevc_test_nmc_replay()
end

di as result "PASS test_all_probe_replay.do"
