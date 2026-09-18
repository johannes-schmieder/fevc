program define fevc__failure_guidance, rclass
    version 18.0
    args failure_status

    local reason "The requested calculation did not pass a registered validation gate."
    local suggestion "Review the technical detail and the troubleshooting section of the help file before changing the model or sample."

    if inlist("`failure_status'", "INVALID_DEPVAR",              ///
        "INVALID_CONTROLS", "INVALID_INPUT", "NONFINITE_INPUT", ///
        "INVALID_FREQUENCY", "INVALID_TARGET_WEIGHT",           ///
        "INVALID_IDENTIFIER", "INVALID_PROBE_ORDER") {
        local reason "One or more submitted variables, identifiers, weights, or controls do not satisfy the command's data contract."
        local suggestion "Check variable types, missing and nonfinite values, positive-integer frequency weights, nonnegative target mass, and complete identifiers on the requested sample."
    }
    else if inlist("`failure_status'", "INVALID_TUNING",         ///
        "INVALID_TOLERANCE", "INVALID_NUISANCE",                ///
        "INVALID_STAYER_CONVENTION", "INVALID_PRECONDITIONER",  ///
        "INVALID_MEMORY_ENVELOPE", "INVALID_WALL_ENVELOPE",    ///
        "INVALID_ENGINE", "INVALID_BACKEND") |                  ///
        "`failure_status'" == "INVALID_RNG" {
        local reason "A command option is outside its supported range or names an unsupported mode."
        local suggestion "Check the option spelling and documented range in help fevc; do not loosen numerical tolerances to force an estimate through."
    }
    else if "`failure_status'" == "INVALID_INFERENCE_TUNING" {
        local reason "An inference precision, simulation or seed option is outside its supported range."
        local suggestion "Use integer inferencegramprobes() from 512 through 2147483647 (default 2048) for structured Rust inference, and check the documented simulation and seed ranges."
    }
    else if "`failure_status'" == "INFERENCE_GRAM_TUPLE_REQUIRED" {
        local reason "Gram precision applies only to the supported structured Rust component-inference calculation."
        local suggestion "Use the documented explicit structured Rust tuple, or omit inferencegramprobes() for point estimation, exact Mata inference or project()."
    }
    else if inlist("`failure_status'",                         ///
        "RUST_RNG_BACKEND_MISMATCH",                           ///
        "COUNTER_RNG_BACKEND_MISMATCH") {
        local reason "The explicitly requested backend and RNG contracts select incompatible runtimes."
        local suggestion "Use rng(auto), pair backend(rust) with rng(counter_v1), or pair backend(mata) with rng(stata)."
    }
    else if "`failure_status'" == "RUST_BACKEND_UNQUALIFIED" {
        local reason "The loaded Rust runtime did not satisfy the versioned transport or request-capability contract."
        local suggestion "Restart Stata and reinstall one complete qualified build; use backend(mata) only as a new explicit request, never as post-preparation fallback."
    }
    else if "`failure_status'" == "RUST_OPTION_UNSUPPORTED" {
        local reason "The effective request is outside the capability surface admitted by the loaded Rust runtime."
        local suggestion "Use backend(mata), or choose a documented Rust tuple; do not weaken the model, validation gates, or RNG contract merely to enter a native route."
    }
    else if inlist("`failure_status'", "UNSUPPORTED_ALGORITHM", ///
        "UNSUPPORTED_DELETION", "UNSUPPORTED_DELETION_ID",      ///
        "UNSUPPORTED_STAYER_CONVENTION", "CROSS_COORDINATE_MATCH", ///
        "MATCH_INPUT_MISSING") {
        local reason "The requested deletion or sample definition is internally inconsistent."
        local suggestion "Verify that every match ID stays within one worker-firm coordinate and that all frozen match inputs are complete; change deletion assumptions only when scientifically justified."
    }
    else if inlist("`failure_status'", "NO_USABLE_OBSERVATIONS", ///
        "NO_MOVER_SAMPLE", "NO_LEAVEOUT_COMPONENT",             ///
        "AMBIGUOUS_LARGEST_COMPONENT", "INVALID_GRAPH_INPUT",   ///
        "GRAPH_ITERATION_FAILED", "GRAPH_BRIDGE_CERTIFICATE_FAILED") {
        local reason "The requested rows do not yield a uniquely selected, leave-out-connected target graph."
        local suggestion "Check the if/in restriction, worker and firm IDs, match IDs, and mover histories; inspect whether a meaningful leave-out-connected component exists before changing the sample rule."
    }
    else if "`failure_status'" == "AMBIGUOUS_CONTROL_BASIS" {
        local reason "Numerical error prevents certification of a coordinate-invariant control basis; this does not demonstrate model singularity."
        local suggestion "Inspect the diagnostic cause, control scales and near dependencies. Consider a scientifically equivalent, better-scaled control representation; keep the sample, rank and residual tolerances unchanged."
    }
    else if inlist("`failure_status'", "SINGULAR_INFORMATION",  ///
        "SINGULAR_NUISANCE_BLOCK", "NONESTIMABLE_DELETION",     ///
        "UNVERIFIED_DELETION_RANK", ///
        "INVERSE_FORWARD_ERROR_FAILED", "INVERSE_RESIDUAL_FAILED", ///
        "BLOCK_INVERSE_FAILED", "CONTROL_SCHUR_RESIDUAL_FAILED") {
        local reason "The full model, nuisance block, or at least one declared deletion could not be certified as identified and numerically stable."
        local suggestion "Inspect collinear or weakly supported controls and thin matches. For UNVERIFIED_DELETION_RANK, try algorithm(exact) on a feasible design or revise the controls; do not add a hidden ridge."
    }
    else if inlist("`failure_status'", "EXACT_SIZE_LIMIT",       ///
        "BLOCK_SIZE_LIMIT") {
        local reason "The deterministic exact calculation exceeds a declared dense dimension or deletion-block safety limit."
        local suggestion "Use algorithm(auto) or algorithm(jla) for a large identified design; increase a safety limit only after confirming the required allocation is appropriate."
    }
    else if inlist("`failure_status'", "PHYSICAL_TOTAL_LIMIT",  ///
        "PHYSICAL_COPY_LIMIT", "RESOURCE_ADMISSION_FAILED",     ///
        "GENERIC_RESOURCE_ADMISSION_FAILED", "SOLVER_MEMORY_LIMIT", ///
        "RAW_MEMORY_MEASUREMENT_FAILED") {
        local reason "The requested literal-copy or direct-allocation workload exceeds a certified numerical or memory boundary."
        local suggestion "Check frequency weights and available RAM. Reduce batch width where relevant or raise memory_gib()/physical_limit() only when the machine can safely support the resulting allocation."
    }
    else if inlist("`failure_status'", "PCG_BREAKDOWN",          ///
        "PCG_NONCONVERGENCE", "SOLVER_RESIDUAL_FAILED",         ///
        "FORCED_CMG_FAILED", "JLA_CONSTRAINT_FAILED",          ///
        "JLA_MOMENT_FAILED", "JLA_INVERSE_FAILED") {
        local reason "The randomized or iterative calculation failed convergence, curvature, moment, or complete-equation residual certification."
        local suggestion "Check graph connectivity and scaling, allow more maxiter(), and use preconditioner(auto) or a supported alternative; do not relax tolerance merely to accept a failed residual."
    }
    else if strpos("`failure_status'", "FASTPATH_") == 1 {
        local reason "The forced compressed engine does not represent this design exactly."
        local suggestion "Use engine(auto) or engine(generic) for controls, observation deletion, cross-cell blocks, or unsupported target structure; the command will not reinterpret the requested estimand."
    }
    else if inlist("`failure_status'", "RNG_RUNTIME_UNREGISTERED", ///
        "RNG_SETUP_FAILED", "RNG_GUARD_FAILED", "RNG_RESTORE_FAILED") {
        local reason "The randomized estimator could not establish or restore its registered Stata RNG contract."
        local suggestion "Use a supported Stata 18 or 19 runtime for JLA, or use algorithm(exact) when feasible; restart Stata if caller RNG restoration failed."
    }
    else if strpos("`failure_status'", "STALE_") == 1 |         ///
        strpos("`failure_status'", "RUNTIME") > 0 |              ///
        strpos("`failure_status'", "NOT_FOUND") > 0 {
        local reason "The installed ado and Mata runtime files are missing, stale, or from incompatible package builds."
        local suggestion "Run discard or restart Stata, then reinstall one complete fevc build and confirm that all package files resolve from the same adopath location."
    }
    else if strpos("`failure_status'", "NONFINITE_") == 1 |     ///
        inlist("`failure_status'", "TARGET_IDENTITY_FAILED",     ///
        "SOLVER_DIAGNOSTICS_INVALID") {
        local reason "A fitted value, correction, final target, or accounting diagnostic became nonfinite or numerically inconsistent."
        local suggestion "Inspect extreme outcomes, controls, and weights and consider economically neutral rescaling; report the technical status if finite, well-scaled inputs still reproduce the failure."
    }
    else if inlist("`failure_status'", "DATA_RESTORATION_FAILED", ///
        "SCALE_STATE_RELEASE_FAILED", "PHASE_MARKER_FAILED",     ///
        "RESOURCE_RECEIPT_FAILED", "TIMER_RESERVATION_FAILED") {
        local reason "The command could not safely complete its caller-state or diagnostic lifecycle."
        local suggestion "Restart Stata before continuing and report the technical status with a reproducible example; do not rely on partial results from this call."
    }
    else if strpos("`failure_status'", "STAYER_HYBRID_") == 1 {
        local reason "The mixed mover-match/stayer-observation convention is unavailable for this request tuple."
        local suggestion "Use deletion(match) with exact or generic JLA, or request stayers(movers) for the mover-only target."
    }

    return local reason `"`reason'"'
    return local suggestion `"`suggestion'"'
end
