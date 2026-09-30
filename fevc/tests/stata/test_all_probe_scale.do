version 18.0

mata:
struct vckss_scale_engine_atom_batch scalar fevc_test_nmc_bad_provider(
    pointer scalar context, real scalar first, real scalar last)
{
    return(vckss_scale_eng__rng_lev(context,first,last))
}

void fevc_test_nmc_scale()
{
    struct vckss_scale_design scalar design
    struct vckss_fe_design scalar base, raw_base
    struct vckss_solver_backend scalar backend
    struct vckss_scale_rng_context scalar rng
    struct vckss_scale_atom_provider scalar provider, bad_provider
    struct vckss_scale_engine_result scalar ordinary, enabled
    struct vckss_nmc__attachment scalar attachment, generic
    struct vckss_result scalar raw
    struct vckss_rng__full_snapshot scalar caller
    struct vckss_rng__snapshot scalar after
    real colvector worker, firm, deletion, frequency, y, mass, rank, unit_rank, target_rank
    real scalar g, R, seed
    worker =   (1\1\1\1\1\2\2\2\3\3\3\3)
    firm =     (1\1\1\2\2\1\2\2\1\1\2\2)
    deletion = (101\101\102\103\103\104\105\105\106\106\107\108)
    frequency =(2\1\3\2\1\2\2\1\1\2\3\1)
    y =  (1.2\.8\1.5\2.1\1.9\-.4\.3\.7\1.1\.9\-.2\.2)
    mass =   (2\2\3\2\.5\2\4\1\1\2\3\1)
    rank = (1..rows(y))'
    design = vckss_scale__prepare(worker,firm,deletion,frequency,y,mass,1e-12)
    assert(design.status=="CONVERGED")
    // Independently address minima of the original row ranks per group.
    unit_rank = J(design.deletion_units,1,.)
    for (g=1; g<=design.deletion_units; g++) {
        unit_rank[g] = min(rank[design.unit_row_order[|
            design.unit_row_panel[g,1]\design.unit_row_panel[g,2]|]])
    }
    target_rank = J(design.strata.count,1,.)
    for (g=1; g<=design.strata.count; g++) {
        target_rank[g] = min(rank[selectindex(design.strata.row_to_stratum:==g)])
    }
    R=33;seed=2026092911
    base = vckss__fe_prepare(design.cell_worker,design.cell_firm,design.cell_frequency,1e-12)
    backend = vckss__diagonal_backend()
    caller = vckss_rng__capture_full()
    rng = vckss_scale_eng__rng_context("per_domain_stream_cursor",seed,unit_rank,
        design.unit_frequency,target_rank,design.strata.physical_count)
    provider = vckss_scale_eng__rng_provider(&rng)
    ordinary = vckss_scale_eng__run_prepared(design,base,backend,provider,R,7,5,
        1e-12,10000,1e-12,1e-10)
    assert(ordinary.status=="CONVERGED")
    rng = vckss_scale_eng__rng_context("per_domain_stream_cursor",seed,unit_rank,
        design.unit_frequency,target_rank,design.strata.physical_count)
    provider = vckss_scale_eng__rng_provider(&rng)
    enabled = vckss_scale_eng__run_prepared(design,base,backend,provider,R,7,5,
        1e-12,10000,1e-12,1e-10,&attachment)
    assert(enabled.status=="CONVERGED")
    assert(vckss__norm2(enabled.corrected-ordinary.corrected)/vckss__norm2(ordinary.corrected)<1e-11)
    assert(vckss__norm2(enabled.numerical_mcse-ordinary.numerical_mcse)/vckss__norm2(ordinary.numerical_mcse)<1e-11)
    assert(rows(attachment.replay_rhs)==R & attachment.replay_failure=="")
    assert(max(attachment.replay_rhs[.,3])<=1e-11)
    assert(rng.leverage_cursor.next_probe==R+1 & rng.target_cursor.next_probe==R+1)
    after = vckss_rng__capture()
    assert(after.state==caller.active.state & after.algorithm==caller.active.algorithm & after.stream==caller.active.stream)
    // A same-label custom provider cannot substitute unrelated atoms.
    rng = vckss_scale_eng__rng_context("per_domain_stream_cursor",seed,unit_rank,
        design.unit_frequency,target_rank,design.strata.physical_count)
    provider = vckss_scale_eng__rng_provider(&rng)
    bad_provider = provider
    bad_provider.leverage = &fevc_test_nmc_bad_provider()
    ordinary = vckss_scale_eng__run_prepared(design,base,backend,bad_provider,R,7,5,
        1e-12,10000,1e-12,1e-10,&generic)
    assert(ordinary.status=="INVALID_PROBE_PROVIDER")
    assert(rng.leverage_initialized==0 & rng.target_initialized==0)
    after = vckss_rng__capture()
    assert(after.state==caller.active.state & after.algorithm==caller.active.algorithm & after.stream==caller.active.stream)
    raw_base = vckss__fe_prepare(worker,firm,frequency,1e-12)
    raw = vckss__jla_backend(y,worker,firm,J(rows(y),0,.),frequency,mass,
        deletion,"match","joint",R,7,seed,1e-12,10000,1e-12,1e-10,100,
        raw_base,backend,0,rank,1,J(0,1,.),&generic)
    assert(raw.status=="CONVERGED")
    assert(vckss__norm2(raw.corrected-enabled.corrected)/vckss__norm2(raw.corrected)<1e-11)
    assert(vckss__norm2(generic.covariance.conditional-attachment.covariance.conditional)/
        vckss__norm2(generic.covariance.conditional)<1e-11)
    assert(vckss__norm2(generic.covariance.leverage-attachment.covariance.leverage)/
        vckss__norm2(generic.covariance.leverage)<1e-11)
    assert(generic.covariance.status==attachment.covariance.status)
    assert(vckss_rng__restore_full(caller)==0)
    // The admitted width exceeds eight; replay must retain the original cursor atoms.
    rng = vckss_scale_eng__rng_context("per_domain_stream_cursor",seed,unit_rank,
        design.unit_frequency,target_rank,design.strata.physical_count)
    provider = vckss_scale_eng__rng_provider(&rng)
    enabled = vckss_scale_eng__run_prepared(design,base,backend,provider,R,11,5,
        1e-12,10000,1e-12,1e-10,&attachment)
    assert(enabled.status=="CONVERGED" & rows(attachment.replay_rhs)==R)
    assert(vckss__norm2(raw.corrected-enabled.corrected)/vckss__norm2(raw.corrected)<1e-11)
    assert(vckss__norm2(generic.covariance.conditional-attachment.covariance.conditional)/
        vckss__norm2(generic.covariance.conditional)<1e-11)
    assert(vckss__norm2(generic.covariance.leverage-attachment.covariance.leverage)/
        vckss__norm2(generic.covariance.leverage)<1e-11)
    assert(rng.leverage_cursor.next_probe==R+1 & rng.target_cursor.next_probe==R+1)
    after = vckss_rng__capture()
    assert(after.state==caller.active.state & after.algorithm==caller.active.algorithm & after.stream==caller.active.stream)
    assert(vckss_rng__restore_full(caller)==0)
}
fevc_test_nmc_scale()
end

di as result "PASS test_all_probe_scale.do"
