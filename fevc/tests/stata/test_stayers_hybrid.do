version 18.0
clear
set more off
set varabbrev off
set seed 20260822

args package_dir
if `"`package_dir'"' != "" adopath ++ `"`package_dir'"'

set obs 33
generate long original_order = _n
generate long worker = .
generate byte firm = .
generate long match_id = .
generate double c1 = .
generate double c2 = .
generate double y = .
generate long frequency = 1
generate double target_mass = 1 + mod(_n,5)/7

local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
forvalues row=1/24 {
    quietly replace worker = floor((`row'-1)/4) in `row'
    local one_firm : word `row' of `firms'
    quietly replace firm = `one_firm' in `row'
    local one_match : word `row' of `matches'
    quietly replace match_id = `one_match' in `row'
    quietly replace c1 = mod(`row'-1,4)-1.5 in `row'
    quietly replace c2 = mod(`row',3)==0 in `row'
}
// Two eligible stayers on retained mover firms.
quietly replace worker = 100 in 25/26
quietly replace firm = 0 in 25/26
quietly replace match_id = 1000+_n in 25/26
quietly replace c1 = -.4+.8*(_n-25) in 25/26
quietly replace c2 = _n==26 in 25/26
quietly replace frequency = 2 in 25
quietly replace worker = 101 in 27/28
quietly replace firm = 2 in 27/28
quietly replace match_id = 1000+_n in 27/28
quietly replace c1 = -.3+.6*(_n-27) in 27/28
quietly replace c2 = _n==28 in 27/28
// One physical-copy singleton on a retained firm: deliberately ineligible.
quietly replace worker = 102 in 29
quietly replace firm = 1 in 29
quietly replace match_id = 1029 in 29
quietly replace c1 = .2 in 29
quietly replace c2 = 0 in 29
// One stayer and one mover in a smaller, unretained firm component.
quietly replace worker = 103 in 30/31
quietly replace firm = 9 in 30/31
quietly replace match_id = 1000+_n in 30/31
quietly replace c1 = -.1+.2*(_n-30) in 30/31
quietly replace c2 = _n==31 in 30/31
quietly replace worker = 200 in 32/33
quietly replace firm = 8 in 32
quietly replace firm = 9 in 33
quietly replace match_id = 1000+_n in 32/33
quietly replace c1 = -.2+.4*(_n-32) in 32/33
quietly replace c2 = _n==33 in 32/33
// Exercise literal-copy match deletion as well as stayer-copy deletion.
quietly replace frequency = 2 in 1
quietly replace y = 1.2 + .07*worker - .11*firm + .35*c1 - .2*c2 + sin(_n)/20

capture quietly fevc_rust probe
local rust_available = (_rc==0)
if `rust_available' capture quietly fevc_rust clear
local caller_rng `"`c(rngstate)'"'
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_id) algorithm(exact)    ///
    nuisance(joint) targetweight(target_mass) stayers(movers) ///
    backend(mata) rng(stata) nodisplay
assert original_order == _n
assert `"`c(rngstate)'"' == `"`caller_rng'"'
matrix mover_results = e(results)
matrix mover_b = e(b)
matrix mover_plugin = e(plugin)
matrix mover_correction = e(correction)
matrix mover_decomp = e(decomposition)
matrix mover_graph = (e(N_retained),e(N_physical),e(N_mover_input), ///
    e(N_initial_component),e(N_graph_dropped),e(graph_edges),       ///
    e(graph_articulation_workers),e(graph_leaveout_components),     ///
    e(graph_retained_edges),e(graph_bridge_units_removed),          ///
    e(graph_bridge_rows_removed),e(graph_final_bridge_units))
generate byte mover_sample = e(sample)

fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_id) algorithm(exact)    ///
    nuisance(joint) targetweight(target_mass) stayers(both)   ///
    backend(mata) rng(stata) nodisplay
assert original_order == _n
assert `"`c(rngstate)'"' == `"`caller_rng'"'
matrix both_graph = (e(N_retained),e(N_physical),e(N_mover_input), ///
    e(N_initial_component),e(N_graph_dropped),e(graph_edges),      ///
    e(graph_articulation_workers),e(graph_leaveout_components),    ///
    e(graph_retained_edges),e(graph_bridge_units_removed),         ///
    e(graph_bridge_rows_removed),e(graph_final_bridge_units))
assert mreldif(mover_results,e(mover_results)) < 2e-9
assert mreldif(mover_plugin,e(mover_plugin)) < 2e-9
assert mreldif(mover_correction,e(mover_correction)) < 2e-9
assert mreldif(e(results),e(stayer_hybrid_results)) < 2e-12
assert mreldif(e(b),e(stayer_hybrid_kss)) < 2e-12
generate byte both_sample = e(sample)
assert both_sample == (_n<=28)
assert mover_sample <= both_sample
assert "`e(target_population)'" == "retained movers plus eligible attached stayers"
assert "`e(stayer_hybrid_status)'" == "CONVERGED"
assert strpos("`e(stayer_hybrid_assumption)'","not match-robust") > 0
assert strpos("`e(stayer_hybrid_esample)'","eligible attached stayers") > 0
assert e(stayer_hybrid_N_stayers) == 2
assert e(stayer_hybrid_N_stayer_rows) == 4
assert e(stayer_hybrid_N_stayer_physical) == 5
assert e(stayer_hybrid_N_singleton_drop) == 1
assert e(stayer_hybrid_N_unattached) == 1
assert e(stayer_hybrid_N_stored) == 28
assert e(stayer_hybrid_N_physical) == 30
assert e(stayer_hybrid_worker_levels) == 8
assert e(stayer_hybrid_firm_levels) == 4
assert e(stayer_hybrid_deletion_units) == e(deletion_units)
quietly summarize target_mass in 1/24, meanonly
assert abs(e(stayer_hybrid_mover_target_mass)-r(sum)) < 1e-12
quietly summarize target_mass in 25/28, meanonly
assert abs(e(stayer_hybrid_stayer_target_mass)-r(sum)) < 1e-12
quietly summarize target_mass in 1/28, meanonly
assert abs(e(stayer_hybrid_target_mass)-r(sum)) < 1e-12

matrix mata_hybrid_joint = e(stayer_hybrid_results)
matrix mata_hybrid_source = e(stayer_hybrid_correction_source)
matrix mata_hybrid_account = e(stayer_hybrid_sample_accounting)

// Match deletion defaults to the current MATLAB population convention.
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm)            ///
    deletion(match) deletionid(match_id) algorithm(exact)         ///
    nuisance(joint) targetweight(target_mass) backend(mata)       ///
    rng(stata) nodisplay
assert "`e(stayers)'" == "both"
assert mreldif(mata_hybrid_joint,e(results)) < 2e-9
assert e(sample) == both_sample

// A default project() request uses the same mover-plus-eligible-stayer
// population and the mixed match/physical-observation deletion partition.
generate double projection_y = y - 50
generate double projection_z = sin(_n/3)
fevc projection_y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletionid(match_id) algorithm(exact) nuisance(joint)        ///
    targetweight(target_mass) project(projection_z)              ///
    projecteffect(firm) nodisplay
assert "`e(deletion)'" == "match"
assert "`e(stayers)'" == "both"
assert "`e(inference_deletion)'" == "exact match deletion; stayers both"
assert e(stayer_hybrid_N_stayers) == 2
assert e(stayer_hybrid_N_stayer_physical) == 5
assert rowsof(e(projection_V)) == 2 & colsof(e(projection_V)) == 2
assert e(sample) == both_sample
matrix mata_projection_b = e(projection_b)
matrix mata_projection_V = e(projection_V)
matrix mata_projection_results = e(projection_results)
matrix exact_projection_targets = (mata_projection_V[1,1], ///
    mata_projection_V[1,2], mata_projection_V[2,2],       ///
    mata_projection_results[1,2], mata_projection_results[1,5], ///
    mata_projection_results[1,6], mata_projection_results[2,2], ///
    mata_projection_results[2,5], mata_projection_results[2,6])
generate byte mata_projection_sample = e(sample)

// The explicit Rust route must reproduce the established Mata public
// contract with the combined target and e(sample) as the primary result.
if `rust_available' {
    // The qualified sparse route uses the same mixed block estimand.  Check
    // complete covariance matrices under several independent Counter-V1
    // streams rather than demanding pathwise equality to exact deletion.
    foreach projection_probes in 600 1200 {
        matrix jla_projection_draws = J(8,9,.)
        local projection_success = 0
        local projection_psd_failure = 0
        forvalues projection_seed = 20260901/20260908 {
            capture quietly fevc projection_y c1 c2 [fw=frequency], ///
                worker(worker) firm(firm)                            ///
                deletionid(match_id) algorithm(jla) engine(generic)          ///
                backend(rust) rng(counter_v1) seed(`projection_seed')        ///
                probes(`projection_probes') batch(8) preconditioner(diagonal) ///
                tolerance(1e-12) nuisance(joint) targetweight(target_mass)   ///
                project(projection_z) projecteffect(firm) nodisplay
            if _rc {
                assert _rc == 498
                assert "`e(withholding_status)'" == "JLA_CONSTRAINT_FAILED"
                local ++projection_psd_failure
                continue
            }
            local ++projection_success
            assert "`e(deletion)'" == "match"
            assert "`e(stayers)'" == "both"
            assert "`e(inference_method)'" == "sparse JLA block cross-fit projection"
            assert e(sample) == mata_projection_sample
            assert mreldif(mata_projection_b,e(projection_b)) < 2e-8
            assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)
            matrix current_projection_V = e(projection_V)
            matrix current_projection_results = e(projection_results)
            matrix jla_projection_draws[`projection_success',1] = ///
                current_projection_V[1,1]
            matrix jla_projection_draws[`projection_success',2] = ///
                current_projection_V[1,2]
            matrix jla_projection_draws[`projection_success',3] = ///
                current_projection_V[2,2]
            matrix jla_projection_draws[`projection_success',4] = ///
                current_projection_results[1,2]
            matrix jla_projection_draws[`projection_success',5] = ///
                current_projection_results[1,5]
            matrix jla_projection_draws[`projection_success',6] = ///
                current_projection_results[1,6]
            matrix jla_projection_draws[`projection_success',7] = ///
                current_projection_results[2,2]
            matrix jla_projection_draws[`projection_success',8] = ///
                current_projection_results[2,5]
            matrix jla_projection_draws[`projection_success',9] = ///
                current_projection_results[2,6]
        }
        assert `projection_success' >= 6
        assert `projection_psd_failure' == 8-`projection_success'
        noisily display as text "projection JLA probes `projection_probes': " ///
            `projection_success' "/8 accepted; " `projection_psd_failure' ///
            " typed PSD rejections"
        forvalues projection_column = 1/9 {
            scalar projection_exact = exact_projection_targets[1,`projection_column']
            preserve
                clear
                quietly set obs 8
                svmat double jla_projection_draws
                quietly summarize jla_projection_draws`projection_column'
                scalar projection_average = r(mean)
                scalar projection_mcse = r(sd)/sqrt(r(N))
            restore
            scalar projection_scale = max(1,abs(projection_exact), ///
                abs(projection_average))
            assert abs(projection_average-projection_exact) <= ///
                1e-8*projection_scale+6*projection_mcse
        }
    }

    fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
        deletion(match) deletionid(match_id) algorithm(exact)    ///
        nuisance(joint) targetweight(target_mass) stayers(both)   ///
        backend(rust) rng(counter_v1) nodisplay
    assert "`e(backend_selected)'" == "rust"
    assert "`e(result_family)'" == "exact"
    assert "`e(rng_selected)'" == "NOT_APPLICABLE"
    assert e(probes) == 0 & e(seed) == 0 & e(batch) == 0
    assert e(rust_rng_contract_code) == 0
    assert e(rust_counter_plan_complete) == 1
    assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
    assert mreldif(mover_results,e(mover_results)) < 2e-9
    assert mreldif(mover_plugin,e(mover_plugin)) < 2e-9
    assert mreldif(mover_correction,e(mover_correction)) < 2e-9
    assert mreldif(mata_hybrid_joint,e(results)) < 2e-9
    matrix rust_both_graph = (e(N_retained),e(N_physical),e(N_mover_input), ///
        e(N_initial_component),e(N_graph_dropped),e(graph_edges),          ///
        e(graph_articulation_workers),e(graph_leaveout_components),       ///
        e(graph_retained_edges),e(graph_bridge_units_removed),             ///
        e(graph_bridge_rows_removed),e(graph_final_bridge_units))
    generate byte rust_both_sample = e(sample)
    assert rust_both_sample == both_sample
    assert mreldif(mata_hybrid_joint,e(stayer_hybrid_results)) < 2e-9
    assert mreldif(mata_hybrid_source,e(stayer_hybrid_correction_source)) < 2e-9
    assert mreldif(mata_hybrid_account,e(stayer_hybrid_sample_accounting)) < 2e-12
    assert original_order == _n
    assert `"`c(rngstate)'"' == `"`caller_rng'"'
}
else {
    // The projection comparison above changes e(); restore the saved Mata
    // point-estimation design before the literal-refit oracle when no local
    // developer plugin is available to exercise the Rust branch.
    quietly fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
        deletion(match) deletionid(match_id) algorithm(exact)    ///
        nuisance(joint) targetweight(target_mass) stayers(both)   ///
        backend(mata) rng(stata) nodisplay
}

matrix hybrid_joint = e(stayer_hybrid_results)
matrix hybrid_source = e(stayer_hybrid_correction_source)
matrix hybrid_sum = hybrid_source[1,1..4]+hybrid_source[2,1..4]
assert mreldif(hybrid_sum,e(stayer_hybrid_correction)) < 1e-14
forvalues source=1/2 {
    assert abs(hybrid_source[`source',4] -                     ///
        (hybrid_source[`source',1]+hybrid_source[`source',2]+ ///
        2*hybrid_source[`source',3])) < 1e-12
}
forvalues result=1/3 {
    assert abs(hybrid_joint[`result',4] -                    ///
        (hybrid_joint[`result',1]+hybrid_joint[`result',2]+ ///
        2*hybrid_joint[`result',3])) < 1e-12
}
matrix hybrid_account = e(stayer_hybrid_sample_accounting)
assert hybrid_account[1,1] + hybrid_account[2,1] == hybrid_account[3,1]
assert hybrid_account[1,2] + hybrid_account[2,2] == hybrid_account[3,2]
assert hybrid_account[1,4] + hybrid_account[2,4] == hybrid_account[3,4]
assert hybrid_account[1,5] + hybrid_account[2,5] == hybrid_account[3,5]
capture matrix list e(V)
assert _rc != 0

// Independent literal-refit oracle.  Each mover block is removed in full;
// each eligible stayer stored row is refit once per physical copy.
generate byte hybrid_stayer = inlist(worker,100,101)
generate byte hybrid_use = both_sample | hybrid_stayer
egen long hybrid_worker = group(worker) if hybrid_use
egen long hybrid_firm = group(firm) if hybrid_use
capture mata: mata drop vckss_test__hybrid_refit()
mata:
real matrix vckss_test__hybrid_refit(
    string scalar sample_name,
    string scalar stayer_name,
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar control_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar nuisance)
{
    real colvector y, worker, firm, frequency, target, deletion, stayer
    real colvector working_y, full_beta, beta, mover, row_order, sorted
    real colvector index, beta_delete, residual_delete
    real matrix controls, full_design, design, information, inverse, rhs
    real matrix panel, block_design, block_information, block_rhs
    real matrix left, right, Q_worker, Q_firm, Q_covariance
    real matrix cell_share, cross
    real colvector worker_share, firm_share
    real rowvector plugin, correction, mover_correction, stayer_correction
    real scalar W, F, pfull, p, k, g, begin, finish, row, one, total

    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    controls = st_data(.,tokens(control_names),sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target = st_data(.,target_name,sample_name)
    deletion = st_data(.,deletion_name,sample_name)
    stayer = st_data(.,stayer_name,sample_name)
    W = max(worker)
    F = max(firm)
    k = cols(controls)
    pfull = W+F-1+k
    full_design = J(rows(y),pfull,0)
    for (row=1; row<=rows(y); row++) {
        full_design[row,worker[row]] = 1
        if (firm[row] < F) full_design[row,W+firm[row]] = 1
    }
    if (k > 0) full_design[.,(W+F)..pfull] = controls
    full_beta = invsym(full_design'*(frequency:*full_design)) *
        (full_design'*(frequency:*y))
    if (nuisance == "fixedoffset" & k > 0) {
        working_y = y-controls*full_beta[(pfull-k+1)..pfull]
        p = W+F-1
        design = full_design[.,1..p]
    }
    else {
        working_y = y
        design = full_design
        p = pfull
    }
    information = design'*(frequency:*design)
    inverse = invsym(information)
    rhs = design'*(frequency:*working_y)
    beta = inverse*rhs

    // Independent pooled target construction in the same identified
    // last-firm-grounded quotient coordinate used by the refit oracle.
    total = sum(target)
    worker_share = J(W,1,0)
    firm_share = J(F-1,1,0)
    cell_share = J(W,F-1,0)
    for (row=1; row<=rows(y); row++) {
        worker_share[worker[row]] = worker_share[worker[row]]+
            target[row]/total
        if (firm[row] < F) {
            firm_share[firm[row]] = firm_share[firm[row]]+
                target[row]/total
            cell_share[worker[row],firm[row]] =
                cell_share[worker[row],firm[row]]+target[row]/total
        }
    }
    Q_worker = J(p,p,0)
    Q_firm = J(p,p,0)
    Q_covariance = J(p,p,0)
    Q_worker[|1,1\W,W|] =
        diag(worker_share)-worker_share*worker_share'
    Q_firm[|W+1,W+1\W+F-1,W+F-1|] =
        diag(firm_share)-firm_share*firm_share'
    cross = cell_share-worker_share*firm_share'
    Q_covariance[|1,W+1\W,W+F-1|] = .5*cross
    Q_covariance[|W+1,1\W+F-1,W|] = .5*cross'
    plugin = ((beta'*Q_worker*beta)[1,1],
        (beta'*Q_firm*beta)[1,1],
        (beta'*Q_covariance*beta)[1,1],0)
    plugin[4] = plugin[1]+plugin[2]+2*plugin[3]
    mover_correction = J(1,4,0)
    stayer_correction = J(1,4,0)

    mover = selectindex(stayer:==0)
    row_order = mover[order(deletion[mover],1)]
    sorted = deletion[row_order]
    panel = panelsetup(sorted,1)
    for (g=1; g<=rows(panel); g++) {
        begin = panel[g,1]
        finish = panel[g,2]
        index = row_order[|begin\finish|]
        block_design = design[index,.]
        block_information = block_design'*(frequency[index]:*block_design)
        block_rhs = block_design'*(frequency[index]:*working_y[index])
        beta_delete = invsym(information-block_information)*(rhs-block_rhs)
        residual_delete = working_y[index]-block_design*beta_delete
        left = inverse*block_rhs
        right = inverse*(block_design'*(frequency[index]:*residual_delete))
        mover_correction[1] = mover_correction[1]+
            (left'*Q_worker*right)[1,1]
        mover_correction[2] = mover_correction[2]+
            (left'*Q_firm*right)[1,1]
        mover_correction[3] = mover_correction[3]+
            (left'*Q_covariance*right)[1,1]
    }
    mover_correction[4] = mover_correction[1]+mover_correction[2]+
        2*mover_correction[3]
    index = selectindex(stayer:==1)
    for (row=1; row<=rows(index); row++) {
        one = index[row]
        beta_delete = invsym(information-design[one,.]'*design[one,.]) *
            (rhs-design[one,.]'*working_y[one])
        residual_delete = working_y[one]-design[one,.]*beta_delete
        left = inverse*design[one,.]'*working_y[one]
        right = inverse*design[one,.]'*residual_delete
        stayer_correction[1] = stayer_correction[1]+frequency[one]*
            (left'*Q_worker*right)[1,1]
        stayer_correction[2] = stayer_correction[2]+frequency[one]*
            (left'*Q_firm*right)[1,1]
        stayer_correction[3] = stayer_correction[3]+frequency[one]*
            (left'*Q_covariance*right)[1,1]
    }
    stayer_correction[4] = stayer_correction[1]+stayer_correction[2]+
        2*stayer_correction[3]
    correction = mover_correction+stayer_correction
    correction[4] = correction[1]+correction[2]+2*correction[3]
    return(plugin\mover_correction\stayer_correction\correction\
        (plugin-correction))
}
end
mata: st_matrix("hybrid_oracle",vckss_test__hybrid_refit( ///
    "hybrid_use","hybrid_stayer","y","hybrid_worker", ///
    "hybrid_firm","c1 c2","frequency","target_mass", ///
    "match_id","joint"))
matrix hybrid_oracle = hybrid_oracle
matrix hybrid_joint_core = hybrid_joint[1..3,1..4]
matrix hybrid_oracle_core = (hybrid_oracle[1,1..4] \ ///
    hybrid_oracle[4,1..4] \ hybrid_oracle[5,1..4])
matrix hybrid_oracle_source = hybrid_oracle[2..3,1..4]
assert mreldif(hybrid_joint_core,hybrid_oracle_core) < 2e-9
assert mreldif(hybrid_source,hybrid_oracle_source) < 2e-9

// Arbitrary deletion-ID relabeling cannot change the deletion partition.
generate long match_relabel = 900000-17*match_id
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_relabel) algorithm(exact) ///
    nuisance(joint) targetweight(target_mass) stayers(both) nodisplay
assert mreldif(hybrid_joint,e(stayer_hybrid_results)) < 2e-10
assert original_order == _n

// Explicit target mass is stored-row mass, not frequency-scaled mass.
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_id) algorithm(exact)    ///
    nuisance(joint) stayers(both) nodisplay
assert e(stayer_hybrid_target_mass) == 30
assert mreldif(hybrid_joint,e(stayer_hybrid_results)) > 1e-8

// The fixed-offset convention uses the combined full-fit control index but
// excludes controls from every deletion correction design.
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_id) algorithm(exact)    ///
    nuisance(fixedoffset) targetweight(target_mass) stayers(both) ///
    backend(mata) rng(stata) nodisplay
matrix mata_hybrid_fixed = e(stayer_hybrid_results)
matrix mata_hybrid_fixed_source = e(stayer_hybrid_correction_source)
if `rust_available' {
    fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
        deletion(match) deletionid(match_id) algorithm(exact)    ///
        nuisance(fixedoffset) targetweight(target_mass) stayers(both) ///
        backend(rust) rng(counter_v1) nodisplay
    assert mreldif(mata_hybrid_fixed,e(stayer_hybrid_results)) < 2e-9
    assert mreldif(mata_hybrid_fixed_source,                      ///
        e(stayer_hybrid_correction_source)) < 2e-9
}
matrix hybrid_fixed = e(stayer_hybrid_results)
assert e(stayer_hybrid_full_parameters) == 13
assert e(stayer_hybrid_corr_parameters) == 11
mata: st_matrix("fixed_oracle",vckss_test__hybrid_refit( ///
    "hybrid_use","hybrid_stayer","y","hybrid_worker", ///
    "hybrid_firm","c1 c2","frequency","target_mass", ///
    "match_id","fixedoffset"))
matrix fixed_oracle = fixed_oracle
matrix hybrid_fixed_core = hybrid_fixed[1..3,1..4]
matrix fixed_oracle_core = (fixed_oracle[1,1..4] \ ///
    fixed_oracle[4,1..4] \ fixed_oracle[5,1..4])
matrix fixed_oracle_source = fixed_oracle[2..3,1..4]
matrix hybrid_fixed_source = e(stayer_hybrid_correction_source)
assert mreldif(hybrid_fixed_core,fixed_oracle_core) < 2e-9
assert mreldif(hybrid_fixed_source,fixed_oracle_source) < 2e-9

// Expanding literal copies and dividing explicit stored-row target mass over
// the copies reproduces both mixed-deletion point estimates.
preserve
generate long copies = frequency
generate double copy_target = target_mass/copies
expand copies
replace frequency = 1
fevc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match_id) algorithm(exact) nuisance(joint)     ///
    targetweight(copy_target) stayers(both) nodisplay
matrix hybrid_expanded = e(stayer_hybrid_results)
matrix hybrid_expanded_source = e(stayer_hybrid_correction_source)
mata: st_matrix("expanded_oracle",vckss_test__hybrid_refit( ///
    "hybrid_use","hybrid_stayer","y","hybrid_worker", ///
    "hybrid_firm","c1 c2","frequency","copy_target", ///
    "match_id","joint"))
matrix expanded_oracle = expanded_oracle
matrix hybrid_expanded_core = hybrid_expanded[1..3,1..4]
matrix expanded_oracle_core = (expanded_oracle[1,1..4] \ ///
    expanded_oracle[4,1..4] \ expanded_oracle[5,1..4])
matrix expanded_oracle_source = expanded_oracle[2..3,1..4]
assert mreldif(hybrid_joint,hybrid_expanded) < 2e-9
assert mreldif(hybrid_expanded_core,expanded_oracle_core) < 2e-9
assert mreldif(hybrid_expanded_source,expanded_oracle_source) < 2e-9
assert e(stayer_hybrid_N_physical) == 30
assert abs(e(stayer_hybrid_target_mass)-36) < 1e-12
restore
assert original_order == _n
assert `"`c(rngstate)'"' == `"`caller_rng'"'

// A request with no eligible stayers reduces exactly to the mover result.
preserve
keep in 1/24
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_id) algorithm(exact)    ///
    nuisance(joint) targetweight(target_mass) stayers(movers) nodisplay
matrix no_stayer_headline = e(results)
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match_id) algorithm(exact)    ///
    nuisance(joint) targetweight(target_mass) stayers(both) nodisplay
assert "`e(stayer_hybrid_status)'" == "CONVERGED"
assert e(stayer_hybrid_N_stayers) == 0
assert e(stayer_hybrid_N_stayer_rows) == 0
assert mreldif(no_stayer_headline,e(stayer_hybrid_results)) < 2e-12
assert mreldif(no_stayer_headline,e(results)) < 2e-12
matrix no_stayer_source = e(stayer_hybrid_correction_source)
forvalues component=1/4 {
    assert no_stayer_source[2,`component'] == 0
}
if `rust_available' {
    fevc y [fw=frequency], worker(worker) firm(firm)             ///
        deletion(match) deletionid(match_id) algorithm(jla)     ///
        engine(auto) probes(64) batch(7) seed(20260831)          ///
        backend(rust) rng(counter_v1) stayers(movers) nodisplay
    matrix no_stayer_rust_mover = e(results)
    assert "`e(engine_selected)'" == "compressed"
    fevc y [fw=frequency], worker(worker) firm(firm)             ///
        deletion(match) deletionid(match_id) algorithm(jla)     ///
        engine(auto) probes(64) batch(7) seed(20260831)          ///
        backend(rust) rng(counter_v1) stayers(both) nodisplay
    assert "`e(engine_selected)'" == "compressed"
    assert "`e(stayers)'" == "both"
    assert "`e(target_population)'" ==                          ///
        "retained movers plus eligible attached stayers"
    assert e(stayer_hybrid_N_stayers) == 0
    assert e(stayer_hybrid_N_stayer_rows) == 0
    assert mreldif(no_stayer_rust_mover,e(results)) < 2e-12
    assert mreldif(e(results),e(stayer_hybrid_results)) < 2e-12
    matrix no_stayer_rust_accounting =                           ///
        e(stayer_hybrid_sample_accounting)
    forvalues column=1/5 {
        assert no_stayer_rust_accounting[2,`column'] == 0
        assert no_stayer_rust_accounting[1,`column'] ==          ///
            no_stayer_rust_accounting[3,`column']
    }
}
restore

// Observation deletion remains incompatible with the mixed convention.
capture noisily fevc y c1 c2 [fw=frequency], worker(worker) ///
    firm(firm) deletion(observation) algorithm(exact) stayers(both) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "STAYER_HYBRID_DELETION_UNSUPPORTED"
// The Mata and Rust generic JLA implementations estimate the same combined
// target; exact parity is assessed against each estimator's reported MCSE.
fevc y c1 c2 [fw=frequency], worker(worker) firm(firm)            ///
    deletion(match) deletionid(match_id) algorithm(jla)           ///
    engine(generic) preconditioner(diagonal) probes(1024) batch(31) ///
    seed(20260831) nuisance(joint) targetweight(target_mass)       ///
    backend(mata) rng(stata) stayers(both) nodisplay
matrix mata_jla = e(results)
forvalues component=1/4 {
    assert abs(mata_jla[2,`component']-hybrid_joint[2,`component']) <= ///
        12*mata_jla[4,`component']+2e-6
}
assert e(N_retained)==28 & e(N_physical)==30
assert e(deletion_units)==e(stayer_hybrid_deletion_units)
if `rust_available' {
    capture noisily fevc y c1 c2 [fw=frequency], worker(worker) ///
        firm(firm) deletion(match) deletionid(match_id) algorithm(exact) ///
        backend(rust) rng(counter_v1) stayers(both) nodisplay
    assert _rc == 0
    assert "`e(backend_selected)'" == "rust"
    assert "`e(stayer_hybrid_status)'" == "CONVERGED"
    fevc y c1 c2 [fw=frequency], worker(worker) firm(firm)        ///
        deletion(match) deletionid(match_id) algorithm(jla)       ///
        engine(generic) preconditioner(diagonal) probes(1024) batch(31) ///
        seed(20260831) nuisance(joint) targetweight(target_mass)   ///
        backend(rust) rng(counter_v1) stayers(both) nodisplay
    matrix rust_jla = e(results)
    forvalues component=1/4 {
        assert abs(rust_jla[2,`component']-hybrid_joint[2,`component']) <= ///
            12*rust_jla[4,`component']+2e-6
    }
    assert e(N_retained)==28 & e(N_physical)==30
}
capture noisily fevc y c1 c2 [fw=frequency], worker(worker) ///
    firm(firm) deletion(match) deletionid(match_id) algorithm(exact) ///
    nuisance(joint) targetweight(target_mass) stayers(both)       ///
    exact_limit(11) nodisplay
assert _rc == 198
assert "`e(status)'" == "WITHHELD"
assert "`e(withholding_status)'" == "EXACT_SIZE_LIMIT"
capture matrix list e(results)
assert _rc != 0
assert original_order == _n
assert `"`c(rngstate)'"' == `"`caller_rng'"'

di as result "PASS test_stayers_hybrid.do"
