*! Validates component-inference and projection requests and checks backend
*! capabilities. Normalized options and route flags are returned in caller locals.

program define fevc__component_model_route, rclass
    version 18.0
    args inference inferencemodel project projecteffect projectweight ///
        inferencesimulations inferencebins inferenceseed backend rng  ///
        algorithm engine preconditioner batch stayers deletionid     ///
        targetweight deletion nuisance weighted inferencegramprobes ///
        inferencedraws hascontrols
    if "`inferencedraws'"=="" local inferencedraws = 8
    if "`hascontrols'"=="" local hascontrols = 1

    local gram_supplied = (strtrim(`"`inferencegramprobes'"') != "")
    if `gram_supplied' {
        capture confirm integer number `inferencegramprobes'
        if _rc | !inrange(real(`"`inferencegramprobes'"'),512,2147483647) {
            quietly _fevc_component_route_failure "INVALID_INFERENCE_TUNING" ///
                "inferencegramprobes() must be an integer in [512,2147483647]."
            exit 198
        }
    }
    else local inferencegramprobes = 2048

    // Standard errors are requested by default. inference(none) turns them
    // off; after a failed attempt the caller suppresses them for its retry.
    local inference_supplied = (strtrim(`"`inference'"') != "")
    if !`inference_supplied' {
        local inference auto
        // Suites written for the earlier point-only default keep it by
        // setting FEVC_DEFAULT_INFERENCE=none in the environment or as a
        // global macro. Nothing else reads this setting.
        if lower(strtrim(`"`: environment FEVC_DEFAULT_INFERENCE'"'))=="none" | ///
            lower(strtrim("$FEVC_DEFAULT_INFERENCE"))=="none" local inference none
    }
    local inference = lower(strtrim(`"`inference'"'))
    if !inlist("`inference'", "none", "auto", "highrank", "q1") {
        quietly _fevc_component_route_failure "INVALID_INFERENCE"          ///
            "inference() must be none, auto, highrank, or q1."
        di as error "inference() must be none, auto, highrank, or q1"
        exit 198
    }
    if !inrange(`inferencedraws',2,4096) {
        quietly _fevc_component_route_failure "INVALID_INFERENCE_TUNING"   ///
            "inferencedraws() must be an integer in [2,4096]."
        di as error "inferencedraws() must be an integer in [2,4096]"
        exit 198
    }
    // inference(auto), the default, asks for leave-out residual standard
    // errors whenever this request can carry them and otherwise for point
    // estimates only. It never changes the backend, algorithm, or solver
    // route the user selected.
    local se_auto = ("`inference'"=="auto")
    local se_auto_reason ""
    local se_suppressed = (`se_auto' & "$VCKSS_SE_SUPPRESS"=="1")
    local residual_model = lower(strtrim(`"`inferencemodel'"'))
    if `se_auto' & !inlist("`residual_model'","","leaveout_residual") {
        quietly _fevc_component_route_failure "INFERENCE_MODEL_WITHOUT_INFERENCE" ///
            "inference(auto) selects inferencemodel(leaveout_residual); other models need inference(highrank) or inference(q1)."
        exit 198
    }
    if `se_auto' {
        local inferencemodel leaveout_residual
        local inference highrank
    }
    local project_supplied = (strtrim(`"`project'"') != "")
    local projecteffect_supplied = (strtrim(`"`projecteffect'"') != "")
    local projectweight_supplied = (strtrim(`"`projectweight'"') != "")
    if !`projectweight_supplied' local projectweight frequency
    local projectweight = lower(strtrim(`"`projectweight'"'))
    if !inlist("`projectweight'", "frequency", "target") {
        quietly _fevc_component_route_failure "INVALID_PROJECTION_WEIGHT"  ///
            "projectweight() must be frequency or target."
        di as error "projectweight() must be frequency or target"
        exit 198
    }
    if `project_supplied' {
        local projecteffect = lower(strtrim(`"`projecteffect'"'))
        if !inlist("`projecteffect'", "worker", "firm") {
            quietly _fevc_component_route_failure "INVALID_PROJECTION_EFFECT" ///
                "projecteffect() must be worker or firm when project() is supplied."
            di as error "project() requires projecteffect(worker) or projecteffect(firm)"
            exit 198
        }
    }
    else if `projecteffect_supplied' | `projectweight_supplied' {
        quietly _fevc_component_route_failure "PROJECTION_OPTIONS_INCOMPLETE" ///
            "projecteffect() and projectweight() require project()."
        di as error "projecteffect() and projectweight() require project()"
        exit 198
    }
    local inference_requested = ("`inference'" != "none" | `project_supplied')
    if "`inference'" != "none" & (`inferencesimulations' < 100 |  ///
        `inferencebins' < 4 | `inferenceseed' < 1 |                ///
        `inferenceseed' > 2147483629) {
        quietly _fevc_component_route_failure "INVALID_INFERENCE_TUNING"   ///
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
        "structured_common","structured_leverage","leaveout_residual") {
        quietly _fevc_component_route_failure "INVALID_INFERENCE_MODEL"   ///
            "inferencemodel() must be leaveout_residual, target_lowess, structured_common, or structured_leverage."
        exit 198
    }
    if `supplied' & "`inference'"=="none" {
        quietly _fevc_component_route_failure "INFERENCE_MODEL_WITHOUT_INFERENCE" ///
            "inferencemodel() requires inference(highrank) or inference(q1)."
        exit 198
    }
    local structured = "`inference'"!="none" &                  ///
        inlist("`inferencemodel'","structured_common","structured_leverage")
    local residual = "`inference'"!="none" & "`inferencemodel'"=="leaveout_residual"
    if `residual' & "`inference'"=="q1" {
        quietly _fevc_component_route_failure "LEAVEOUT_RESIDUAL_Q1_UNSUPPORTED" ///
            "inferencemodel(leaveout_residual) uses the normal reference; specify inference(highrank) or inference(auto)."
        exit 498
    }
    if `residual' & `gram_supplied' {
        quietly _fevc_component_route_failure "INFERENCE_GRAM_TUPLE_REQUIRED" ///
            "inferencegramprobes() does not apply to inferencemodel(leaveout_residual); use inferencedraws()."
        exit 498
    }
    if `structured' & `project_supplied' {
        quietly _fevc_component_route_failure "COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED" ///
            "Structured component inference and project() cannot share one prepared generation."
        exit 498
    }
    if `structured' & lower(strtrim("`deletion'"))=="observation" & strtrim(`"`weighted'"')!="" {
        quietly _fevc_component_route_failure "STRUCTURED_FREQUENCY_UNSUPPORTED" ///
            "Structured component inference currently requires unweighted, unit-frequency observations."
        di as error "structured component inference does not yet support frequency weights"
        exit 498
    }
    local observation_tuple = lower(strtrim("`deletion'"))=="observation" & ///
        inlist(lower(strtrim("`stayers'")),"","movers","both") & ///
        inlist(lower(strtrim("`nuisance'")),"","joint")
    local match_tuple = lower(strtrim("`deletion'"))=="match" &  ///
        inlist(lower(strtrim("`stayers'")),"","movers","both") &                  ///
        lower(strtrim("`nuisance'"))=="fixedoffset" &             ///
        lower(strtrim("`engine'"))=="generic"
    local exact_structured = `structured' & "`backend_requested'"=="mata" & ///
        inlist("`rng_requested'","auto","stata") & `algorithm_supplied' & ///
        lower(strtrim("`algorithm'"))=="exact" & ///
        lower(strtrim("`deletion'"))=="match" & ///
        lower(strtrim("`nuisance'"))=="fixedoffset"
    local scalable = `structured' & !`project_supplied' &         ///
        "`backend_requested'"=="rust" & "`rng_requested'"=="counter_v1" & ///
        `algorithm_supplied' & lower(strtrim("`algorithm'"))=="jla" & ///
        (`observation_tuple' | `match_tuple') &                   ///
        `preconditioner_supplied' &                               ///
        inlist(lower(strtrim("`preconditioner'")),"diagonal","cmg") & ///
        inlist(lower(strtrim("`engine'")),"","auto","generic")
    // Leave-out residual standard errors. Units are the deletion units of
    // either deletion mode; controls enter through a fixed-offset geometry
    // under either nuisance mode. Observation deletion on the native route
    // keeps controls in the operator and needs unit frequencies.
    local residual_deletion = lower(strtrim("`deletion'"))
    local residual_units_ok =                                             ///
        inlist(lower(strtrim("`stayers'")),"","movers","both") &         ///
        (inlist("`residual_deletion'","","match") |                       ///
            ("`residual_deletion'"=="observation" &                       ///
                inlist(lower(strtrim("`nuisance'")),"","joint")))
    local residual_native = `residual' & !`project_supplied' &            ///
        inlist("`backend_requested'","auto","rust") &                    ///
        inlist("`rng_requested'","auto","counter_v1") &                  ///
        inlist(lower(strtrim("`algorithm'")),"","jla") &                 ///
        inlist(lower(strtrim("`engine'")),"","auto","generic") &         ///
        inlist(lower(strtrim("`preconditioner'")),"","auto","diagonal","cmg") & ///
        `residual_units_ok' &                                             ///
        !("`residual_deletion'"=="observation" & strtrim(`"`weighted'"')!="")
    local residual_mata = `residual' & !`project_supplied' &              ///
        "`backend_requested'"=="mata" &                                  ///
        inlist("`rng_requested'","auto","stata") &                       ///
        inlist(lower(strtrim("`algorithm'")),"","jla","exact") &         ///
        `residual_units_ok'
    local residual_reason ""
    if `residual' & !`residual_native' & !`residual_mata' {
        local residual_reason "this combination of options does not carry them (see help fevc, Standard errors)"
    }
    if `se_suppressed' {
        local residual_native = 0
        local residual_mata = 0
        local residual_reason "$VCKSS_SE_SUPPRESS_REASON"
    }
    if `residual_native' {
        // A plugin that loads but predates the capability cannot carry them.
        // With backend(auto) and no plugin, the Mata runtime will.
        capture quietly fevc__rust_public_call probe
        local residual_rc = _rc
        if !`residual_rc' {
            if r(component_residual_api)!=1 | r(component_mixed_api)!=1 {
                local residual_native = 0
                local residual_reason "the installed native plugin predates them; reinstall the matching plugin"
            }
            else {
                capture quietly fevc__rust_public_call componentversion
                if _rc | r(interface_version)!=4 {
                    local residual_native = 0
                    local residual_reason "the installed native plugin predates them; reinstall the matching plugin"
                }
            }
        }
        else if "`backend_requested'"=="rust" {
            local residual_native = 0
            local residual_reason "the native plugin is not available"
        }
    }
    if `residual' & !`residual_native' & !`residual_mata' {
        if `se_auto' {
            // Point estimates only, exactly as with inference(none).
            local se_auto_reason "`residual_reason'"
            local inference none
            local inferencemodel target_lowess
            local supplied = 0
            local residual = 0
            local inference_requested = `project_supplied'
        }
        else {
            quietly _fevc_component_route_failure "LEAVEOUT_RESIDUAL_TUPLE_REQUIRED" ///
                "Leave-out residual standard errors need Rust/Counter-V1 or Mata with algorithm(jla) (or Mata algorithm(exact)), no project(), and, for native observation deletion, joint nuisance without frequency weights; `residual_reason'."
            di as error "leave-out residual standard errors are not available for this request"
            exit 498
        }
    }
    if `residual' {
        // The request is a scalable component request on the native route
        // and shares the exact mixed-unit posting on the Mata routes. One
        // native probe is one pseudo-noise solve: a fit and three kernel
        // images per draw. No Gram matrix is fitted.
        local scalable = 1
        local exact_structured = 1
        local inferencesimulations = 4*`inferencedraws'
        local inferencegramprobes = 0
        c_local inferencesimulations `inferencesimulations'
    }
    if `structured' & !(`scalable' | `exact_structured') {
        quietly _fevc_component_route_failure "STRUCTURED_INFERENCE_TUPLE_REQUIRED" ///
            "Structured inference requires explicit Rust/Counter-V1 JLA with diagonal or CMG, or explicit Mata/exact match deletion with fixedoffset nuisance. Match inference permits stayers(both) or stayers(movers)."
        exit 498
    }
    if `gram_supplied' & !`scalable' {
        quietly _fevc_component_route_failure "INFERENCE_GRAM_TUPLE_REQUIRED" ///
            "inferencegramprobes() requires supported explicit structured Rust component inference."
        exit 498
    }
    // The structured models are native-only and fail closed without the
    // plugin. Leave-out residual standard errors checked their capability
    // above and may run on the Mata runtime instead.
    if `scalable' & !`residual' & `match_tuple' & lower(strtrim("`stayers'"))!="movers" {
        capture quietly fevc__rust_public_call probe
        local mixed_rc = _rc
        if !`mixed_rc' local mixed_rc = (r(component_mixed_api)!=1)
        if `mixed_rc' {
            quietly _fevc_component_route_failure "MIXED_COMPONENT_NATIVE_REQUIRED" ///
                "Pooled component inference requires the mixed-unit native capability; install the matching plugin."
            exit 498
        }
    }
    if `scalable' & !`residual' {
        capture quietly fevc__rust_public_call componentversion
        local interface_rc = _rc
        if !`interface_rc' local interface_rc = (r(interface_version)!=4)
        if `interface_rc' {
            quietly _fevc_component_route_failure "COMPONENT_NATIVE_INTERFACE_REQUIRED" ///
                "Structured inference requires the updated native component interface; reinstall the matching plugin."
            di as error "structured inference requires the updated matching native plugin"
            exit 498
        }
    }
    // The display and e() report whether default standard errors were
    // attached and, if not, why. They must never cost the point estimate: the
    // public entry point reruns the command without them if this attempt
    // fails, which it recognizes by VCKSS_SE_ATTACHED.
    global VCKSS_SE_AUTO `se_auto'
    global VCKSS_SE_AUTO_REASON "`se_auto_reason'"
    global VCKSS_SE_ATTACHED = (`se_auto' & `residual')
    c_local inferencemodel "`inferencemodel'"
    c_local inferencedraws `inferencedraws'
    c_local se_auto `se_auto'
    c_local se_auto_reason "`se_auto_reason'"
    c_local residual_se_requested `residual'
    c_local inferencegramprobes `inferencegramprobes'
    c_local inferencemodel_supplied `supplied'
    c_local scalable_component_requested `scalable'
    c_local exact_structured_requested `exact_structured'
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

// The caller owns e() and the private failure-posting routine in fevc.ado.
program define _fevc_component_route_failure
    version 18.0
    args status detail
    global VCKSS_INFERENCE_ERROR "`status'"
    global VCKSS_INFERENCE_DETAIL `"`detail'"'
end
