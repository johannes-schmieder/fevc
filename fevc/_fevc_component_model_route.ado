program define _fevc_component_model_route, rclass
    version 18.0
    args inference inferencemodel project projecteffect projectweight ///
        inferencesimulations inferencebins inferenceseed backend rng  ///
        algorithm engine preconditioner batch stayers deletionid     ///
        targetweight deletion nuisance weighted inferencegramprobes

    local gram_supplied = (strtrim(`"`inferencegramprobes'"') != "")
    if `gram_supplied' {
        capture confirm integer number `inferencegramprobes'
        if _rc | !inrange(real(`"`inferencegramprobes'"'),512,2147483647) {
            quietly _vckss_post_failure "INVALID_INFERENCE_TUNING" ///
                "inferencegramprobes() must be an integer in [512,2147483647]."
            exit 198
        }
    }
    else local inferencegramprobes = 2048

    local inference_supplied = (strtrim(`"`inference'"') != "")
    if !`inference_supplied' local inference none
    local inference = lower(strtrim(`"`inference'"'))
    if !inlist("`inference'", "none", "highrank", "q1") {
        quietly _vckss_post_failure "INVALID_INFERENCE"          ///
            "inference() must be none, highrank, or q1."
        di as error "inference() must be none, highrank, or q1"
        exit 198
    }
    local project_supplied = (strtrim(`"`project'"') != "")
    local projecteffect_supplied = (strtrim(`"`projecteffect'"') != "")
    local projectweight_supplied = (strtrim(`"`projectweight'"') != "")
    if !`projectweight_supplied' local projectweight frequency
    local projectweight = lower(strtrim(`"`projectweight'"'))
    if !inlist("`projectweight'", "frequency", "target") {
        quietly _vckss_post_failure "INVALID_PROJECTION_WEIGHT"  ///
            "projectweight() must be frequency or target."
        di as error "projectweight() must be frequency or target"
        exit 198
    }
    if `project_supplied' {
        local projecteffect = lower(strtrim(`"`projecteffect'"'))
        if !inlist("`projecteffect'", "worker", "firm") {
            quietly _vckss_post_failure "INVALID_PROJECTION_EFFECT" ///
                "projecteffect() must be worker or firm when project() is supplied."
            di as error "project() requires projecteffect(worker) or projecteffect(firm)"
            exit 198
        }
    }
    else if `projecteffect_supplied' | `projectweight_supplied' {
        quietly _vckss_post_failure "PROJECTION_OPTIONS_INCOMPLETE" ///
            "projecteffect() and projectweight() require project()."
        di as error "projecteffect() and projectweight() require project()"
        exit 198
    }
    local inference_requested = ("`inference'" != "none" | `project_supplied')
    if "`inference'" != "none" & (`inferencesimulations' < 100 |  ///
        `inferencebins' < 4 | `inferenceseed' < 1 |                ///
        `inferenceseed' > 2147483629) {
        quietly _vckss_post_failure "INVALID_INFERENCE_TUNING"   ///
            "Inference requires at least 100 simulations, at least four bins, and a seed in [1,2147483629]."
        di as error "invalid inference simulations, bins, or seed"
        exit 198
    }

    local backend_supplied = (strtrim(`"`backend'"') != "")
    local rng_supplied = (strtrim(`"`rng'"') != "")
    local algorithm_supplied = (strtrim(`"`algorithm'"') != "")
    local engine_supplied = (strtrim(`"`engine'"') != "")
    local preconditioner_supplied = (strtrim(`"`preconditioner'"') != "")
    local batch_supplied = (strtrim(`"`batch'"') != "")
    local stayers_supplied = (strtrim(`"`stayers'"') != "")
    local deletionid_supplied = (strtrim(`"`deletionid'"') != "")
    local targetweight_supplied = (strtrim(`"`targetweight'"') != "")
    if !`backend_supplied' local backend_requested auto
    else local backend_requested = lower(strtrim(`"`backend'"'))
    if !`rng_supplied' local rng_requested auto
    else local rng_requested = lower(strtrim(`"`rng'"'))

    local supplied = (strtrim(`"`inferencemodel'"')!="")
    if !`supplied' local inferencemodel target_lowess
    local inferencemodel = lower(strtrim(`"`inferencemodel'"'))
    if !inlist("`inferencemodel'","target_lowess",               ///
        "structured_common","structured_leverage") {
        quietly _vckss_post_failure "INVALID_INFERENCE_MODEL"   ///
            "inferencemodel() must be target_lowess, structured_common, or structured_leverage."
        exit 198
    }
    if `supplied' & "`inference'"=="none" {
        quietly _vckss_post_failure "INFERENCE_MODEL_WITHOUT_INFERENCE" ///
            "inferencemodel() requires inference(highrank) or inference(q1)."
        exit 198
    }
    local structured = "`inference'"!="none" &                  ///
        inlist("`inferencemodel'","structured_common","structured_leverage")
    if `structured' & `project_supplied' {
        quietly _vckss_post_failure "COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED" ///
            "Structured component inference and project() cannot share one prepared generation."
        exit 498
    }
    if `structured' & lower(strtrim("`deletion'"))=="observation" & strtrim(`"`weighted'"')!="" {
        quietly _vckss_post_failure "STRUCTURED_FREQUENCY_UNSUPPORTED" ///
            "Structured component inference currently requires unweighted, unit-frequency observations."
        di as error "structured component inference does not yet support frequency weights"
        exit 498
    }
    local observation_tuple = lower(strtrim("`deletion'"))=="observation" & ///
        inlist(lower(strtrim("`stayers'")),"","movers") &       ///
        inlist(lower(strtrim("`nuisance'")),"","joint")
    local match_tuple = lower(strtrim("`deletion'"))=="match" &  ///
        lower(strtrim("`stayers'"))=="movers" &                  ///
        lower(strtrim("`nuisance'"))=="fixedoffset" &             ///
        lower(strtrim("`engine'"))=="generic"
    local scalable = `structured' & !`project_supplied' &         ///
        "`backend_requested'"=="rust" & "`rng_requested'"=="counter_v1" & ///
        `algorithm_supplied' & lower(strtrim("`algorithm'"))=="jla" & ///
        (`observation_tuple' | `match_tuple') &                   ///
        `preconditioner_supplied' &                               ///
        inlist(lower(strtrim("`preconditioner'")),"diagonal","cmg") & ///
        inlist(lower(strtrim("`engine'")),"","auto","generic")
    if `structured' & !`scalable' {
        quietly _vckss_post_failure "STRUCTURED_INFERENCE_TUPLE_REQUIRED" ///
            "Structured inference requires explicit Rust/Counter-V1 JLA and diagonal or CMG; use observation deletion with joint nuisance, or explicit deletion(match) nuisance(fixedoffset) stayers(movers) engine(generic)."
        exit 498
    }
    if `gram_supplied' & !`scalable' {
        quietly _vckss_post_failure "INFERENCE_GRAM_TUPLE_REQUIRED" ///
            "inferencegramprobes() requires supported explicit structured Rust component inference."
        exit 498
    }
    if `scalable' {
        capture quietly _fevc_rust_public_call componentversion
        local interface_rc = _rc
        if !`interface_rc' local interface_rc = (r(interface_version)!=4)
        if `interface_rc' {
            quietly _vckss_post_failure "COMPONENT_NATIVE_INTERFACE_REQUIRED" ///
                "Structured inference requires the updated native component interface; reinstall the matching plugin."
            di as error "structured inference requires the updated matching native plugin"
            exit 498
        }
    }
    c_local inferencemodel "`inferencemodel'"
    c_local inferencegramprobes `inferencegramprobes'
    c_local inferencemodel_supplied `supplied'
    c_local scalable_component_requested `scalable'
    c_local inference_supplied `inference_supplied'
    c_local inference "`inference'"
    c_local project_supplied `project_supplied'
    c_local projecteffect_supplied `projecteffect_supplied'
    c_local projectweight_supplied `projectweight_supplied'
    c_local projecteffect "`projecteffect'"
    c_local projectweight "`projectweight'"
    c_local inference_requested `inference_requested'
    c_local backend_supplied `backend_supplied'
    c_local rng_supplied `rng_supplied'
    c_local algorithm_supplied `algorithm_supplied'
    c_local engine_supplied `engine_supplied'
    c_local preconditioner_supplied `preconditioner_supplied'
    c_local batch_supplied `batch_supplied'
    c_local stayers_supplied `stayers_supplied'
    c_local deletionid_supplied `deletionid_supplied'
    c_local targetweight_supplied `targetweight_supplied'
    c_local backend_requested "`backend_requested'"
    c_local rng_requested "`rng_requested'"
end
