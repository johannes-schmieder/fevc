version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

varcomp_kss_rust clear
set obs 96
generate long cell = floor((_n-1)/2)
generate double worker = floor(cell/4)+1
generate double firm = mod(cell,4)+1
generate double replicate = mod(_n-1,2)
generate double deletion_id = cell+1
generate double outcome = .7*worker-.45*firm+.3*replicate+mod(7*(_n-1),5)/11
generate double frequency = mod(_n-1,3)+1
generate double target_weight = .5+mod(5*(_n-1),7)/3
generate double control = (worker-.4*firm)*(replicate+1)+mod(3*(_n-1),7)/17
sort cell replicate
set rng kiss32
set seed 20260822
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1)        ///
    targetweight(target_weight) nodisplay
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "generic"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(status)'"' == "KSS_POINT_ESTIMATES_ONLY"
assert `"`e(version)'"' == "0.3.0-dev"
assert strpos(`"`e(cmdline)'"',"algorithm(jla)") > 0
assert strpos(`"`e(cmdline)'"',"backend(rust)") > 0
assert e(rust_cap_schema) == 2
assert e(rust_cap_profile_code) == 3
assert e(rust_rng_contract_code) == 1
assert e(probes) == 7
assert e(leverage_batch) == 2 & e(target_batch) == 2
assert e(full_parameters) == e(worker_levels)+e(firm_levels)
assert e(parameters) == e(full_parameters)
assert e(correction_parameters) == e(full_parameters)
assert e(inverse_relres) >= . & e(information_rcond) >= .
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_working_fit_residual) <= e(residual_acceptance_tolerance)
assert inlist(e(rust_full_fit_zero_rhs),0,1)
assert e(rust_max_complete_residual) == e(complete_residual_max)
assert e(rust_maker_relres) == e(correction_reciprocal_residual)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
assert rowsof(e(rust_rhs_receipts)) == 1+1+3*7
assert colsof(e(rust_rhs_receipts)) == 15
assert e(rust_rhs_v2_copy_bytes) == rowsof(e(rust_rhs_receipts))*216
assert e(rust_rhs_receipt_schema) == 2
assert e(rust_control_rhs_count) == 1
assert e(rust_requested_engine_code) == 2
assert e(rust_selected_engine_code) == 2
assert e(rust_batch_mode_code) == 1
assert e(rust_stayers_mode_code) == 1
assert e(rust_target_weight_mode_code) == 1
assert e(rust_deletion_source_code) == 2
assert e(rust_probeorder_supplied) == 0
assert e(rust_wallseconds_supplied) == 0
assert e(rust_frequency_use_code) == 1
assert e(rust_solve_physical_limit) == e(physical_limit)
assert e(rust_result_cap_schema) == 2 & e(rust_result_cap_profile) == 3
assert e(rust_solve_signature_hi) == e(rust_cap_signature_hi)
assert e(rust_solve_signature_lo) == e(rust_cap_signature_lo)
assert colsof(e(rust_memory_receipt)) == 12
assert colsof(e(rust_request_capability_receipt)) == 23
assert colsof(e(rust_control_rank_receipt)) == 10
assert e(rust_support_flags) == 38
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(algorithm_option_supplied) == 1
assert e(engine_option_supplied) == 1
assert e(preconditioner_option_supplied) == 1
assert e(batch_option_supplied) == 1
assert e(stayers_option_supplied) == 0
assert e(targetweight_option_supplied) == 1
tempname initial_control_rank
matrix `initial_control_rank' = e(rust_control_rank_receipt)
assert `initial_control_rank'[1,1] ==                    ///
    `initial_control_rank'[1,2]/`initial_control_rank'[1,3]
assert `initial_control_rank'[1,1] > `initial_control_rank'[1,8]
assert `initial_control_rank'[1,4] >= 0
assert `initial_control_rank'[1,5] >= 0 & `initial_control_rank'[1,5] < .25
assert `initial_control_rank'[1,6] > 0
assert `initial_control_rank'[1,7] == e(rust_rhs_receipts)[1,7]
assert `initial_control_rank'[1,7] <= `initial_control_rank'[1,10]
assert `initial_control_rank'[1,8] == max(e(rust_rank_tolerance),1e-12)
assert `initial_control_rank'[1,9] == 1e-13
assert `initial_control_rank'[1,10] == 1e-11
foreach result_matrix in results plugin correction kss numerical_mcse decomposition {
    confirm matrix e(`result_matrix')
}
capture confirm matrix e(V)
assert _rc != 0
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

tempname public_reference repeat_reference private_reference
matrix `public_reference' = e(results)
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1)        ///
    targetweight(target_weight) nodisplay
matrix `repeat_reference' = e(results)
assert mreldif(`public_reference',`repeat_reference') == 0

// Match IDs only label deletion blocks: the implicit cell partition and a
// strictly increasing relabel preserve the same canonical block ordering.
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) nuisance(joint) algorithm(jla) backend(rust)        ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2)   ///
    probes(7) seed(81227) tolerance(1e-12) memory_gib(1)                ///
    targetweight(target_weight) nodisplay
assert mreldif(e(results),`public_reference') == 0
generate double deletion_relabel = 1000000+17*deletion_id
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_relabel) nuisance(joint)       ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic)       ///
    preconditioner(diagonal) batch(2) probes(7) seed(81227)            ///
    tolerance(1e-12) memory_gib(1) targetweight(target_weight) nodisplay
assert mreldif(e(results),`public_reference') == 0

// The private direct lifecycle is a same-atom Counter-V1 reference for the
// public boundary, not an independent estimator oracle.
preserve
quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1) controls(1) ///
    frequencyused(1) engine(generic) batchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid) physicallimit(50000000)
local private_schema = r(request_schema)
local private_profile = r(profile_code)
local private_signature_hi = r(request_signature_hi)
local private_signature_lo = r(request_signature_lo)
tempvar private_keep
quietly varcomp_kss_rust prepare worker firm deletion_id outcome frequency ///
    target_weight control, cleanup generate(`private_keep') memorygib(1) ///
    deletion(match)
local private_handle = r(handle)
quietly varcomp_kss_rust solve `private_handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) seed(81227) probes(7) leveragebatch(2) ///
    targetbatch(2) tolerance(1e-12) engine(generic) batchmode(explicit) ///
    stayers(movers) targetweightmode(explicit) deletionsource(matchid) ///
    physicallimit(50000000) capabilityschema(`private_schema')       ///
    capabilityprofile(`private_profile') frequencyused(1)           ///
    signaturehi(`private_signature_hi') signaturelo(`private_signature_lo')
quietly varcomp_kss_rust result `private_handle'
matrix `private_reference' = r(result)
assert mreldif(`public_reference',`private_reference') == 0
quietly varcomp_kss_rust release `private_handle'
restore
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0

// Omitted and auto backends remain Mata; no public option tuple implies Rust.
foreach mata_backend in omitted auto {
    local backend_option
    if "`mata_backend'" == "auto" local backend_option backend(auto)
    quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
        deletion(observation) nuisance(joint) algorithm(jla) engine(generic) ///
        preconditioner(diagonal) batch(2) probes(4) seed(81227)       ///
        targetweight(target_weight) `backend_option' nodisplay
    assert `"`e(backend_selected)'"' == "mata"
}

// Generic JLA is never inferred from a partial tuple.
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) engine(generic) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) batch(2) probes(4) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) preconditioner(diagonal) probes(4) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) algorithm(jla) engine(generic) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "RUST_COUNTER_RNG_REQUIRED"
foreach forbidden in "stayers(movers)" "probeorder(replicate)"      ///
    "wallseconds(10)" "preconditioner(cmg)" "batch(auto)" {
    local preconditioner_option preconditioner(diagonal)
    local batch_option batch(2)
    if "`forbidden'" == "preconditioner(cmg)" local preconditioner_option
    if "`forbidden'" == "batch(auto)" local batch_option
    capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
        deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
        engine(generic) `preconditioner_option' `batch_option' probes(4) ///
        `forbidden' nodisplay
    assert _rc == 498
    assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
    assert `"`e(backend_selected)'"' == ""
    assert `"`e(rng_selected)'"' == ""
}
foreach auto_option in "algorithm(auto)" "engine(auto)"           ///
    "engine(compressed)" "preconditioner(auto)" {
    local algorithm_option algorithm(jla)
    local engine_option engine(generic)
    local preconditioner_option preconditioner(diagonal)
    if "`auto_option'" == "algorithm(auto)" local algorithm_option
    if inlist("`auto_option'","engine(auto)","engine(compressed)") ///
        local engine_option
    if "`auto_option'" == "preconditioner(auto)" local preconditioner_option
    capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
        deletion(observation) backend(rust) rng(counter_v1)        ///
        `algorithm_option' `engine_option' `preconditioner_option' ///
        batch(2) probes(4) `auto_option' nodisplay
    assert _rc == 498
    assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
    assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
}

// Registered deletion/nuisance/control/weight combinations are all public.
foreach deletion in match observation {
    local deletion_option
    if "`deletion'" == "match" local deletion_option deletionid(deletion_id)
    foreach nuisance in joint fixedoffset {
        quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
            deletion(`deletion') `deletion_option' nuisance(`nuisance') ///
            algorithm(jla) backend(rust) rng(counter_v1) engine(generic) ///
            preconditioner(diagonal) batch(3) probes(5) seed(81227) ///
            targetweight(target_weight) tolerance(1e-12) memory_gib(1) nodisplay
        assert `"`e(deletion)'"' == "`deletion'"
        assert `"`e(nuisance)'"' == "`nuisance'"
        assert e(rust_leverage_probes_accepted) == 5
        assert e(rust_target_probes_accepted) == 5
        assert e(target_weight_sum) == 144
        if "`deletion'" == "observation" assert e(deletion_units) == e(N_physical)
        else assert e(deletion_units) == 48
        if "`nuisance'" == "joint" assert e(parameters) == e(full_parameters)
        else assert e(parameters) == e(worker_levels)+e(firm_levels)-1
        local distinct_working = ("`nuisance'"=="fixedoffset")
        assert rowsof(e(rust_rhs_receipts)) == 1+1+`distinct_working'+3*5
    }
}

// Q=0 and factor controls use the actual materialized control count.
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(fixedoffset) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 0
assert rowsof(e(rust_rhs_receipts)) == 1+3*4
tempname q0_control_rank
matrix `q0_control_rank' = e(rust_control_rank_receipt)
assert `q0_control_rank'[1,1] == 1
assert `q0_control_rank'[1,2] == 1
assert `q0_control_rank'[1,3] == 1
forvalues column = 4/7 {
    assert `q0_control_rank'[1,`column'] == 0
}
assert `q0_control_rank'[1,8] == max(e(rust_rank_tolerance),1e-12)
assert `q0_control_rank'[1,9] == 1e-13
assert `q0_control_rank'[1,10] == 1e-11

// Multiple deletion IDs may occupy one coefficient cell.
generate long deletion_subcell = 2*cell+replicate+1
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_subcell) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(coefficient_cells) == 48
assert e(deletion_units) == 96
assert e(deletion_units) > e(coefficient_cells)

// Factor dummies and interactions materialize in the same semantic order as
// their explicit numeric columns and therefore use identical Counter atoms.
generate byte category = mod(7*_n,5)
forvalues level = 1/4 {
    generate byte category_`level' = category == `level'
}
generate double control_rep0 = control*(replicate==0)
generate double control_rep1 = control*(replicate==1)
tempname factor_results factor_rhs factor_interaction_results
tempname factor_interaction_rhs
quietly varcomp_kss outcome ib0.category [fw=frequency],            ///
    worker(worker) firm(firm)                                      ///
    deletion(observation) nuisance(joint) algorithm(jla)           ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 4
assert e(rust_cap_supported) == 1
matrix `factor_results' = e(results)
matrix `factor_rhs' = e(rust_rhs_receipts)
quietly varcomp_kss outcome category_1 category_2 category_3 category_4 ///
    [fw=frequency], worker(worker) firm(firm) deletion(observation) ///
    nuisance(joint) algorithm(jla) backend(rust)                    ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 4
assert mreldif(`factor_results',e(results)) == 0
assert mreldif(`factor_rhs',e(rust_rhs_receipts)) == 0
quietly varcomp_kss outcome c.control#ib0.replicate [fw=frequency], ///
    worker(worker) firm(firm) deletion(observation) nuisance(joint) ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic)   ///
    preconditioner(diagonal) batch(2) probes(4) seed(81227)        ///
    memory_gib(1) nodisplay
assert e(controls_count) == 2
matrix `factor_interaction_results' = e(results)
matrix `factor_interaction_rhs' = e(rust_rhs_receipts)
quietly varcomp_kss outcome control_rep0 control_rep1 [fw=frequency], worker(worker) ///
    firm(firm) deletion(observation) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 2
assert mreldif(`factor_interaction_results',e(results)) == 0
assert mreldif(`factor_interaction_rhs',e(rust_rhs_receipts)) == 0

// The advertised Q=32 boundary is accepted; Q=33 is declined before prepare.
preserve
clear
set obs 544
generate long row0 = _n-1
generate long repetition_q = mod(row0,34)
generate long firm_q = mod(floor(row0/34),4)+1
generate long worker_q = floor(row0/(4*34))+1
generate double outcome_q = .4*(worker_q-1)-.3*(firm_q-1)+      ///
    repetition_q/17+mod(row0,9)/23
generate double target_q = .75+(row0+1)/1000
local controls32
forvalues q = 1/33 {
    generate double q`q' = cos(c(pi)*(repetition_q+.5)*`q'/34)
    if `q' <= 32 local controls32 `controls32' q`q'
}
capture quietly varcomp_kss outcome_q `controls32', worker(worker_q) firm(firm_q) ///
    deletion(observation) nuisance(joint) algorithm(jla)            ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(3) seed(81227) tolerance(1e-11) memory_gib(1) ///
    targetweight(target_q) nodisplay
local q32_rc = _rc
if !`q32_rc' {
    assert e(controls_count) == 32
    assert e(full_parameters) == e(worker_levels)+e(firm_levels)-1+32
    assert e(parameters) == e(full_parameters)
}
capture quietly varcomp_kss outcome_q `controls32' q33, worker(worker_q) firm(firm_q) ///
    deletion(observation) nuisance(joint) algorithm(jla)            ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(3) seed(81227) memory_gib(1) nodisplay
local q33_rc = _rc
assert `q33_rc' == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
assert e(rust_cap_reason_code) == 9
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
restore

// if/in and observation deletion exclude incomplete rows while preserving
// the original requested-sample e(sample) map.
generate byte eligible = _n <= 80
generate double outcome_missing = outcome
replace outcome_missing = . in 10
quietly varcomp_kss outcome_missing control if eligible in 1/70 [fw=frequency], ///
    worker(worker) firm(firm) deletion(observation) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(N_requested) == 70 & e(N_complete) == 69
assert !e(sample) in 10
quietly count if e(sample)
assert r(N) == e(N_retained)
capture quietly varcomp_kss outcome_missing control if eligible in 1/70, ///
    worker(worker) firm(firm) deletion(match) deletionid(deletion_id) ///
    nuisance(joint) algorithm(jla) backend(rust) rng(counter_v1) engine(generic) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert _rc == 459 & `"`e(withholding_status)'"' == "MATCH_INPUT_MISSING"

// Canonical row order makes storage permutation irrelevant.
generate long original_order = _n
gsort -cell -replicate
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
assert mreldif(e(results),`public_reference') == 0
sort original_order

// Registered public resource and rank gates fail closed and return idle.
local failure_rng `"`c(rng)'"'
local failure_stream = c(rngstream)
local failure_state `"`c(rngstate)'"'
local failure_sortedby : sortedby
quietly _datasignature
local failure_signature `"`r(datasignature)'"'
capture quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) physical_limit(1) memory_gib(1) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "PHYSICAL_COPY_LIMIT"
assert `"`e(native_error_phase)'"' == "physical_limit"
assert e(native_error_code) >= .
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
assert `"`c(rng)'"' == `"`failure_rng'"'
assert c(rngstream) == `failure_stream'
assert `"`c(rngstate)'"' == `"`failure_state'"'
local failure_sortedby_after : sortedby
assert `"`failure_sortedby_after'"' == `"`failure_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`failure_signature'"'
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
capture quietly varcomp_kss outcome, worker(worker) firm(firm)        ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) blocksize_limit(1) nodisplay
assert _rc == 198 & `"`e(withholding_status)'"' == "BLOCK_SIZE_LIMIT"
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(.000001) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',"RESOURCE_LIMIT",       ///
    "ALLOCATION_FAILED")
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`failure_rng'"'
assert c(rngstream) == `failure_stream'
assert `"`c(rngstate)'"' == `"`failure_state'"'
local allocation_sortedby_after : sortedby
assert `"`allocation_sortedby_after'"' == `"`failure_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`failure_signature'"'

// The fully explicit generic route never silently accepts an unconverged fit.
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) maxiter(1) memory_gib(1) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "PCG_MAXITER"
assert `"`e(native_error_phase)'"' == "solve"
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`failure_state'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`failure_signature'"'

generate double duplicate_control = control
local rank_rng_state `"`c(rngstate)'"'
local rank_sortedby : sortedby
quietly _datasignature
local rank_signature `"`r(datasignature)'"'
capture quietly varcomp_kss outcome control duplicate_control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(1) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',"SINGULAR_INFORMATION", ///
    "AMBIGUOUS_CONTROL_BASIS")
assert e(native_error_code) < .
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`rank_rng_state'"'
local rank_sortedby_after : sortedby
assert `"`rank_sortedby_after'"' == `"`rank_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`rank_signature'"'
generate byte spike_control = _n == 1
capture quietly varcomp_kss outcome spike_control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(1) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "UNVERIFIED_DELETION_RANK"
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0

// Missing/corrupt capability receipts fail before prepare; a corrupted V6
// echo fails after result export and still releases to idle.
capture program drop _vckss_rust_public_call
program define _vckss_rust_public_call, rclass
    version 18.0
    gettoken subcommand rest : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "prepare" {
        global VCKSS_GENERIC_PREPARE_CALLED = $VCKSS_GENERIC_PREPARE_CALLED+1
    }
    if "${VCKSS_GENERIC_FAULT}" == "missing_capability" &          ///
        "`subcommand'" == "requestcapability" exit 199
    if "${VCKSS_GENERIC_FAULT}" == "userbreak_solve" &             ///
        "`subcommand'" == "solve" exit 1
    quietly varcomp_kss_rust `0'
    local signature_lo = cond("`subcommand'"=="requestcapability", ///
        r(request_signature_lo),.)
    local result_signature_lo = cond("`subcommand'"=="result",     ///
        r(request_signature_lo),.)
    tempname returned_rhs
    if "`subcommand'" == "result" matrix `returned_rhs' = r(rhs_receipts)
    return add
    if "${VCKSS_GENERIC_FAULT}" == "corrupt_capability" &          ///
        "`subcommand'" == "requestcapability" {
        return scalar request_signature_lo = mod(`signature_lo'+1,4294967296)
    }
    if "${VCKSS_GENERIC_FAULT}" == "corrupt_result" &              ///
        "`subcommand'" == "result" {
        return scalar request_signature_lo = mod(`result_signature_lo'+1,4294967296)
    }
    if "`subcommand'" == "result" {
        local corrupt_field
        local corrupt_value .
        if "${VCKSS_GENERIC_FAULT}" == "cr_rcond" {
            local corrupt_field control_rank_rcond
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_small" {
            local corrupt_field control_rank_smallest_lower
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_large" {
            local corrupt_field control_rank_largest_upper
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_projection" {
            local corrupt_field control_rank_projection_error
            local corrupt_value -1
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_normalization" {
            local corrupt_field control_rank_normalization_err
            local corrupt_value .25
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_fe_bound" {
            local corrupt_field control_rank_fe_info_lower
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_max_projection" {
            local corrupt_field control_rank_max_projection
            local corrupt_value -1
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_effective_tol" {
            local corrupt_field control_rank_effective_tolerance
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_pcg_tol" {
            local corrupt_field control_rank_projection_pcg_tol
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_residual_gate" {
            local corrupt_field control_rank_projection_gate
            local corrupt_value 0
        }
        if "`corrupt_field'" != "" {
            return scalar `corrupt_field' = `corrupt_value'
        }
        if "${VCKSS_GENERIC_FAULT}" == "rhs_control_complete" {
            matrix `returned_rhs'[1,7] = `returned_rhs'[1,7]+1e-15
            return matrix rhs_receipts = `returned_rhs'
        }
        if "${VCKSS_GENERIC_FAULT}" == "rhs_full_reduced" {
            matrix `returned_rhs'[2,6] = `returned_rhs'[2,6]+1e-15
            return matrix rhs_receipts = `returned_rhs'
        }
    }
end
local reconcile_rng `"`c(rng)'"'
local reconcile_stream = c(rngstream)
local reconcile_state `"`c(rngstate)'"'
local reconcile_sortedby : sortedby
quietly _datasignature
local reconcile_signature `"`r(datasignature)'"'
foreach fault in missing_capability corrupt_capability {
    global VCKSS_GENERIC_FAULT `fault'
    global VCKSS_GENERIC_PREPARE_CALLED 0
    capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
        rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
        probes(4) memory_gib(1) nodisplay
    assert _rc == 498
    assert "$VCKSS_GENERIC_PREPARE_CALLED" == "0"
    assert `"`e(native_error_phase)'"' == cond("`fault'"=="missing_capability", ///
        "request_capability","request_capability_reconcile")
    assert `"`e(backend_requested)'"' == "rust"
    assert `"`e(algorithm)'"' == "jla"
    assert `"`e(engine_requested)'"' == "generic"
    assert `"`e(preconditioner_requested)'"' == "diagonal"
    assert `"`e(rng_requested)'"' == "counter_v1"
    assert e(backend_option_supplied) == 1
    assert e(algorithm_option_supplied) == 1
    assert e(engine_option_supplied) == 1
    assert e(preconditioner_option_supplied) == 1
    assert e(batch_option_supplied) == 1
    assert e(rng_option_supplied) == 1
    assert e(native_error_code) >= .
    assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
    quietly varcomp_kss_rust snapshot
    assert r(state) == 0 & r(handle) == 0
    assert `"`c(rng)'"' == `"`reconcile_rng'"'
    assert c(rngstream) == `reconcile_stream'
    assert `"`c(rngstate)'"' == `"`reconcile_state'"'
    local reconcile_sortedby_after : sortedby
    assert `"`reconcile_sortedby_after'"' == `"`reconcile_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`reconcile_signature'"'
}
foreach fault in corrupt_result cr_rcond cr_small cr_large cr_projection ///
    cr_normalization cr_fe_bound cr_max_projection cr_effective_tol     ///
    cr_pcg_tol cr_residual_gate rhs_control_complete rhs_full_reduced {
    global VCKSS_GENERIC_FAULT `fault'
    global VCKSS_GENERIC_PREPARE_CALLED 0
    capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
        rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
        probes(4) memory_gib(1) nodisplay
    assert _rc == 498
    assert `"`e(native_error_phase)'"' == "result_reconcile"
    assert "$VCKSS_GENERIC_PREPARE_CALLED" == "1"
    assert e(native_error_code) >= .
    assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
    quietly varcomp_kss_rust snapshot
    assert r(state) == 0 & r(handle) == 0
    assert `"`c(rng)'"' == `"`reconcile_rng'"'
    assert c(rngstream) == `reconcile_stream'
    assert `"`c(rngstate)'"' == `"`reconcile_state'"'
    local reconcile_sortedby_after : sortedby
    assert `"`reconcile_sortedby_after'"' == `"`reconcile_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`reconcile_signature'"'
}

// UserBreak is untyped, clears e(), releases the prepared handle, and restores
// the complete caller state through the outer finally guard.
global VCKSS_GENERIC_FAULT userbreak_solve
global VCKSS_GENERIC_PREPARE_CALLED 0
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(1) nodisplay
assert _rc == 1
assert `"`e(cmd)'"' == ""
assert "$VCKSS_GENERIC_PREPARE_CALLED" == "1"
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`reconcile_rng'"'
assert c(rngstream) == `reconcile_stream'
assert `"`c(rngstate)'"' == `"`reconcile_state'"'
local reconcile_sortedby_after : sortedby
assert `"`reconcile_sortedby_after'"' == `"`reconcile_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`reconcile_signature'"'
global VCKSS_GENERIC_FAULT
global VCKSS_GENERIC_PREPARE_CALLED
capture program drop _vckss_rust_public_call

assert `q32_rc' == 0

di as result "VARCOMP_KSS RUST PUBLIC GENERIC PASS"
exit 0
