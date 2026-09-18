*! version 0.4.0-alpha.1 13sep2026
program define fevc__rust_cmg_model, rclass
    version 18.0
    args handle controls nuisance probes cmg_rhs
    // Separate additive model accounting from the frozen 46-field FE receipt.
    fevc_rust fullcmgmodelreceipt `handle'
    local logical = `controls'+1+(`controls'>0 & `nuisance'==2)+3*`probes'
    local controlled = cond(`controls'>0,1+2*`probes'*(`nuisance'==1),0)
    if r(struct_size)!=56 | r(schema_version)!=1 | r(generation)!=`handle' | ///
        r(controls_count)!=`controls' | r(nuisance_mode)!=`nuisance' | ///
        r(logical_rhs_count)!=`logical' | r(explicit_options_rhs_count)!=`controls' | ///
        r(controlled_rhs_count)!=`controlled' | ///
        missing(r(control_refinement_rhs_count)) | r(control_refinement_rhs_count)<0 | ///
        r(control_refinement_rhs_count)!=floor(r(control_refinement_rhs_count)) | ///
        `cmg_rhs'!=`logical'+r(control_refinement_rhs_count) {
        di as err "Full-CMG model work does not reconcile with the statistical request"
        exit 498
    }
    tempname model
    matrix `model' = (`controls',`nuisance',`logical',r(explicit_options_rhs_count), ///
        r(controlled_rhs_count),r(control_refinement_rhs_count))
    matrix colnames `model' = controls nuisance logical_rhs strict_rhs controlled_rhs refinement_rhs
    return matrix receipt = `model'
    return local schema "CMG-FULL-MODEL-V1"
end
