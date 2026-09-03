program define _fevc_rust_component_attach, rclass
    version 18.0
    args handle rows resident memorylimit model reference simulations batch ///
        inferenceseed level ranktol receiptout
    local expected_peak = `resident'+4096
    local critical_simulations = max(1000,`simulations')
    capture noisily _fevc_rust_public_call augmentcomponent `handle', model(`model') ///
        reference(`reference') probes(`simulations') batch(`batch') ///
        spectrumprobes(128) spectrumiterations(128) seed(`inferenceseed') ///
        psdtolerance(1e-8) spectrumtolerance(.002) confidence(`=`level'/100') ///
        criticalsimulations(`critical_simulations') observationsperterm(5) ///
        foldseed(`inferenceseed') ranktolerance(`ranktol')             ///
        positivitymultiplier(1e-8)
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') handle(`handle') ///
            phase(component_augmentation)
        exit `failure_rc'
    }
    local model_code = cond("`model'"=="structured_common",1,2)
    local reference_code = ("`reference'"=="q1")
    local ok = r(schema_version)==1 & r(rows)==`rows' &              ///
        r(variance_source)==`model_code' &                           ///
        r(reference_distribution)==`reference_code' &               ///
        r(augmentation_peak_forecast_bytes)==`expected_peak' &      ///
        r(component_persistent_bytes)==0 &                          ///
        r(total_prepared_resident_bytes)==`resident' &              ///
        r(augmentation_peak_forecast_bytes)<=`memorylimit'
    if !`ok' {
        capture noisily _fevc_rust_abort, rc(498) handle(`handle') ///
            phase(component_augmentation)
        exit 498
    }
    tempname receipt
    matrix `receipt' = (r(schema_version),r(rows),r(variance_source), ///
        r(reference_distribution),r(augmentation_peak_forecast_bytes), ///
        r(component_persistent_bytes),r(total_prepared_resident_bytes))
    matrix colnames `receipt' = schema rows model reference          ///
        augmentation_peak persistent prepared
    matrix `receiptout' = `receipt'
    return scalar peak = `expected_peak'
end
