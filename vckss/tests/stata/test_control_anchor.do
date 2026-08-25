version 18.0
clear
set more off
set varabbrev off

capture findfile vckss.mata
assert _rc == 0
quietly do `"`r(fn)'"'
capture quietly vckss_rust probe
local rust_available = (_rc==0)
if `rust_available' capture quietly vckss_rust clear

// An exact score at the fuzzy eligibility boundary used by API 11 could
// choose different first anchors after an invertible basis transformation.
// API 13 must fail closed unless both accepted canonical matrices agree.
mata:
Q = (sqrt(.7999999),0 \ 0,sqrt(.8) \ sqrt(.2000001),0 \ 0,sqrt(.2))
T = (1,2 \ 0,1)
frequency = J(4,1,1)
base = vckss__canonical_controls(Q,frequency,1e-10)
changed = vckss__canonical_controls(Q*T,frequency,1e-10)
assert(base.status == "AMBIGUOUS_CONTROL_BASIS")
assert(changed.status == "AMBIGUOUS_CONTROL_BASIS")
too_many = vckss__canonical_controls(I(33),J(33,1,1),1e-10)
assert(too_many.status == "AMBIGUOUS_CONTROL_BASIS")
end

// Reviewer M's eight-row API-12 witness.  Raw-control sorting made exact and
// auto-to-exact choose different physical anchors under this determinant-four
// map; native API 12 accepted both and returned gaps near 1.4e-9.  Every
// controlled backend now receives the same ID-free semantic row order.  The
// original d=.002 design is too ill-conditioned downstream to certify its
// final result and must be withheld consistently in every representation.
set obs 8
generate byte worker = 1 + mod(floor((_n-1)/4),2)
generate byte firm = 1 + mod(floor((_n-1)/2),2)
generate double worker_sign = 2*worker-3
generate double firm_sign = 2*firm-3
generate double copy_sign = 2*mod(_n-1,2)-1
generate long match = 2*(worker-1)+firm
generate long worker_relabel = 3-worker
generate long firm_relabel = 3-firm
generate long match_relabel = 100-match
scalar anchor_d = .002
scalar anchor_a = sqrt(1-anchor_d^2)
scalar anchor_s = 1/sqrt(8)
generate double order_z1 = anchor_s*(anchor_a*worker_sign + ///
    anchor_d*copy_sign*firm_sign)
generate double order_z2 = anchor_s*(anchor_a*firm_sign - ///
    anchor_d*copy_sign*worker_sign)
generate double order_u1 = order_z1+order_z2
generate double order_u2 = -3*order_z1+order_z2
generate double y = sin(1.7*(_n-1))+.1*(_n-1)

foreach deletion_mode in observation match {
    local deletion_options "deletion(`deletion_mode')"
    local relabeled_options "deletion(`deletion_mode')"
    if "`deletion_mode'" == "match" {
        local deletion_options "`deletion_options' deletionid(match)"
        local relabeled_options ///
            "`relabeled_options' deletionid(match_relabel)"
    }
    foreach nuisance_mode in joint fixedoffset {
        foreach selected_algorithm in exact auto {
            capture noisily vckss y order_z1 order_z2, ///
                worker(worker) firm(firm) ///
                `deletion_options' algorithm(`selected_algorithm') ///
                nuisance(`nuisance_mode') nodisplay
            assert _rc == 498
            assert "`e(withholding_status)'" == ///
                "AMBIGUOUS_CONTROL_BASIS"

            capture noisily vckss y order_u1 order_u2, ///
                worker(worker) firm(firm) ///
                `deletion_options' algorithm(`selected_algorithm') ///
                nuisance(`nuisance_mode') nodisplay
            assert _rc == 498
            assert "`e(withholding_status)'" == ///
                "AMBIGUOUS_CONTROL_BASIS"

            capture noisily vckss y order_u1 order_u2, ///
                worker(worker_relabel) ///
                firm(firm_relabel) `relabeled_options' ///
                algorithm(`selected_algorithm') ///
                nuisance(`nuisance_mode') nodisplay
            assert _rc == 498
            assert "`e(withholding_status)'" == ///
                "AMBIGUOUS_CONTROL_BASIS"
        }
    }
}

// A better-conditioned member of the same family must remain accepted and
// agree under both the control map and simultaneous identifier relabeling.
scalar anchor_d = .2
scalar anchor_a = sqrt(1-anchor_d^2)
quietly replace order_z1 = anchor_s*(anchor_a*worker_sign + ///
    anchor_d*copy_sign*firm_sign)
quietly replace order_z2 = anchor_s*(anchor_a*firm_sign - ///
    anchor_d*copy_sign*worker_sign)
quietly replace order_u1 = order_z1+order_z2
quietly replace order_u2 = -3*order_z1+order_z2

foreach deletion_mode in observation match {
    local deletion_options "deletion(`deletion_mode')"
    local relabeled_options "deletion(`deletion_mode')"
    if "`deletion_mode'" == "match" {
        local deletion_options "`deletion_options' deletionid(match)"
        local relabeled_options ///
            "`relabeled_options' deletionid(match_relabel)"
    }
    foreach nuisance_mode in joint fixedoffset {
        foreach selected_algorithm in exact auto {
            vckss y order_z1 order_z2, worker(worker) firm(firm) ///
                `deletion_options' algorithm(`selected_algorithm') ///
                nuisance(`nuisance_mode') nodisplay
            assert "`e(algorithm)'" == "exact"
            matrix order_reference = e(results)

            vckss y order_u1 order_u2, worker(worker) firm(firm) ///
                `deletion_options' algorithm(`selected_algorithm') ///
                nuisance(`nuisance_mode') nodisplay
            assert "`e(algorithm)'" == "exact"
            assert mreldif(order_reference,e(results)) < 2e-10

            vckss y order_u1 order_u2, worker(worker_relabel) ///
                firm(firm_relabel) `relabeled_options' ///
                algorithm(`selected_algorithm') ///
                nuisance(`nuisance_mode') nodisplay
            assert "`e(algorithm)'" == "exact"
            assert mreldif(order_reference,e(results)) < 2e-10
        }
    }
}

clear

// Reviewer N's three safely eligible control rows.  API 12 sorted Q and -Q
// by their raw coordinates and chose different anchors even though T=-I has
// determinant one.  Distinct outcomes now supply one common semantic order.
set obs 9
generate byte worker = 1
generate byte firm = 1
replace firm = 2 in 4/5
replace worker = 2 in 6/9
replace firm = 2 in 8/9
generate double q1 = 0
generate double q2 = 0
replace q1 = sqrt(2/3) in 1
replace q1 = -1/sqrt(6) in 2/3
replace q2 = 1/sqrt(2) in 2
replace q2 = -1/sqrt(2) in 3
generate double minus_q1 = -q1
generate double minus_q2 = -q2
generate double y = _n + sin(_n)/10

preserve
sort y
mata:
Q = st_data(.,("q1","q2"))
minus_Q = st_data(.,("minus_q1","minus_q2"))
frequency = J(9,1,1)
base = vckss__canonical_controls(Q,frequency,1e-10)
changed = vckss__canonical_controls(minus_Q,frequency,1e-10)
assert(base.status == "CONVERGED")
assert(changed.status == "CONVERGED")
assert(mreldif(base.controls,changed.controls) < 2e-12)
end
restore

foreach nuisance_mode in joint fixedoffset {
    foreach selected_algorithm in exact auto {
        vckss y q1 q2, worker(worker) firm(firm) ///
            deletion(observation) algorithm(`selected_algorithm') ///
            nuisance(`nuisance_mode') nodisplay
        assert "`e(algorithm)'" == "exact"
        matrix safe_eligible_reference = e(results)
        vckss y minus_q1 minus_q2, worker(worker) firm(firm) ///
            deletion(observation) algorithm(`selected_algorithm') ///
            nuisance(`nuisance_mode') nodisplay
        assert "`e(algorithm)'" == "exact"
        assert mreldif(safe_eligible_reference,e(results)) < 2e-10
    }
}

clear

// Reviewer K's compact boundary witness. In native Stata API 11 accepted
// both bases at seed 2 and mreldif(e(results)) was 3.308e-5. The transformed
// basis has determinant one but its anchor decision is not certifiable at the
// registered numerical margin.
set obs 14
generate long worker = 2
replace worker = 1 in 1/4
replace worker = 1 in 9/11
generate byte firm = .
replace firm = 1 in 1/2
replace firm = 2 in 3/4
replace firm = 1 in 5/6
replace firm = 2 in 7/8
replace firm = 3 in 9
replace firm = 4 in 10
replace firm = 5 in 11
replace firm = 3 in 12
replace firm = 4 in 13
replace firm = 5 in 14
generate long frequency = 1
replace frequency = 8 in 9/10
replace frequency = 11 in 11
replace frequency = 14 in 12
replace frequency = 2 in 13
replace frequency = 3 in 14
generate double z1 = 0
generate double z2 = 0
local z1s .7037651560862372 -.2552775470643382 ///
    -.1128966060152190 -.26212880006292877 -.08287286955710015 ///
    -.06938638543245550 .08296261314678174 -.5826882952311562
local z2s -.01213100064390886 .6559466556462534 ///
    .28729193595054625 -.15584457130166096 -.1733117487431483 ///
    -.0772968683597004 -.5616086968926074 -.33368629034981906
forvalues row = 1/8 {
    local value : word `row' of `z1s'
    quietly replace z1 = `value' in `row'
    local value : word `row' of `z2s'
    quietly replace z2 = `value' in `row'
}
generate double u1 = 8000*z1
generate double u2 = z1 + .000125*z2
generate double y = 10*(_n-1) + sin(_n-1)

mata:
Q = st_data(.,("z1","z2"))
QT = st_data(.,("u1","u2"))
frequency = st_data(.,"frequency")
base = vckss__canonical_controls(Q,frequency,1e-10)
changed = vckss__canonical_controls(QT,frequency,1e-10)
assert(base.status == "CONVERGED")
assert(changed.status == "AMBIGUOUS_CONTROL_BASIS")
end

foreach nuisance_mode in joint fixedoffset {
    capture noisily vckss y z1 z2 [fw=frequency], worker(worker) ///
        firm(firm) deletion(observation) algorithm(exact) ///
        nuisance(`nuisance_mode') backend(mata) rng(stata) nodisplay
    assert _rc == 0
    capture noisily vckss y u1 u2 [fw=frequency], worker(worker) ///
        firm(firm) deletion(observation) algorithm(exact) ///
        nuisance(`nuisance_mode') backend(mata) rng(stata) nodisplay
    assert _rc == 498
    assert "`e(withholding_status)'" == "AMBIGUOUS_CONTROL_BASIS"

    foreach selected_algorithm in jla auto {
        local dispatch "algorithm(`selected_algorithm')"
        if "`selected_algorithm'" == "auto" {
            local dispatch "`dispatch' exact_limit(2)"
        }
        foreach batch_size in 1 2 {
            capture noisily vckss y z1 z2 [fw=frequency], ///
                worker(worker) firm(firm) deletion(observation) ///
                `dispatch' nuisance(`nuisance_mode') probes(2) ///
                batch(`batch_size') seed(2) tolerance(1e-4) ///
                backend(mata) rng(stata) nodisplay
            assert _rc == 0
            assert "`e(algorithm)'" == "jla"

            capture noisily vckss y u1 u2 [fw=frequency], ///
                worker(worker) firm(firm) deletion(observation) ///
                `dispatch' nuisance(`nuisance_mode') probes(2) ///
                batch(`batch_size') seed(2) tolerance(1e-4) ///
                backend(mata) rng(stata) nodisplay
            local failure_rc = _rc
            local failure_status "`e(withholding_status)'"
            assert `failure_rc' == 498
            assert "`failure_status'" == "AMBIGUOUS_CONTROL_BASIS"
        }
    }
}

// Counter-V1 has a different finite-probe path than Stata's RNG.  Exercise
// the same public anchor contract with enough probes that the valid basis is
// certified, while the transformed basis must still fail before estimation.
if `rust_available' {
    foreach nuisance_mode in joint fixedoffset {
        foreach selected_algorithm in jla auto {
            local dispatch "algorithm(`selected_algorithm')"
            if "`selected_algorithm'" == "auto" {
                local dispatch "`dispatch' exact_limit(2)"
            }
            local rust_batches "1 17"
            if "`selected_algorithm'" == "auto" local rust_batches "auto"
            foreach batch_size of local rust_batches {
                capture noisily vckss y z1 z2 [fw=frequency], ///
                    worker(worker) firm(firm) deletion(observation) ///
                    `dispatch' nuisance(`nuisance_mode') probes(200) ///
                    batch(`batch_size') seed(2) tolerance(1e-4) ///
                    backend(rust) rng(counter_v1) nodisplay
                assert _rc == 0
                assert "`e(algorithm)'" == "jla"

                capture noisily vckss y u1 u2 [fw=frequency], ///
                    worker(worker) firm(firm) deletion(observation) ///
                    `dispatch' nuisance(`nuisance_mode') probes(200) ///
                    batch(`batch_size') seed(2) tolerance(1e-4) ///
                    backend(rust) rng(counter_v1) nodisplay
                assert _rc == 498
                assert "`e(withholding_status)'" == "AMBIGUOUS_CONTROL_BASIS"
            }
        }
    }
}

di as result "PASS test_control_anchor.do"
