version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

set obs 96
generate double worker = floor((_n - 1) / 8) + 1
generate double firm = mod(floor((_n - 1) / 2), 4) + 1
generate double deletion = 4 * (worker - 1) + firm
generate double outcome = cond(mod(worker + firm, 2) == 0, 1, -1) * ///
    (2 * (worker - 1) + firm) + cond(mod(_n, 2), .25, -.25)
generate double frequency = mod(firm - 1, 2) + 1
generate double target_weight = mod(worker - 1, 3) + firm

varcomp_kss_rust clear
varcomp_kss_rust prepare worker firm deletion outcome frequency          ///
    target_weight, cleanup generate(rust_keep)
local handle = r(handle)
assert rust_keep == 1
varcomp_kss_rust solve `handle', seed(91827) probes(200)                 ///
    leveragebatch(8) targetbatch(8) route(diagonal)
varcomp_kss_rust result `handle'
matrix rust_result = r(result)
varcomp_kss_rust release `handle'

varcomp_kss outcome [fw=frequency], worker(worker) firm(firm)            ///
    deletion(match) deletionid(deletion) algorithm(jla) probes(200)      ///
    batch(8) seed(91827) preconditioner(diagonal)                        ///
    targetweight(target_weight) nodisplay
matrix mata_result = e(results)

/* Plugin components contain no estimator randomness and must agree tightly.
   Corrections deliberately use different registered RNG contracts.  Their
   comparison is therefore a numerical diagnostic against combined MCSE, not
   a fixed-seed parity or public-backend qualification claim. */
forvalues component = 1/4 {
    assert abs(rust_result[1,`component'] - mata_result[1,`component'])   ///
        <= 1e-9
    scalar combined_mcse = sqrt(rust_result[4,`component']^2 +           ///
        mata_result[4,`component']^2)
    scalar correction_gap = abs(rust_result[2,`component'] -            ///
        mata_result[2,`component'])
    display as txt "RUST_MATA_DIAGNOSTIC component=`component' gap="    ///
        correction_gap " combined_mcse=" combined_mcse
    assert correction_gap <= 8 * combined_mcse + 1e-10
}

display as result "VARCOMP_KSS RUST MATA DIAGNOSTIC PASS"
exit 0
