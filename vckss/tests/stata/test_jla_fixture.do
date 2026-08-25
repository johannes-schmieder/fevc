version 18.0
clear
set obs 24
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time - 1.5
generate double c2 = time == 2
generate byte firm = .
generate long match = .
generate double noise = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise

// Frozen Stata-RNG/Mata numerical fixture. Native Counter-V1 public-result
// qualification is intentionally separate in test_rust_public_generic.do.
vckss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) nuisance(joint) ///
    probes(2000) seed(20260814) tolerance(1e-12) ///
    backend(mata) rng(stata) nodisplay
assert "`e(algorithm)'" == "jla"
assert "`e(deletion_rank_certificate)'" == ///
    "full-fit within-cell trace and direct factor gates"
assert e(probes) == 2000
assert e(full_parameters) == 11
assert e(correction_parameters) == 11
assert e(numerical_mcse_available) == 1
assert missing(e(information_rcond))
assert e(preconditioner_ratio) > 0 & e(preconditioner_ratio) <= 1
assert e(setup_seconds) == e(preconditioner_seconds)
assert e(schur_seconds) >= 0
assert e(preconditioner_apply_seconds) >= 0
assert e(pcg_seconds) >= e(schur_seconds)
assert rowsof(e(solver_rhs_diagnostics)) > 0
assert colsof(e(solver_rhs_diagnostics)) == 6
assert e(control_schur_rcond) > 0 & e(control_schur_rcond) <= 1
assert e(deletion_rank_gap) > 0 & e(deletion_rank_gap) <= 1
assert e(inverse_relres) < 1e-10
assert e(solver_max_residual) < 1e-10
assert abs(el(e(plugin),1,1) - .23886949630417731) < 2e-10
assert abs(el(e(plugin),1,4) - .21441326290890825) < 2e-10
assert abs(el(e(correction),1,1) + .019250119561150449) < ///
    5*el(e(numerical_mcse),1,1) + 2e-4
assert abs(el(e(correction),1,2) + .0078437874259815812) < ///
    5*el(e(numerical_mcse),1,2) + 2e-4
assert abs(el(e(correction),1,3) + .0043493658055976927) < ///
    5*el(e(numerical_mcse),1,3) + 2e-4
assert abs(el(e(correction),1,4) + .035792638598327396) < ///
    5*el(e(numerical_mcse),1,4) + 2e-4
assert abs(el(e(correction),1,4) - ///
    (el(e(correction),1,1)+el(e(correction),1,2)+ ///
    2*el(e(correction),1,3))) < 2e-12
matrix jla_reference = e(results)

vckss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) nuisance(joint) ///
    probes(2000) batch(1) seed(20260814) tolerance(1e-12) ///
    backend(mata) rng(stata) nodisplay
assert mreldif(jla_reference,e(results)) < 1e-14

gsort -worker -time
vckss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) nuisance(joint) ///
    probes(2000) batch(17) seed(20260814) tolerance(1e-12) ///
    backend(mata) rng(stata) nodisplay
assert mreldif(jla_reference,e(results)) < 1e-14

vckss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) nuisance(joint) ///
    probes(2000) seed(20260815) tolerance(1e-12) ///
    backend(mata) rng(stata) nodisplay
assert mreldif(jla_reference,e(results)) > 1e-8

vckss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) nuisance(fixedoffset) ///
    probes(2000) seed(20260814) tolerance(1e-12) ///
    backend(mata) rng(stata) nodisplay
assert e(full_parameters) == 11
assert e(correction_parameters) == 9
assert "`e(deletion_rank_certificate)'" == ///
    "full-fit within-cell trace and direct factor gates"
assert e(deletion_rank_gap) > 0 & e(deletion_rank_gap) <= 1
assert abs(el(e(plugin),1,1) - .23886949630417761) < 2e-10
assert abs(el(e(correction),1,1) + .00467505214166742) < ///
    5*el(e(numerical_mcse),1,1) + 3e-4
assert abs(el(e(correction),1,4) + .01849902060699364) < ///
    5*el(e(numerical_mcse),1,4) + 3e-4

vckss y c1 c2, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) nuisance(joint) probes(2000) seed(20260814) ///
    tolerance(1e-12) backend(mata) rng(stata) nodisplay
assert abs(el(e(correction),1,1) - .017319153327137557) < ///
    5*el(e(numerical_mcse),1,1) + 3e-4
assert abs(el(e(correction),1,4) - .01816335608515773) < ///
    5*el(e(numerical_mcse),1,4) + 3e-4

di as result "PASS test_jla_fixture.do"
