version 18.0

/* Independent dense point expressions and derivatives. These tests do not
   equate Stata RNG seeds with the native Counter-V1 contract. */
mata:
real rowvector fevc_test_nmc_terms(real rowvector u, real scalar R)
{
    real scalar h, m, B, V
    h = u[1]/(u[1]+u[2]); m = 1-h
    B = (m*u[3]-h*u[4]+(m-h)*u[5])/R
    V = (m^2*u[3]+h^2*u[4]-2*h*m*u[5])/R
    return((h,m,B,V))
}

real scalar fevc_test_nmc_inverse(real rowvector u, real scalar R)
{
    real rowvector terms
    real scalar ell
    terms = fevc_test_nmc_terms(u,R)
    ell = terms[2]-.02
    return(1/ell+terms[3]/ell^2-terms[4]/ell^3)
}

real colvector fevc_test_nmc_block(real rowvector u, real scalar R,
    real matrix C, real colvector v, real colvector e)
{
    real matrix inverse
    real colvector a, w
    real rowvector terms
    terms = fevc_test_nmc_terms(u,R)
    inverse = luinv(I(rows(v))-C-terms[1]*v*v')
    a = inverse*e; w = inverse*v
    return(a+(terms[3]-terms[4]*(v'*w)[1]):*w*(v'*a)[1])
}

void fevc_test_nmc_math()
{
    struct vckss_nmc__finite scalar finite
    struct vckss_nmc__covariance scalar result
    real matrix C, direct, a, b, saved_a, saved_b, conditional, leverage
    real colvector v, e, w, av
    real rowvector u, up, down, derivative, pullback, lambda
    real scalar R, j, step, scale, k, t, multiplier
    real matrix batch_u, batch_gradient
    assert(vckss_nmc__api_level() == 1)
    C = (.1,.02\-.03,.07\.05,-.01\.02,.04)
    C = C*C'
    v = sqrt((1\2\3\1)/7)
    e = (.7\-.2\.4\1.1)
    for (t=1; t<=3; t++) {
        if (t == 1) {
            R = 3
            u = (.21,.77,.061,.68,.048)
        }
        if (t == 2) {
            R = 32
            u = (.31,.71,.17,.81,.097)
        }
        if (t == 3) {
            R = 200
            u = (.17,.86,.032,.93,.040)
        }
        finite = vckss_nmc__finite_derivative(u,R)
        assert(finite.status == "ok_local")
        derivative = J(1,5,.)
        direct = J(4,5,.)
        for (j=1; j<=5; j++) {
            /* Interior centered differences balance cancellation and O(h^2)
               truncation; the independent Python complex-step gate is tighter. */
            step = 1e-5*abs(u[j])
            up = down = u
            up[j] = up[j]+step; down[j] = down[j]-step
            derivative[j] = (fevc_test_nmc_inverse(up,R)-
                fevc_test_nmc_inverse(down,R))/(2*step)
            direct[.,j] = (fevc_test_nmc_block(up,R,C,v,e)-
                fevc_test_nmc_block(down,R,C,v,e))/(2*step)
        }
        lambda = vckss_nmc__observation_gradient(finite,.02)
        assert(vckss__norm2(lambda-derivative)/vckss__norm2(derivative) < 1e-9)
        w = luinv(I(4)-C-finite.h*v*v')*v
        av = luinv(I(4)-C-finite.h*v*v')*e
        k = (v'*w)[1]
        lambda = vckss_nmc__block_gradient(finite,k)
        assert(vckss__norm2(w*(v'*av)[1]*lambda-direct)/vckss__norm2(direct) < 1e-9)
        pullback = vckss_nmc__copy_pullback((1,2,3,4,5))
        assert(max(abs(pullback-(32,12,-20,-26))) == 0)
    }
    batch_u = (.21,.77,.061,.68,.048\
        .31,.71,.17,.81,.097\.17,.86,.032,.93,.040)
    finite = vckss_nmc__finite_derivative(batch_u,200)
    assert(finite.status=="ok_local")
    batch_gradient = vckss_nmc__observation_gradient(finite,J(3,1,.02))
    for (t=1; t<=rows(batch_u); t++) {
        for (j=1; j<=5; j++) {
            u = batch_u[t,.]; step = 1e-5*abs(u[j])
            up = down = u; up[j] = up[j]+step; down[j] = down[j]-step
            derivative[j] = (fevc_test_nmc_inverse(up,200)-
                fevc_test_nmc_inverse(down,200))/(2*step)
        }
        assert(vckss__norm2(batch_gradient[t,.]-derivative)/
            vckss__norm2(derivative)<1e-9)
    }
    a = (1,.2,-.3\-.4,.7,.2\.9,-.2,.8\-.7,.3,-.5\0,.1,-.2)
    b = (.2,.5,.4\-.8,.2,.7\.5,-.1,.3\-.1,.9,-.6\.7,-.3,.1)
    saved_a = a; saved_b = b
    conditional = vckss_nmc__cross_covariance(a,a)
    leverage = vckss_nmc__cross_covariance(a,b)
    direct = ((a:-mean(a))'*(b:-mean(b))+(b:-mean(b))'*(a:-mean(a)))/(2*5*4)
    assert(vckss__norm2(leverage-direct)/vckss__norm2(direct) < 1e-11)
    assert(max(abs(a-saved_a)) == 0 & max(abs(b-saved_b)) == 0)
    result = vckss_nmc__finalize(conditional,leverage)
    assert(result.status == "ok_local")
    assert(abs(result.mcse[4]^2-(result.usable[1,1]+result.usable[2,2]+
        4*result.usable[3,3]+2*result.usable[1,2]+4*result.usable[1,3]+
        4*result.usable[2,3])) < 1e-14)
    for (t=1; t<=2; t++) {
        multiplier = t == 1 ? 1e-150 : 1e150
        scale = multiplier^2
        result = vckss_nmc__finalize(conditional:*scale,leverage:*scale)
        assert(result.status == "ok_local")
        assert(vckss__norm2(result.raw:/scale-conditional-leverage)/
            vckss__norm2(conditional+leverage) < 1e-11)
    }
    result = vckss_nmc__finalize(I(3),diag((-.1,-2,0)))
    assert(result.status == "unstable_nonpsd" & hasmissing(result.usable))
    assert(result.raw[2,2] == -1)
    result = vckss_nmc__finalize(I(3),diag((0,-1-1e-13,0)))
    assert(result.status == "ok_local_psd_adjusted" & result.raw[2,2] < 0)
    assert(result.usable[2,2] == 0 & result.psd_adjustment > 0)
    result = vckss_nmc__finalize(J(3,3,0),J(3,3,0))
    assert(result.status == "ok_local" & max(abs(result.mcse)) == 0)
    result = vckss_nmc__finalize(I(3),J(3,3,.))
    assert(result.status == "nonfinite_derivative" & hasmissing(result.usable))
    finite = vckss_nmc__finite_derivative((.5,.5,.1,.1,1),32)
    assert(finite.status == "nonsmooth_adjustment")
    assert(hasmissing(vckss_nmc__observation_gradient(finite,0)))
}
fevc_test_nmc_math()
end

// Repeated source loads must reuse matching snapshot definitions and preserve
// every registered caller RNG field, including inactive domain streams.
mata: fevc_nmc_saved_rng = vckss_rng__capture_full()
quietly findfile fevc_rng.mata
quietly do `"`r(fn)'"'
mata:
fevc_nmc_after_rng = vckss_rng__capture_full()
assert(fevc_nmc_saved_rng.active.algorithm == fevc_nmc_after_rng.active.algorithm)
assert(fevc_nmc_saved_rng.active.stream == fevc_nmc_after_rng.active.stream)
assert(fevc_nmc_saved_rng.active.state == fevc_nmc_after_rng.active.state)
assert(fevc_nmc_saved_rng.sort_state == fevc_nmc_after_rng.sort_state)
assert(fevc_nmc_saved_rng.mt64s_stream1_state == fevc_nmc_after_rng.mt64s_stream1_state)
assert(fevc_nmc_saved_rng.mt64s_stream2_state == fevc_nmc_after_rng.mt64s_stream2_state)
assert(fevc_nmc_saved_rng.mt64s_selected_stream_state == fevc_nmc_after_rng.mt64s_selected_stream_state)
end

di as result "PASS test_all_probe_math.do"
