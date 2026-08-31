version 18.0
clear all
set more off
set varabbrev off

local pkgroot "fevc"
capture confirm file "`pkgroot'/vckss.mata"
if _rc {
    local pkgroot "../.."
    capture confirm file "`pkgroot'/vckss.mata"
    if _rc {
        di as error "run from the repository root or fevc/tests/stata"
        exit 601
    }
}

quietly do "`pkgroot'/vckss.mata"
quietly do "`pkgroot'/vckss_scale.mata"
quietly do "`pkgroot'/vckss_rng.mata"
quietly do "`pkgroot'/vckss_scale_engine.mata"

mata:
real scalar ksse_oracle__reldif(real matrix left, real matrix right)
{
    return(max(abs(left-right))/max((1,max(abs(left)),max(abs(right)))))
}

real matrix ksse_oracle__design(
    struct vckss_scale_design scalar design)
{
    real scalar cell
    real matrix out

    out = J(design.coefficient_cells,
        design.worker_levels+design.firm_levels-1,0)
    for (cell=1; cell<=design.coefficient_cells; cell++) {
        out[cell,design.cell_worker[cell]] = 1
        if (design.cell_firm[cell] < design.firm_levels) {
            out[cell,design.worker_levels+design.cell_firm[cell]] = 1
        }
    }
    return(out)
}

real rowvector ksse_oracle__plugin(
    struct vckss_scale_design scalar design,
    real colvector coefficient)
{
    real colvector alpha, gamma, worker_effect, firm_effect
    real scalar covariance, firm_mean, firm_variance, mass
    real scalar worker_mean, worker_variance

    mass = sum(design.cell_target_mass)
    alpha = coefficient[1..design.worker_levels]
    gamma = coefficient[(design.worker_levels+1)..rows(coefficient)] \ 0
    worker_effect = alpha[design.cell_worker]
    firm_effect = gamma[design.cell_firm]
    worker_mean = sum(design.cell_target_mass:*worker_effect)/mass
    firm_mean = sum(design.cell_target_mass:*firm_effect)/mass
    worker_variance = sum(design.cell_target_mass:*
        (worker_effect:-worker_mean):^2)/mass
    firm_variance = sum(design.cell_target_mass:*
        (firm_effect:-firm_mean):^2)/mass
    covariance = sum(design.cell_target_mass:*
        (worker_effect:-worker_mean):*(firm_effect:-firm_mean))/mass
    return((worker_variance,firm_variance,covariance,
        worker_variance+firm_variance+2*covariance))
}

void ksse_oracle__run()
{
    pointer scalar route_callback
    struct vckss_scale_design scalar design
    struct vckss_scale_engine_result scalar out, alternate, invalid
    struct vckss_scale_engine_result scalar rng_out, rng_alternate
    struct vckss_scale_engine_matrix_atoms scalar atom_source
    struct vckss_scale_atom_provider scalar provider, rng_provider
    struct vckss_scale_atom_provider scalar rng_provider_alternate
    struct vckss_scale_atom_provider scalar bad_rng_provider
    struct vckss_scale_rng_context scalar rng_context, rng_context_alternate
    struct vckss_scale_rng_context scalar bad_rng_context
    struct vckss_scale_route_context scalar route_context
    struct vckss_scale_engine_atom_batch scalar provider_atoms
    struct vckss_rng__result scalar generated
    struct vckss_rng__snapshot scalar saved_rng
    struct vckss_result scalar ordinary, external_ordinary
    struct vckss_scale_unit_adjust scalar adjustment
    struct vckss_fe_design scalar base, bad_base, external_base
    struct vckss_solver_backend scalar backend
    real colvector unit_rank, stratum_rank
    real colvector worker, firm, deletion, frequency, outcome, target
    real colvector p1, m1, p2, m2, pm, pshare, mshare, bias, variance
    real colvector residual_mass, deleted_mass, cell_weight, cell_atoms
    real colvector first, direction, score, rhs_worker, rhs_firm
    real colvector z_worker, z_firm, z_total, fitted, raw_residual
    real colvector out_beta, projection
    real colvector bad_trials
    real matrix unit_atoms, target_atoms, invalid_atoms, X, H, H_inverse
    real matrix expected_atoms
    real matrix draws
    real colvector stage_ids
    real rowvector plugin, correction
    real scalar group, probe, stratum, probes, parameters
    real scalar epsilon, block_tolerance, one_m, one_p, residual_share, ulp

    /* Repeated rows, literal frequencies, multiple deletion IDs in cells 1
       and 6, and unequal exact per-copy target scales in cells 1, 2, and 4. */
    worker =   (1\1\1\1\1\2\2\2\3\3\3\3)
    firm =     (1\1\1\2\2\1\2\2\1\1\2\2)
    deletion = (101\101\102\103\103\104\105\105\106\106\107\108)
    frequency =(2\1\3\2\1\2\2\1\1\2\3\1)
    outcome =  (1.2\.8\1.5\2.1\1.9\-.4\.3\.7\1.1\.9\-.2\.2)
    target =   (2\2\3\2\.5\2\4\1\1\2\3\1)
    design = vckss_scale__prepare(
        worker,firm,deletion,frequency,outcome,target,1e-12)
    assert(vckss_scale_engine__api_level() == 4)
    assert(vckss_scale_engine__build_id() ==
        "vckss-scale-engine-api4-fe-buf1-buffered")
    assert(design.status == "CONVERGED")
    assert(design.coefficient_cells == 6)
    assert(design.deletion_units == 8)
    assert(design.strata.count == 9)
    assert(sum(design.unit_cell:==1) == 2)
    assert(sum(design.unit_cell:==6) == 2)

    /* Repair a constant-field roundoff remainder without classifying a
       genuinely near-null direction as zero. */
    first = J(design.coefficient_cells,1,sqrt(1/6))
    direction = vckss_scale_eng__center_target(
        first,J(design.coefficient_cells,1,1/6))
    score = vckss_scale_eng__balance_score(
        direction,(sum(abs(first))+abs(sum(first))))
    assert(rows(score) == design.coefficient_cells)
    assert(sum(score) == 0)
    direction = J(design.coefficient_cells,1,0)
    direction[1] = 1e-14
    direction[2] = -1e-14
    first = first+direction
    score = vckss_scale_eng__center_target(
        first,J(design.coefficient_cells,1,1/6))
    score = vckss_scale_eng__balance_score(
        score,(sum(abs(first))+abs(sum(first))))
    assert(rows(score) == design.coefficient_cells)
    assert(vckss__norm2(score) > 1e-15)
    assert(sum(score) == 0)

    probes = 40
    rseed(20260816)
    unit_atoms = J(design.deletion_units,probes,.)
    for (group=1; group<=design.deletion_units; group++) {
        for (probe=1; probe<=probes; probe++) {
            unit_atoms[group,probe] = sum(2:*
                rbinomial(design.unit_frequency[group],1,1,.5):-1)
        }
    }
    target_atoms = J(design.strata.count,probes,.)
    for (stratum=1; stratum<=design.strata.count; stratum++) {
        for (probe=1; probe<=probes; probe++) {
            target_atoms[stratum,probe] = sum(2:*
                rbinomial(design.strata.physical_count[stratum],1,1,.5):-1)
        }
    }
    assert(vckss_scale_eng__valid_atoms(
        unit_atoms,design.unit_frequency))
    assert(vckss_scale_eng__valid_atoms(
        target_atoms,design.strata.physical_count))

    backend = vckss__diagonal_backend()
    out = vckss_scale_eng__run_atoms(
        design,backend,unit_atoms,target_atoms,7,9,
        1e-12,10000,1e-12,1e-10)
    assert(out.status == "CONVERGED")
    assert(out.rng_contract == "TEST_MATRIX_ATOMS")
    assert(out.solver_route == "DIAGONAL")
    assert(rows(out.solver_rhs_diagnostics) == 1+3*probes)
    stage_ids = uniqrows(sort(out.solver_rhs_diagnostics[.,1],1))
    assert(rows(stage_ids) == 3)
    assert(max(abs(stage_ids-(2\4\5))) == 0)
    assert(max(abs(select(out.solver_rhs_diagnostics[.,3],
        out.solver_rhs_diagnostics[.,1]:==4)-(1::probes))) == 0)
    assert(max(abs(select(out.solver_rhs_diagnostics[.,3],
        out.solver_rhs_diagnostics[.,1]:==5)-(1::(2*probes)))) == 0)
    assert(out.max_complete_residual <= 1e-11)
    assert(out.max_reciprocal_residual <= 1e-10)
    assert(out.target_identity_residual <= 1e-12)

    /* The routed callback consumes an already prepared coefficient-cell FE
       design and returns the ordinary installed-result contract. */
    atom_source.leverage_unit = unit_atoms
    atom_source.target_stratum = target_atoms
    provider = vckss_scale_eng__mat_provider(&atom_source)
    route_context = vckss_scale_eng__route_context(
        &design,provider,probes,7,9,1e-12,10000,1e-12,1e-10)
    base = vckss__fe_prepare(
        design.cell_worker,design.cell_firm,design.cell_frequency,1e-12)
    route_callback = vckss_scale_eng__callback_ptr()
    assert(route_callback != NULL)
    ordinary = (*route_callback)(&route_context,base,backend,.25)
    assert(ordinary.status == "CONVERGED")
    assert(route_context.last.status == "CONVERGED")
    assert(ksse_oracle__reldif(ordinary.plugin,out.plugin) < 2e-10)
    assert(ksse_oracle__reldif(
        ordinary.correction,out.correction) < 2e-9)
    assert(ordinary.preconditioner_seconds == .25)
    assert(ordinary.preconditioner_ratio == base.preconditioner_ratio)
    assert(rows(ordinary.solver_rhs_diagnostics) == 1+3*probes)
    external_base = vckss_scale__fe_view(&design)
    assert(external_base.status == "CONVERGED")
    assert(external_base.external_operator == 1)
    assert(rows(external_base.worker) == 0)
    assert(rows(external_base.firm) == 0)
    assert(rows(external_base.frequency) == 0)
    assert(rows(external_base.worker_order) == 0)
    assert(rows(external_base.firm_order) == 0)
    external_ordinary = (*route_callback)(
        &route_context,external_base,backend,.25)
    assert(external_ordinary.status == "CONVERGED")
    assert(ksse_oracle__reldif(
        external_ordinary.plugin,ordinary.plugin) < 2e-10)
    assert(ksse_oracle__reldif(
        external_ordinary.correction,ordinary.correction) < 2e-9)
    assert(external_ordinary.solver_max_residual <= 1e-11)
    bad_base = base
    bad_base.frequency[1] = bad_base.frequency[1]+1
    ordinary = vckss_scale_eng__route_callback(
        &route_context,bad_base,backend,.25)
    assert(ordinary.status == "FASTPATH_BASE_MISMATCH")

    /* Registered RNG output is restored from canonical semantic-key order
       to the deliberately permuted design order before support validation.
       The provider restores the caller's complete RNG state on every call. */
    unit_rank = (8\1\7\2\6\3\5\4)
    stratum_rank = (9\1\8\2\7\3\6\4\5)
    assert(max(abs(vckss_rng__canonical_order(unit_rank)-
        (1..rows(unit_rank))')) > 0)
    assert(max(abs(vckss_rng__canonical_order(stratum_rank)-
        (1..rows(stratum_rank))')) > 0)
    saved_rng = vckss_rng__capture()
    rng_context = vckss_scale_eng__rng_context(
        "per_domain_stream_cursor",24681357,
        unit_rank,design.unit_frequency,
        stratum_rank,design.strata.physical_count)
    assert(rng_context.status == "CONVERGED")
    assert(rng_context.leverage_initialized == 0)
    assert(rng_context.target_initialized == 0)
    assert(rngstate() == saved_rng.state)
    rng_provider = vckss_scale_eng__rng_provider(&rng_context)
    assert(rng_provider.status == "CONVERGED")
    assert(rng_context.leverage_initialized == 0)
    assert(rng_context.target_initialized == 0)
    assert(rngstate() == saved_rng.state)
    provider_atoms = (*rng_provider.leverage)(
        rng_provider.context,1,4)
    assert(provider_atoms.status == "CONVERGED")
    assert(rng_context.leverage_initialized == 1)
    assert(rng_context.target_initialized == 0)
    generated = vckss_rng__generate(
        "per_domain_stream",24681357,"leverage",1,4,
        unit_rank,design.unit_frequency)
    assert(generated.status == "OK")
    expected_atoms = J(design.deletion_units,4,.)
    expected_atoms[generated.canonical_order,.] = generated.atoms
    assert(ksse_oracle__reldif(
        provider_atoms.value,expected_atoms) == 0)
    provider_atoms = (*rng_provider.target)(rng_provider.context,1,3)
    generated = vckss_rng__generate(
        "per_domain_stream",24681357,"target",1,3,
        stratum_rank,design.strata.physical_count)
    expected_atoms = J(design.strata.count,3,.)
    expected_atoms[generated.canonical_order,.] = generated.atoms
    assert(provider_atoms.status == "CONVERGED")
    assert(generated.status == "OK")
    assert(rng_context.target_initialized == 1)
    assert(ksse_oracle__reldif(
        provider_atoms.value,expected_atoms) == 0)
    assert(st_global("c(rng)") == saved_rng.algorithm)
    assert(st_numscalar("c(rngstream)") == saved_rng.stream)
    assert(rngstate() == saved_rng.state)
    provider_atoms = (*rng_provider.leverage)(
        rng_provider.context,6,6)
    assert(provider_atoms.status == "RNG_SEQUENCE_MISMATCH")
    provider_atoms = (*rng_provider.leverage)(
        rng_provider.context,5,6)
    assert(provider_atoms.status == "CONVERGED")
    generated = vckss_rng__generate(
        "per_domain_stream",24681357,"leverage",5,2,
        unit_rank,design.unit_frequency)
    expected_atoms = J(design.deletion_units,2,.)
    expected_atoms[generated.canonical_order,.] = generated.atoms
    assert(ksse_oracle__reldif(
        provider_atoms.value,expected_atoms) == 0)
    assert(rngstate() == saved_rng.state)

    /* Fresh cursors drive complete estimator runs.  Different numerical
       batch partitions consume the identical atom sequence and need only
       satisfy the registered floating-point tolerance. */
    rng_context = vckss_scale_eng__rng_context(
        "per_domain_stream_cursor",24681357,
        unit_rank,design.unit_frequency,
        stratum_rank,design.strata.physical_count)
    rng_provider = vckss_scale_eng__rng_provider(&rng_context)
    assert(rng_context.leverage_initialized == 0)
    assert(rng_context.target_initialized == 0)
    rng_out = vckss_scale_engine__run(
        design,backend,rng_provider,200,7,9,
        1e-12,10000,1e-12,1e-10)
    rng_context_alternate = vckss_scale_eng__rng_context(
        "per_domain_stream_cursor",24681357,
        unit_rank,design.unit_frequency,
        stratum_rank,design.strata.physical_count)
    rng_provider_alternate = vckss_scale_eng__rng_provider(
        &rng_context_alternate)
    rng_alternate = vckss_scale_engine__run(
        design,backend,rng_provider_alternate,200,11,13,
        1e-12,10000,1e-12,1e-10)
    assert(rng_out.status == "CONVERGED")
    assert(rng_alternate.status == "CONVERGED")
    assert(rng_context.leverage_initialized == 1)
    assert(rng_context.target_initialized == 1)
    assert(rng_out.rng_contract == vckss_rng__production_contract())
    assert(ksse_oracle__reldif(
        rng_out.unit_projection_share,
        rng_alternate.unit_projection_share) < 2e-10)
    assert(ksse_oracle__reldif(
        rng_out.correction,rng_alternate.correction) < 2e-9)
    assert(rngstate() == saved_rng.state)

    /* A production provider must bind its binomial trials exactly to the
       design masses; compatible-looking atom support is not enough. */
    bad_trials = design.unit_frequency
    bad_trials[1] = bad_trials[1]+1
    bad_rng_context = vckss_scale_eng__rng_context(
        "per_domain_stream_cursor",24681357,unit_rank,bad_trials,
        stratum_rank,design.strata.physical_count)
    bad_rng_provider = vckss_scale_eng__rng_provider(&bad_rng_context)
    invalid = vckss_scale_engine__run(
        design,backend,bad_rng_provider,probes,7,9,
        1e-12,10000,1e-12,1e-10)
    assert(invalid.status == "PROBE_TRIAL_MISMATCH")
    assert(rngstate() == saved_rng.state)

    /* Independent dense grounded normal equations. */
    X = ksse_oracle__design(design)
    parameters = cols(X)
    H = X'*(design.cell_frequency:*X)
    H_inverse = invsym(H)
    out_beta = H_inverse*(X'*design.cell_outcome_sum)
    fitted = X*out_beta
    assert(ksse_oracle__reldif(out.coefficient,out_beta) < 2e-10)
    assert(ksse_oracle__reldif(out.fitted_cell,fitted) < 2e-10)
    plugin = ksse_oracle__plugin(design,out_beta)
    assert(ksse_oracle__reldif(out.plugin,plugin) < 2e-10)
    raw_residual = outcome-fitted[design.row_to_cell]
    assert(abs(out.weighted_rss-sum(frequency:*raw_residual:^2)) < 2e-10)

    /* Dense projection moments by deletion unit. */
    p1 = m1 = p2 = m2 = pm = J(design.deletion_units,1,0)
    for (probe=1; probe<=probes; probe++) {
        cell_atoms = J(design.coefficient_cells,1,0)
        for (group=1; group<=design.deletion_units; group++) {
            cell_atoms[design.unit_cell[group]] =
                cell_atoms[design.unit_cell[group]]+unit_atoms[group,probe]
        }
        projection = X*(H_inverse*(X'*cell_atoms))
        for (group=1; group<=design.deletion_units; group++) {
            one_p = sqrt(design.unit_frequency[group])*
                projection[design.unit_cell[group]]
            one_m = unit_atoms[group,probe]/
                sqrt(design.unit_frequency[group])-one_p
            p1[group] = p1[group]+one_p^2
            m1[group] = m1[group]+one_m^2
            p2[group] = p2[group]+one_p^4
            m2[group] = m2[group]+one_m^4
            pm[group] = pm[group]+one_p^2*one_m^2
        }
    }
    pshare = (p1:/probes):/((p1+m1):/probes)
    mshare = (m1:/probes):/((p1+m1):/probes)
    p2 = p2:/probes
    m2 = m2:/probes
    pm = pm:/probes
    variance = (mshare:^2:*p2+pshare:^2:*m2-
        2:*pshare:*mshare:*pm):/probes
    bias = (mshare:*p2-pshare:*m2+(mshare-pshare):*pm):/probes
    variance = variance:*(variance:>0)
    residual_mass = design.unit_outcome_sum-
        design.unit_frequency:*fitted[design.unit_cell]
    deleted_mass = residual_mass:*(1:/mshare+bias:/mshare:^2-
        variance:/mshare:^3)
    cell_weight = J(design.coefficient_cells,1,0)
    for (group=1; group<=design.deletion_units; group++) {
        cell_weight[design.unit_cell[group]] =
            cell_weight[design.unit_cell[group]]+
            design.unit_outcome_sum[group]*deleted_mass[group]
    }
    assert(ksse_oracle__reldif(
        out.unit_projection_share,pshare) < 2e-10)
    assert(ksse_oracle__reldif(
        out.unit_residual_share,mshare) < 2e-10)
    assert(ksse_oracle__reldif(out.unit_finite_bias,bias) < 2e-10)
    assert(ksse_oracle__reldif(
        out.unit_finite_variance,variance) < 2e-10)
    assert(ksse_oracle__reldif(out.unit_d,deleted_mass) < 2e-9)
    assert(ksse_oracle__reldif(
        out.cell_correction_weight,cell_weight) < 2e-9)

    /* Dense target solves and K_c contractions. */
    draws = J(probes,4,.)
    for (probe=1; probe<=probes; probe++) {
        first = J(design.coefficient_cells,1,0)
        for (stratum=1; stratum<=design.strata.count; stratum++) {
            first[design.strata.cell[stratum]] =
                first[design.strata.cell[stratum]]+
                sqrt(design.strata.per_copy_mass[stratum]/
                    design.target_weight_sum)*target_atoms[stratum,probe]
        }
        direction = first-(design.cell_target_mass:/
            design.target_weight_sum)*sum(first)
        score = X'*direction
        rhs_worker = J(parameters,1,0)
        rhs_worker[1..design.worker_levels] =
            score[1..design.worker_levels]
        rhs_firm = J(parameters,1,0)
        rhs_firm[(design.worker_levels+1)..parameters] =
            score[(design.worker_levels+1)..parameters]
        z_worker = X*(H_inverse*rhs_worker)
        z_firm = X*(H_inverse*rhs_firm)
        z_total = z_worker+z_firm
        draws[probe,1] = sum(cell_weight:*z_worker:^2)
        draws[probe,2] = sum(cell_weight:*z_firm:^2)
        draws[probe,4] = sum(cell_weight:*z_total:^2)
        draws[probe,3] = .5*(draws[probe,4]-draws[probe,1]-draws[probe,2])
    }
    correction = colsum(draws):/probes
    assert(ksse_oracle__reldif(out.target_draws,draws) < 2e-9)
    assert(ksse_oracle__reldif(out.correction,correction) < 2e-9)
    assert(ksse_oracle__reldif(
        out.corrected,plugin-correction) < 2e-9)

    /* Atom identity is independent of numerical batching. */
    alternate = vckss_scale_eng__run_atoms(
        design,backend,unit_atoms,target_atoms,11,13,
        1e-12,10000,1e-12,1e-10)
    assert(alternate.status == "CONVERGED")
    assert(ksse_oracle__reldif(out.plugin,alternate.plugin) < 2e-10)
    assert(ksse_oracle__reldif(out.correction,alternate.correction) < 2e-9)
    assert(ksse_oracle__reldif(
        out.unit_projection_share,alternate.unit_projection_share) < 2e-10)

    /* Invalid sign support is rejected before its solve. */
    invalid_atoms = unit_atoms
    invalid_atoms[1,1] = invalid_atoms[1,1]+1
    invalid = vckss_scale_eng__run_atoms(
        design,backend,invalid_atoms,target_atoms,7,9,
        1e-12,10000,1e-12,1e-10)
    assert(invalid.status == "INVALID_PROBE_ATOM")

    /* The scalar specialization retains the coefficient-one formula and
       inherits the generic engine's exact strict block gate. */
    adjustment = vckss_scale_eng__unit_adjust(
        .37,.63,-.004,.0007,2.5,1e-12,1e-10)
    assert(adjustment.status == "CONVERGED")
    assert(abs(adjustment.deleted_mass-
        2.5*(1/.63-.004/.63^2-.0007/.63^3)) < 1e-14)
    block_tolerance = 1e-8
    epsilon = 2.2204460492503131e-16
    for (ulp=-32; ulp<=32; ulp=ulp+8) {
        residual_share = block_tolerance+ulp*epsilon
        adjustment = vckss_scale_eng__unit_adjust(
            1-residual_share,residual_share,0,0,1,
            1e-12,block_tolerance)
        if (1-(1-residual_share) <= block_tolerance) {
            assert(adjustment.status == "NONESTIMABLE_DELETION")
        }
        else assert(adjustment.status == "CONVERGED")
    }
    residual_share = block_tolerance-128*epsilon
    adjustment = vckss_scale_eng__unit_adjust(
        1-residual_share,residual_share,0,0,1,
        1e-12,block_tolerance)
    assert(adjustment.status == "NONESTIMABLE_DELETION")
    residual_share = block_tolerance+128*epsilon
    adjustment = vckss_scale_eng__unit_adjust(
        1-residual_share,residual_share,0,0,1,
        1e-12,block_tolerance)
    assert(adjustment.status == "CONVERGED")
}

ksse_oracle__run()
mata drop ksse_oracle__*()
end

di as result "PASS test_scale_engine.do"
exit 0
