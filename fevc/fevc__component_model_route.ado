*! Validates component-inference and projection requests and checks backend
*! capabilities. Normalized options and route flags are returned in caller locals.

program define fevc__component_model_route, rclass
    version 15.0
    // fevc 0.6.0 computes the standard errors of the components by one
    // method, the repeat-measurement model. Requests for the variance models
    // it replaced, and for their tuning options, stop before every other
    // check with a message that names the change; fevc.ado runs this action
    // first. The projection code still takes simulations and bins as
    // positional constants, returned here.
    if `"`1'"'=="removed" {
        args action inferencemodel inferencesimulations inferencebins ///
            inferencegramprobes
        local model = lower(strtrim(`"`inferencemodel'"'))
        if inlist(`"`model'"',"leaveout_residual","target_lowess",       ///
            "structured_common","structured_leverage") {
            local text "inferencemodel(`model') was removed in fevc 0.6.0; fevc reports repeat-measurement standard errors by default (help fevc, Standard errors)"
            quietly _fevc_component_route_failure "INFERENCE_MODEL_REMOVED" "`text'"
            di as error "`text'"
            exit 198
        }
        if !inlist(`"`model'"',"","repeat_measurement") {
            quietly _fevc_component_route_failure "INVALID_INFERENCE_MODEL" ///
                "inferencemodel() accepts only repeat_measurement, the model fevc uses by default."
            di as error "inferencemodel() accepts only repeat_measurement"
            exit 198
        }
        foreach option in inferencesimulations inferencebins inferencegramprobes {
            if strtrim(`"``option''"')!="" {
                local text "`option'() applied only to the variance models removed in fevc 0.6.0"
                quietly _fevc_component_route_failure "INFERENCE_OPTION_REMOVED" "`text'"
                di as error "`text'"
                exit 198
            }
        }
        c_local inferencesimulations 1000
        c_local inferencebins 1000
        exit
    }
    args inference inferencemodel project projecteffect projectweight ///
        inferencesimulations inferencebins inferenceseed backend rng  ///
        algorithm engine preconditioner batch stayers deletionid     ///
        targetweight deletion nuisance weighted inferencegramprobes ///
        inferencedraws
    if "`inferencedraws'"=="" local inferencedraws = 8
    // inferencemodel() is empty or repeat_measurement here, and
    // inferencegramprobes() was not supplied (the removed action above). The
    // Gram count is a positional constant of the projection code.
    local inferencegramprobes = 2048

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
    // inference(auto), the default, asks for repeat-measurement standard
    // errors with one interval per component (normal or q = 1, chosen from the
    // leading direction) whenever this request can carry them and otherwise
    // for point estimates only; inference(highrank) and inference(q1) ask
    // for them strictly. None of them changes the backend, algorithm, or
    // solver route the user selected. On the native backend the attachment does
    // select the resolved automatic executor in place of the point-only
    // full-CMG one, and the generic engine for a mover-only request without
    // controls; both compute the same estimates.
    local se_auto = ("`inference'"=="auto")
    local se_auto_reason ""
    local se_suppressed = (`se_auto' & "$VCKSS_SE_SUPPRESS"=="1")
    local supplied = (strtrim(`"`inferencemodel'"') != "")
    // The leading direction of every component is found in every mode, so
    // that its share of the estimated sampling variance is always reported.
    // inference(auto) then reports, component by component, the normal
    // interval or the q = 1 interval; inference(q1) reports the q = 1 interval
    // wherever it exists; inference(highrank) reports the normal interval of
    // every component. The pass itself is the high-rank one, and the routes
    // request the search alike for the three modes.
    local se_leading ""
    if inlist("`inference'","auto","highrank","q1") {
        local se_leading `inference'
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
    if "`inference'" != "none" & !`se_auto' &                       ///
        (`inferenceseed' < 1 | `inferenceseed' > 2147483629) {
        quietly _fevc_component_route_failure "INVALID_INFERENCE_TUNING"   ///
            "inferenceseed() must be an integer in [1,2147483629]."
        di as error "inferenceseed() must be an integer in [1,2147483629]"
        exit 198
    }
    // Block projections are a separate estimand with their own covariance;
    // the default leaves the components to point estimates beside them.
    if "`inference'" != "none" & !`se_auto' & `project_supplied' {
        quietly _fevc_component_route_failure "COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED" ///
            "Standard errors of the components and project() cannot be combined in one call; run them in separate calls."
        di as error "standard errors of the components and project() cannot be combined in one call;"
        di as error "run them in separate calls"
        exit 498
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

    if `supplied' & "`inference'"=="none" {
        quietly _fevc_component_route_failure "INFERENCE_MODEL_WITHOUT_INFERENCE" ///
            "inferencemodel() requires inference(auto), inference(highrank) or inference(q1)."
        exit 198
    }
    // A request without component standard errors keeps the internal
    // target_lowess sentinel of the exact projection runtime.
    local inferencemodel = cond("`inference'"=="none","target_lowess","repeat_measurement")
    // Repeat-measurement standard errors are a variant of the leave-out
    // residual pass: same units, solves and routes. Units that share a
    // worker-firm cell receive the sample variance of their outcomes (an
    // eligible stayer's observations; under observation deletion also a
    // mover's match of two or more observations), the remaining units a
    // two-step fitted model.
    local repeat = "`inference'"!="none"
    local residual = `repeat'
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
        ("`backend_requested'"=="mata" |                                 ///
            ("`backend_requested'"=="auto" & "`rng_requested'"=="stata")) & ///
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
            if r(component_residual_api)!=1 | r(component_mixed_api)!=1 | ///
                (`repeat' & r(component_repeat_api)!=1) {
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
            quietly _fevc_component_route_failure "REPEAT_MEASUREMENT_TUPLE_REQUIRED" ///
                "Repeat-measurement standard errors need Rust/Counter-V1 or Mata with algorithm(jla) (or Mata algorithm(exact)) and, for observation deletion, joint nuisance (natively also no frequency weights); `residual_reason'."
            di as error "repeat-measurement standard errors are not available for this request"
            exit 498
        }
    }
    // The request is a scalable component request on the native route and
    // shares the exact mixed-unit posting on the Mata routes. One native
    // probe is one pseudo-noise solve: a fit and three kernel images per
    // draw. No Gram matrix is fitted.
    local scalable = `residual'
    local exact_structured = `residual'
    if `residual' {
        local inferencesimulations = 4*`inferencedraws'
        local inferencegramprobes = 0
        c_local inferencesimulations `inferencesimulations'
    }
    // The display and e() report whether default standard errors were
    // attached and, if not, why. They must never cost the point estimate: the
    // public entry point reruns the command without them if this attempt
    // fails, which it recognizes by VCKSS_SE_ATTACHED.
    global VCKSS_SE_AUTO `se_auto'
    global VCKSS_SE_AUTO_REASON "`se_auto_reason'"
    global VCKSS_SE_ATTACHED = (`se_auto' & `residual')
    // Read by the Mata runtimes and the posting helper; empty unless this
    // request carries repeat-measurement standard errors.
    global VCKSS_SE_MODEL = cond(`residual',"`inferencemodel'","")
    // auto, highrank or q1 when the leading directions are computed; the
    // search is capped at this many linear solves per component.
    global VCKSS_SE_LEADING = cond(`residual' & "`inferencemodel'"=="repeat_measurement", ///
        "`se_leading'","")
    global VCKSS_SE_LEADING_CAP 40
    fevc__residual_se_post regime_clear
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
    version 15.0
    args status detail
    global VCKSS_INFERENCE_ERROR "`status'"
    global VCKSS_INFERENCE_DETAIL `"`detail'"'
end
