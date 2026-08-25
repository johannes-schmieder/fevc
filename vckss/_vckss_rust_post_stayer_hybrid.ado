*! version 0.4.0-dev 24aug2026
program define _vckss_rust_post_stayer_hybrid, eclass sortpreserve
    version 18.0
    args depvar target hybridtouse nuisance nhystayers nhystayerrows ///
        nsingleton nunattached augctx resultctx rawctx sourcectx nodisplay

    foreach variable in `depvar' `target' `hybridtouse' {
        confirm numeric variable `variable'
    }
    tempname aug receipt raw source plugin correction corrected kss  ///
        decomposition accounting memory
    matrix `aug' = `augctx'
    matrix `receipt' = `resultctx'
    matrix `raw' = `rawctx'
    matrix `source' = `sourcectx'
    if rowsof(`aug')!=1 | colsof(`aug')!=25 |                       ///
        rowsof(`receipt')!=1 | colsof(`receipt')!=24 |              ///
        rowsof(`raw')!=4 | colsof(`raw')!=4 |                       ///
        rowsof(`source')!=2 | colsof(`source')!=4 {
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "The reconciled Rust stayer-hybrid posting context has an invalid shape."
        exit 498
    }

    matrix colnames `raw' = worker_variance firm_variance           ///
        worker_firm_covariance total_variance
    matrix rownames `raw' = plugin bias_correction corrected numerical_mcse
    matrix colnames `source' = worker_variance firm_variance        ///
        worker_firm_covariance total_variance
    matrix rownames `source' = mover_match stayer_observation
    matrix `plugin' = `raw'[1,1..4]
    matrix `correction' = `raw'[2,1..4]
    matrix `corrected' = `raw'[3,1..4]
    matrix `kss' = `corrected'
    foreach name in plugin correction corrected kss {
        matrix colnames ``name'' = worker_variance firm_variance    ///
            worker_firm_covariance total_variance
    }

    local target_variance = .
    tempvar target_sq
    quietly summarize `depvar' [aw=`target']                        ///
        if `hybridtouse' & `target'>0, meanonly
    if !_rc & !missing(r(mean)) {
        local target_mean = r(mean)
        quietly generate double `target_sq' =                       ///
            `target'*(`depvar'-`target_mean')^2 if `hybridtouse'
        quietly count if `hybridtouse' & `target'>0 & missing(`target_sq')
        if r(N)==0 {
            quietly summarize `target_sq' if `hybridtouse', meanonly
            local target_ss = r(sum)
            quietly summarize `target' if `hybridtouse', meanonly
            if r(sum)>0 local target_variance = `target_ss'/r(sum)
        }
    }

    matrix `decomposition' =                                      ///
        (`raw'[1,1],`raw'[2,1],`raw'[3,1] \                        ///
         `raw'[1,2],`raw'[2,2],`raw'[3,2] \                        ///
         2*`raw'[1,3],2*`raw'[2,3],2*`raw'[3,3] \                  ///
         `raw'[1,4],`raw'[2,4],`raw'[3,4])
    matrix `decomposition' = `decomposition',J(4,4,.)
    if !missing(`target_variance') & `target_variance'>0 {
        forvalues component=1/4 {
            matrix `decomposition'[`component',4] =                 ///
                `decomposition'[`component',1]/`target_variance'
            matrix `decomposition'[`component',5] =                 ///
                `decomposition'[`component',3]/`target_variance'
        }
    }
    if !missing(`decomposition'[4,1]) & `decomposition'[4,1]>0 {
        forvalues component=1/4 {
            matrix `decomposition'[`component',6] =                 ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if !missing(`decomposition'[4,3]) & `decomposition'[4,3]>0 {
        forvalues component=1/4 {
            matrix `decomposition'[`component',7] =                 ///
                `decomposition'[`component',3]/`decomposition'[4,3]
        }
    }
    matrix rownames `decomposition' = worker_variance firm_variance ///
        sorting_2covariance total_worker_firm
    matrix colnames `decomposition' = plugin bias_correction corrected ///
        plugin_share_outcome corrected_share_outcome                 ///
        plugin_share_worker_firm corrected_share_worker_firm

    matrix `accounting' =                                           ///
        (`aug'[1,3],`aug'[1,6],`aug'[1,9],`aug'[1,16],`aug'[1,13] \ ///
         `aug'[1,4],`aug'[1,7],`aug'[1,10],`aug'[1,17],`aug'[1,14] \ ///
         `aug'[1,5],`aug'[1,8],`aug'[1,11],                       ///
            `aug'[1,16]+`aug'[1,17],`aug'[1,15])
    matrix rownames `accounting' = mover stayer total
    matrix colnames `accounting' = stored_rows physical_observations ///
        worker_levels target_weight_mass deletion_units
    matrix `memory' = (`receipt'[1,21],`receipt'[1,22],              ///
        `receipt'[1,20],`aug'[1,23],`aug'[1,24],`aug'[1,25],        ///
        max(`receipt'[1,20],`aug'[1,23]),`aug'[1,21])
    matrix colnames `memory' = fit correction exact_solve           ///
        augmentation_peak augmented_resident total_prepared_resident ///
        command_peak limit

    ereturn scalar stayer_hybrid_N_stored = `aug'[1,5]
    ereturn scalar stayer_hybrid_N_physical = `aug'[1,8]
    ereturn scalar stayer_hybrid_worker_levels = `aug'[1,11]
    ereturn scalar stayer_hybrid_firm_levels = `aug'[1,12]
    ereturn scalar stayer_hybrid_parameters = `receipt'[1,3]
    ereturn scalar stayer_hybrid_full_parameters = `receipt'[1,4]
    ereturn scalar stayer_hybrid_corr_parameters = `receipt'[1,5]
    ereturn scalar stayer_hybrid_deletion_units = `receipt'[1,6]
    ereturn scalar stayer_hybrid_target_mass = `aug'[1,16]+`aug'[1,17]
    ereturn scalar stayer_hybrid_max_leverage = `receipt'[1,7]
    ereturn scalar stayer_hybrid_information_rcond = `receipt'[1,8]
    ereturn scalar stayer_hybrid_inverse_relres = `receipt'[1,9]
    ereturn scalar stayer_hybrid_weighted_rss = `receipt'[1,2]
    ereturn scalar stayer_hybrid_target_y_variance = `target_variance'
    ereturn scalar stayer_hybrid_N_stayers = `nhystayers'
    ereturn scalar stayer_hybrid_N_stayer_rows = `nhystayerrows'
    ereturn scalar stayer_hybrid_N_stayer_physical = `aug'[1,7]
    ereturn scalar stayer_hybrid_N_singleton_drop = `nsingleton'
    ereturn scalar stayer_hybrid_N_unattached = `nunattached'
    ereturn scalar stayer_hybrid_mover_target_mass = `aug'[1,16]
    ereturn scalar stayer_hybrid_stayer_target_mass = `aug'[1,17]
    ereturn scalar rust_stayer_topology_checksum_hi = `aug'[1,19]
    ereturn scalar rust_stayer_topology_checksum_lo = `aug'[1,20]
    ereturn scalar rust_stayer_source_acct_resid = `receipt'[1,24]
    ereturn scalar rust_stayer_accounting_residual = `receipt'[1,23]
    local mover_memory = e(memory_forecast_bytes)
    ereturn scalar memory_forecast_bytes = max(`mover_memory',       ///
        `aug'[1,23],`receipt'[1,20])
    ereturn local stayer_hybrid_status "CONVERGED"
    ereturn local stayer_hybrid_target_population                   ///
        "retained movers plus eligible original one-firm stayers attached to retained mover firms"
    ereturn local stayer_hybrid_deletion                            ///
        "mover matches plus stayer physical observations"
    ereturn local stayer_hybrid_assumption                          ///
        "mover correction is match-robust; stayer correction is not match-robust"
    ereturn local stayer_hybrid_sample_rule                         ///
        "original one-firm stayers; retained mover firm; physical T>=2; graph-dropped movers excluded"
    ereturn local stayer_hybrid_esample "e(sample) marks the mover headline only"
    ereturn local stayer_hybrid_targetweight                        ///
        "pooled stored-row target mass; explicit mass is not multiplied by frequency"
    ereturn local stayer_hybrid_nuisance "`nuisance'"
    // `ereturn matrix` takes ownership of a temporary matrix name.  Post
    // scalars and locals first so every receipt is still available while
    // those fields are derived.
    ereturn matrix stayer_hybrid_plugin = `plugin'
    ereturn matrix stayer_hybrid_correction = `correction'
    ereturn matrix stayer_hybrid_kss = `kss'
    ereturn matrix stayer_hybrid_results = `raw'
    ereturn matrix stayer_hybrid_decomposition = `decomposition'
    ereturn matrix stayer_hybrid_correction_source = `source'
    ereturn matrix stayer_hybrid_sample_accounting = `accounting'
    ereturn matrix rust_stayer_augmentation_receipt = `aug'
    ereturn matrix rust_stayer_exact_memory_receipt = `memory'
    if "`nodisplay'"=="" _vckss_display
end
