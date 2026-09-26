program define fevc__hybrid_sample
    version 18.0
    args touse complete original_stayer worker firm frequency ///
        retained_firm_member stayer_physical_total hybrid_stayer hybrid_touse

    quietly egen byte `retained_firm_member' = max(`touse') ///
        if `complete', by(`firm')
    quietly replace `retained_firm_member' = 0 if !`complete'
    quietly egen double `stayer_physical_total' = total(`frequency') ///
        if `complete', by(`worker')
    // Missing is true in Stata, so masks must be binary on every row.
    // Eligibility comes only from frozen input histories, never graph drops.
    quietly generate byte `hybrid_stayer' = (`complete'==1) & ///
        (`original_stayer'==1) & (`retained_firm_member'==1) & ///
        !missing(`stayer_physical_total') & `stayer_physical_total'>=2
    quietly generate byte `hybrid_touse' = (`complete'==1) & ///
        (`touse'==1 | `hybrid_stayer'==1)
end
