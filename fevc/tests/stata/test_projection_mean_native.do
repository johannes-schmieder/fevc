version 18.0
clear all
set more off
set varabbrev off

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
fevc_rust probe
assert r(projection_centering_api)==1

// Match the independent dense oracle fixture in test_projection_mean.do.
set obs 168
generate long worker = cond(_n<=144,floor((_n-1)/12)+1,13+floor((_n-145)/6))
generate byte firm = cond(_n<=144,floor(mod(_n-1,12)/3)+1,1+floor((_n-145)/6))
generate double c1 = sin(_n*.53)
generate double z = sin(worker/3)+firm*.3+c1*.05
generate byte copies = 1+mod(_n+floor(_n/7),3)
generate double target_mass = .5+mod(_n*7,11)/5
generate double noise = (1+.35*firm+.25*mod(_n,3))*(sin(_n*1.7)+cos(_n*.61))
generate double y = .1+.06*worker-.15*firm+.4*c1+noise
generate double shifted = y+20

local cells=0
set seed 721946
local rng_before `"`c(rngstate)'"'
foreach deletion in observation match {
    foreach population in both movers {
        foreach nuisance in joint fixedoffset {
            local weight frequency
            local effect firm
            if "`nuisance'"=="fixedoffset" {
                local weight target
                local effect worker
            }
            local common worker(worker) firm(firm) deletion(`deletion') ///
                stayers(`population') nuisance(`nuisance') targetweight(target_mass) ///
                project(z) projecteffect(`effect') projectweight(`weight') ///
                tolerance(1e-12) mcse(off) nodisplay
            local native algorithm(jla) engine(generic) backend(rust) ///
                rng(counter_v1) seed(721946) probes(600) batch(8)
            display as text "NATIVE CELL `deletion' `population' `nuisance'"
            quietly fevc y c1 [fw=copies], `common' algorithm(exact) backend(mata)
            matrix exact_b=e(projection_b)
            matrix exact_V=e(projection_V)
            matrix exact_naive=e(projection_V_naive)
            generate byte exact_sample=e(sample)

            quietly fevc y c1 [fw=copies], `common' `native' preconditioner(diagonal)
            assert "`e(centering)'"=="mean"
            assert "`e(backend_selected)'"=="rust"
            assert "`e(status)'"=="KSS_PROJECTION_INFERENCE"
            assert "`e(inference_method)'"=="sparse JLA block cross-fit projection"
            assert e(sample)==exact_sample
            drop exact_sample
            matrix default_b=e(projection_b)
            matrix default_V=e(projection_V)
            matrix default_naive=e(projection_V_naive)
            assert mreldif(exact_b,default_b)<2e-8
            assert mreldif(exact_naive,default_naive)<2e-8
            // Predeclared existing projection integration bound for a fixed
            // 600-probe realization; solver-equivalence gates below are tight.
            assert mreldif(exact_V,default_V)<.005
            assert e(projection_solver_max_complete)<=e(residual_acceptance_tolerance)
            fevc_rust snapshot
            assert r(state)==0 & r(handle)==0

            quietly fevc y c1 [fw=copies], `common' `native' ///
                preconditioner(diagonal) centering(mean)
            assert mreldif(default_b,e(projection_b))<1e-12
            assert mreldif(default_V,e(projection_V))<1e-12
            assert mreldif(default_naive,e(projection_V_naive))<1e-12

            quietly fevc y c1 [fw=copies], `common' `native' ///
                preconditioner(cmg) centering(mean)
            assert "`e(preconditioner_selected)'"=="CMG"
            assert mreldif(default_b,e(projection_b))<2e-8
            assert mreldif(default_V,e(projection_V))<2e-8
            assert mreldif(default_naive,e(projection_V_naive))<2e-8
            assert e(projection_solver_max_complete)<=e(residual_acceptance_tolerance)

            quietly fevc y c1 [fw=copies], `common' `native' ///
                preconditioner(diagonal) centering(none)
            assert mreldif(default_b,e(projection_b))<2e-8
            assert mreldif(default_naive,e(projection_V_naive))<2e-8

            quietly fevc shifted c1 [fw=copies], `common' `native' ///
                preconditioner(diagonal) centering(mean)
            assert mreldif(default_V,e(projection_V))<2e-8
            assert mreldif(default_naive,e(projection_V_naive))<2e-8
            matrix shifted_b=e(projection_b)
            assert abs(default_b[1,2]-shifted_b[1,2])<2e-8
            fevc_rust snapshot
            assert r(state)==0 & r(handle)==0
            local ++cells
        }
    }
}
assert `cells'==8
assert `"`c(rngstate)'"'==`"`rng_before'"'
assert "$VCKSS_CENTERING"==""
di as result "PASS test_projection_mean_native.do cells=`cells'"
exit 0
