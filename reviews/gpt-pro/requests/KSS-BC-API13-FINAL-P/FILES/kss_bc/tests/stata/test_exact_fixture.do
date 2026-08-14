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

kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nuisance(joint) nodisplay

assert "`e(cmd)'" == "kss_bc"
assert "`e(algorithm)'" == "exact"
assert "`e(deletion)'" == "match"
assert "`e(nuisance)'" == "joint"
assert "`e(target_population)'" == "movers"
assert "`e(deletion_rank_certificate)'" == ///
    "dense Woodbury plus direct rank gate"
assert e(N) == 24
assert e(N_stored) == 24
assert e(deletion_units) == 17
assert e(numerical_mcse_available) == 0
assert e(information_rcond) > 0
assert e(full_parameters) == 11
assert e(correction_parameters) == 11
assert e(parameters) == e(correction_parameters)
assert missing(e(preconditioner_ratio))
assert missing(e(control_schur_rcond))
assert missing(e(deletion_rank_gap))
assert e(inverse_relres) < 1e-10
assert e(solver_max_residual) < 1e-10
assert abs(el(e(plugin),1,1) - .23886949630417731) < 2e-11
assert abs(el(e(plugin),1,2) - .029429019126843384) < 2e-11
assert abs(el(e(plugin),1,3) + .026942626261056212) < 2e-11
assert abs(el(e(plugin),1,4) - .21441326290890825) < 2e-11
assert abs(el(e(correction),1,1) + .019250119561150449) < 2e-10
assert abs(el(e(correction),1,2) + .0078437874259815812) < 2e-10
assert abs(el(e(correction),1,3) + .0043493658055976927) < 2e-10
assert abs(el(e(correction),1,4) + .035792638598327396) < 2e-10
assert abs(el(e(kss),1,4) - ///
    (el(e(kss),1,1)+el(e(kss),1,2)+2*el(e(kss),1,3))) < 2e-12
matrix define kss_copy = e(kss)
matrix define b_copy = e(b)
assert mreldif(kss_copy,b_copy) < 1e-14
capture matrix list e(V)
assert _rc != 0

kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nuisance(fixedoffset) nodisplay
assert e(full_parameters) == 11
assert e(correction_parameters) == 9
assert e(parameters) == e(correction_parameters)
assert abs(el(e(plugin),1,1) - .23886949630417761) < 2e-11
assert abs(el(e(correction),1,1) + .00467505214166742) < 2e-10
assert abs(el(e(correction),1,2) - .00105611329461117) < 2e-10
assert abs(el(e(correction),1,3) + .00744004087996870) < 2e-10
assert abs(el(e(correction),1,4) + .01849902060699364) < 2e-10

kss_bc y c1 c2, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) nuisance(joint) nodisplay
assert e(deletion_units) == 24
assert abs(e(max_leverage) - .60288021133431635) < 2e-10
assert abs(el(e(correction),1,1) - .017319153327137557) < 2e-10

di as result "PASS test_exact_fixture.do"
