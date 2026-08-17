*! kss_bc production solver routing adapter
*! version 0.2.0-dev 16aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

real scalar kssbc_solver__api_level()
{
    return(21)
}

string scalar kssbc_solver__build_id()
{
    return("kss-bc-solver-api21-routed-component-receipt")
}

real scalar kssbc_solver__pilot_api()
{
    return(1)
}

struct kssbc_route_result
{
    struct kssbc_result scalar estimator
    string scalar status
    string scalar message
    string scalar route
    string scalar reason
    string scalar fallback_status
    string scalar fallback_message
    real rowvector diagnostics
    real matrix pilot_diagnostics
    string rowvector pilot_status
    string rowvector pilot_failure_reason
}

struct kssbc_solver_pilot_evidence
{
    real matrix diagnostics
    string rowvector status
    string rowvector failure_reason
}

// The command configures this gate from the registered whole-command
// resource forecast.  Route setup then replaces the provisional hierarchy
// allocation with the accepted solver's complete base/hierarchy peak.  The
// gate is deliberately solver-owned so both the row-resident and compressed
// callback routes stop before estimator RNG without coupling the solver to
// either estimator implementation.
struct kssbc_solver_resource_gate
{
    real scalar active
    real scalar applied
    string scalar route
    string scalar status
    string scalar message
    string scalar peak_phase
    real scalar non_solver_numerical_bytes
    real scalar selection_peak_bytes
    real scalar transition_peak_bytes
    real scalar numerical_peak_bytes
    real scalar restoration_peak_bytes
    real scalar peak_bytes
    real scalar memory_headroom_fraction
    real scalar memory_admission_bytes
    real scalar wall_forecast_upper_seconds
    real scalar wall_headroom_fraction
    real scalar wall_admission_seconds
    real scalar hard_memory_bytes
    real scalar hard_wall_seconds
    real scalar memory_admitted
    real scalar wall_admitted
    real scalar admitted
    real scalar routed_solver_peak_bytes
}

struct kssbc_solver_resource_gate scalar kssbc_solver__resource_empty()
{
    struct kssbc_solver_resource_gate scalar out

    out.active = 0
    out.applied = 0
    out.route = ""
    out.status = "NOT_CONFIGURED"
    out.message = "whole-command resource gate is not configured"
    out.peak_phase = ""
    out.non_solver_numerical_bytes = .
    out.selection_peak_bytes = .
    out.transition_peak_bytes = .
    out.numerical_peak_bytes = .
    out.restoration_peak_bytes = .
    out.peak_bytes = .
    out.memory_headroom_fraction = 0.30
    out.memory_admission_bytes = .
    out.wall_forecast_upper_seconds = .
    out.wall_headroom_fraction = 0.50
    out.wall_admission_seconds = .
    out.hard_memory_bytes = .
    out.hard_wall_seconds = .
    out.memory_admitted = 0
    out.wall_admitted = 0
    out.admitted = 0
    out.routed_solver_peak_bytes = .
    return(out)
}

void kssbc_solver__resource_clear()
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE

    KSSBC_SOLVER_RESOURCE_GATE = kssbc_solver__resource_empty()
}

real scalar kssbc_solver__resource_config(
    string scalar route,
    real scalar non_solver_numerical_bytes,
    real scalar selection_peak_bytes,
    real scalar transition_peak_bytes,
    real scalar restoration_peak_bytes,
    real scalar wall_forecast_upper_seconds,
    real scalar hard_memory_bytes,
    real scalar hard_wall_seconds)
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE

    kssbc_solver__resource_clear()
    if (!(route == "compressed" | route == "generic") |
        missing((non_solver_numerical_bytes,selection_peak_bytes,
                 transition_peak_bytes,restoration_peak_bytes,
                 wall_forecast_upper_seconds,hard_memory_bytes,
                 hard_wall_seconds)) |
        min((non_solver_numerical_bytes,selection_peak_bytes,
             transition_peak_bytes,restoration_peak_bytes,
             wall_forecast_upper_seconds,hard_memory_bytes,
             hard_wall_seconds)) <= 0 |
        hard_memory_bytes > 56*1024^3 |
        hard_wall_seconds > 12*60*60) return(198)

    KSSBC_SOLVER_RESOURCE_GATE.active = 1
    KSSBC_SOLVER_RESOURCE_GATE.route = route
    KSSBC_SOLVER_RESOURCE_GATE.status = "CONFIGURED"
    KSSBC_SOLVER_RESOURCE_GATE.message =
        "whole-command resource gate awaits the accepted solver route"
    KSSBC_SOLVER_RESOURCE_GATE.non_solver_numerical_bytes =
        non_solver_numerical_bytes
    KSSBC_SOLVER_RESOURCE_GATE.selection_peak_bytes =
        selection_peak_bytes
    KSSBC_SOLVER_RESOURCE_GATE.transition_peak_bytes =
        transition_peak_bytes
    KSSBC_SOLVER_RESOURCE_GATE.restoration_peak_bytes =
        restoration_peak_bytes
    KSSBC_SOLVER_RESOURCE_GATE.wall_forecast_upper_seconds =
        wall_forecast_upper_seconds
    KSSBC_SOLVER_RESOURCE_GATE.hard_memory_bytes = hard_memory_bytes
    KSSBC_SOLVER_RESOURCE_GATE.hard_wall_seconds = hard_wall_seconds
    return(0)
}

real scalar kssbc_solver__resource_active()
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE

    return(KSSBC_SOLVER_RESOURCE_GATE.active)
}

real scalar kssbc_solver__resource_budget(
    real scalar memory_envelope_bytes)
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE
    real scalar routed_budget

    if (missing(memory_envelope_bytes) | memory_envelope_bytes <= 0) return(.)
    if (!KSSBC_SOLVER_RESOURCE_GATE.active) {
        return(0.65*memory_envelope_bytes)
    }
    routed_budget = floor(
        KSSBC_SOLVER_RESOURCE_GATE.hard_memory_bytes/1.30)-
        KSSBC_SOLVER_RESOURCE_GATE.non_solver_numerical_bytes
    if (missing(routed_budget) | routed_budget <= 0) return(.)
    return(min((memory_envelope_bytes,routed_budget)))
}

// Return one only when the accepted route passes the final whole-command
// envelope.  The solver invokes this after deterministic route setup and
// pilots, but before either estimator callback can initialize probe RNG.
real scalar kssbc_solver__resource_apply(
    real scalar routed_solver_peak_bytes)
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE
    real rowvector peaks

    if (!KSSBC_SOLVER_RESOURCE_GATE.active |
        KSSBC_SOLVER_RESOURCE_GATE.applied |
        missing(routed_solver_peak_bytes) |
        routed_solver_peak_bytes <= 0) {
        KSSBC_SOLVER_RESOURCE_GATE.status =
            "INVALID_ROUTE_RESOURCE_FORECAST"
        KSSBC_SOLVER_RESOURCE_GATE.message =
            "routed solver resource forecast is invalid"
        KSSBC_SOLVER_RESOURCE_GATE.applied = 1
        return(0)
    }

    KSSBC_SOLVER_RESOURCE_GATE.routed_solver_peak_bytes =
        routed_solver_peak_bytes
    KSSBC_SOLVER_RESOURCE_GATE.numerical_peak_bytes =
        KSSBC_SOLVER_RESOURCE_GATE.non_solver_numerical_bytes+
        routed_solver_peak_bytes
    peaks = (
        KSSBC_SOLVER_RESOURCE_GATE.selection_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.transition_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.numerical_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.restoration_peak_bytes)
    KSSBC_SOLVER_RESOURCE_GATE.peak_bytes = max(peaks)
    if (KSSBC_SOLVER_RESOURCE_GATE.peak_bytes == peaks[1]) {
        KSSBC_SOLVER_RESOURCE_GATE.peak_phase = "selection"
    }
    else if (KSSBC_SOLVER_RESOURCE_GATE.peak_bytes == peaks[2]) {
        KSSBC_SOLVER_RESOURCE_GATE.peak_phase = "transition"
    }
    else if (KSSBC_SOLVER_RESOURCE_GATE.peak_bytes == peaks[3]) {
        KSSBC_SOLVER_RESOURCE_GATE.peak_phase = "numerical"
    }
    else KSSBC_SOLVER_RESOURCE_GATE.peak_phase = "restoration"

    KSSBC_SOLVER_RESOURCE_GATE.memory_admission_bytes =
        ceil(1.30*KSSBC_SOLVER_RESOURCE_GATE.peak_bytes)
    KSSBC_SOLVER_RESOURCE_GATE.wall_admission_seconds =
        ceil(1.50*
            KSSBC_SOLVER_RESOURCE_GATE.wall_forecast_upper_seconds)
    KSSBC_SOLVER_RESOURCE_GATE.memory_admitted =
        KSSBC_SOLVER_RESOURCE_GATE.memory_admission_bytes <=
            KSSBC_SOLVER_RESOURCE_GATE.hard_memory_bytes
    KSSBC_SOLVER_RESOURCE_GATE.wall_admitted =
        KSSBC_SOLVER_RESOURCE_GATE.wall_admission_seconds <=
            KSSBC_SOLVER_RESOURCE_GATE.hard_wall_seconds
    KSSBC_SOLVER_RESOURCE_GATE.admitted =
        KSSBC_SOLVER_RESOURCE_GATE.memory_admitted &
        KSSBC_SOLVER_RESOURCE_GATE.wall_admitted
    KSSBC_SOLVER_RESOURCE_GATE.applied = 1
    if (KSSBC_SOLVER_RESOURCE_GATE.admitted) {
        KSSBC_SOLVER_RESOURCE_GATE.status = "ADMITTED"
        KSSBC_SOLVER_RESOURCE_GATE.message =
            "actual routed solver peak passes final pre-RNG admission"
        return(1)
    }
    if (KSSBC_SOLVER_RESOURCE_GATE.route == "compressed") {
        KSSBC_SOLVER_RESOURCE_GATE.status = "RESOURCE_ADMISSION_FAILED"
        KSSBC_SOLVER_RESOURCE_GATE.message =
            "actual compressed route exceeds final memory or wall admission"
    }
    else {
        KSSBC_SOLVER_RESOURCE_GATE.status =
            "GENERIC_RESOURCE_ADMISSION_FAILED"
        KSSBC_SOLVER_RESOURCE_GATE.message =
            "actual generic route exceeds final memory or wall admission"
    }
    return(0)
}

string scalar kssbc_solver__resource_status()
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE

    return(KSSBC_SOLVER_RESOURCE_GATE.status)
}

string scalar kssbc_solver__resource_message()
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE

    return(KSSBC_SOLVER_RESOURCE_GATE.message)
}

real scalar kssbc_solver__target_id_resid(real matrix values)
{
    real colvector residual
    real scalar scale

    if (rows(values) == 0 | cols(values) != 4 |
        hasmissing(values)) return(.)
    residual = values[.,4]-values[.,1]-values[.,2]-2:*values[.,3]
    scale = max((1,max(abs(values))))
    return(max(abs(residual))/scale)
}

real scalar kssbc_solver__rhs_resid_max(real matrix diagnostics)
{
    if (rows(diagnostics) < 1 | cols(diagnostics) < 5 |
        hasmissing(diagnostics[.,5]) |
        min(diagnostics[.,5]) < 0) return(.)
    return(max(diagnostics[.,5]))
}

real scalar kssbc_solver__roundoff_gate()
{
    return(4096*2.2204460492503131e-16)
}

real rowvector kssbc_solver__resource_vector()
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE

    if (!KSSBC_SOLVER_RESOURCE_GATE.applied) return(J(1,15,.))
    return((
        KSSBC_SOLVER_RESOURCE_GATE.selection_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.transition_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.numerical_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.restoration_peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.peak_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.memory_headroom_fraction,
        KSSBC_SOLVER_RESOURCE_GATE.memory_admission_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.wall_forecast_upper_seconds,
        KSSBC_SOLVER_RESOURCE_GATE.wall_headroom_fraction,
        KSSBC_SOLVER_RESOURCE_GATE.wall_admission_seconds,
        KSSBC_SOLVER_RESOURCE_GATE.hard_memory_bytes,
        KSSBC_SOLVER_RESOURCE_GATE.hard_wall_seconds,
        KSSBC_SOLVER_RESOURCE_GATE.memory_admitted,
        KSSBC_SOLVER_RESOURCE_GATE.wall_admitted,
        KSSBC_SOLVER_RESOURCE_GATE.admitted))
}

void kssbc_solver__stata_res_rcpt(
    string scalar components_name,
    string scalar forecasts_name,
    real scalar forecast_row,
    string scalar status_local,
    string scalar message_local,
    string scalar phase_local,
    string scalar applied_local)
{
    external struct kssbc_solver_resource_gate scalar KSSBC_SOLVER_RESOURCE_GATE
    real matrix components, forecasts
    real rowvector forecast_vector
    real scalar reconstructed_numerical, receipt_scale
    string scalar applied

    applied = "NOT_APPLIED"
    if (KSSBC_SOLVER_RESOURCE_GATE.active &
        KSSBC_SOLVER_RESOURCE_GATE.applied) {
        components = st_matrix(components_name)
        forecasts = st_matrix(forecasts_name)
        if (!missing(forecast_row) & forecast_row == floor(forecast_row) &
            forecast_row >= 1 & forecast_row <= rows(components) &
            forecast_row >= 1 & forecast_row <= rows(forecasts) &
            cols(components) == 10 &
            cols(forecasts) == 15) {
            // Component five is provisional before routing.  Replace it
            // with the complete accepted route allocation; do not add it.
            // Its historical name is retained by the public receipt schema.
            components[forecast_row,5] =
                KSSBC_SOLVER_RESOURCE_GATE.routed_solver_peak_bytes
            reconstructed_numerical = sum(
                components[forecast_row,(2,3,4,5,6,8,9)])+
                (KSSBC_SOLVER_RESOURCE_GATE.route == "generic")*
                components[forecast_row,1]
            forecast_vector = kssbc_solver__resource_vector()
            receipt_scale = max((1,abs(reconstructed_numerical),
                                 abs(forecast_vector[3])))
            if (missing(reconstructed_numerical) |
                abs(reconstructed_numerical-forecast_vector[3]) >
                    64*2.2204460492503131e-16*receipt_scale) {
                KSSBC_SOLVER_RESOURCE_GATE.status =
                    "RESOURCE_RECEIPT_FAILED"
                KSSBC_SOLVER_RESOURCE_GATE.message =
                    "routed components do not reconstruct numerical peak"
            }
            else {
                forecasts[forecast_row,.] = forecast_vector
                st_matrix(components_name,components)
                st_matrix(forecasts_name,forecasts)
                applied = "APPLIED"
            }
        }
        else {
            KSSBC_SOLVER_RESOURCE_GATE.status =
                "RESOURCE_RECEIPT_FAILED"
            KSSBC_SOLVER_RESOURCE_GATE.message =
                "resource receipt matrices are invalid"
        }
    }
    st_local(status_local,KSSBC_SOLVER_RESOURCE_GATE.status)
    st_local(message_local,KSSBC_SOLVER_RESOURCE_GATE.message)
    st_local(phase_local,KSSBC_SOLVER_RESOURCE_GATE.peak_phase)
    st_local(applied_local,applied)
    kssbc_solver__resource_clear()
}

struct kssbc_route_result scalar kssbc_solver__empty_route()
{
    struct kssbc_route_result scalar out

    out.estimator = kssbc__failure(
        "INVALID_SOLVER_BACKEND","solver routing input is invalid")
    out.status = "INVALID_SOLVER_BACKEND"
    out.message = "solver routing input is invalid"
    out.route = ""
    out.reason = ""
    out.fallback_status = ""
    out.fallback_message = ""
    out.diagnostics = J(1,26,.)
    out.pilot_diagnostics = J(8,16,.)
    out.pilot_diagnostics[.,1] = J(4,1,1) \ J(4,1,2)
    out.pilot_diagnostics[.,2] = (1::4) \ (1::4)
    out.pilot_diagnostics[.,3] = J(8,1,0)
    out.pilot_diagnostics[.,4] = J(8,1,0)
    out.pilot_diagnostics[.,9] = J(8,1,0)
    out.pilot_diagnostics[.,10] = J(8,1,0)
    out.pilot_diagnostics[.,12] = J(8,1,0)
    out.pilot_diagnostics[.,13] = J(8,1,0)
    out.pilot_diagnostics[.,14] = J(8,1,0)
    out.pilot_diagnostics[.,16] = J(8,1,1)
    out.pilot_status = J(1,8,"NOT_RUN")
    out.pilot_failure_reason = J(1,8,"not attempted")
    return(out)
}

real colvector kssbc_solver__canonical_keys(
    real colvector identifier,
    real scalar levels)
{
    real scalar level, begin, finish
    real colvector row, sorted, key
    real matrix panel

    if (rows(identifier) == 0 | cols(identifier) != 1 |
        levels < 1 | levels != floor(levels) | hasmissing(identifier) |
        min(identifier) != 1 | max(identifier) != levels) {
        return(J(0,1,.))
    }
    row = 1::rows(identifier)
    sorted = order(identifier,1)
    panel = panelsetup(identifier[sorted],1)
    if (rows(panel) != levels) return(J(0,1,.))
    key = J(levels,1,.)
    for (level=1; level<=levels; level++) {
        begin = panel[level,1]
        finish = panel[level,2]
        key[level] = min(row[sorted[|begin\finish|]])
    }
    if (hasmissing(key) | rows(uniqrows(sort(key,1))) != levels) {
        return(J(0,1,.))
    }
    return(key)
}

real scalar kssbc_solver__planned_rhs(
    real scalar probes,
    real scalar controls,
    string scalar nuisance)
{
    if (missing(probes) | probes < 2 | probes != floor(probes) |
        missing(controls) | controls < 0 | controls != floor(controls) |
        !(nuisance == "joint" | nuisance == "fixedoffset")) return(.)
    return(3*probes+controls+1+(nuisance == "fixedoffset" & controls > 0))
}

real scalar kssbc_solver__cmg_runtime_ok()
{
    return(kssbc_cmg__api_level() == 5 &
        kssbc_cmg__design_label() ==
        "clean-room-cmg-inspired-degree3-hybrid-v5-robust-hierarchy")
}

// The routing scores below are deterministic counts of scalar-equivalent
// work.  The coefficients distinguish row aggregation, V-cycle edge/vertex
// passes, terminal triangular solves, and terminal factor setup.  Measured
// wall times remain diagnostics and never enter a routing decision.
real scalar kssbc_solver__diagonal_work(
    real scalar planned_rhs,
    real scalar iterations,
    real scalar observations,
    real scalar workers,
    real scalar firms)
{
    real scalar schur_work, preconditioner_work

    if (missing((planned_rhs,iterations,observations,workers,firms)) |
        planned_rhs < 1 | iterations < 0 | observations < 1 |
        workers < 1 | firms < 2) return(.)
    schur_work = 8*observations+4*(workers+firms)
    preconditioner_work = 4*firms
    return(planned_rhs*iterations*(schur_work+preconditioner_work))
}

real scalar kssbc_solver__cmg_work(
    real scalar planned_rhs,
    real scalar iterations,
    real scalar observations,
    real scalar workers,
    real scalar firms,
    real scalar hybrid_vertices,
    real scalar hybrid_edges,
    real scalar edge_complexity,
    real scalar vertex_complexity,
    real scalar dense_factor_bytes)
{
    real scalar schur_work, vcycle_work, setup_work, factor_entries
    real scalar terminal_order

    if (missing((planned_rhs,iterations,observations,workers,firms,
                 hybrid_vertices,hybrid_edges,edge_complexity,
                 vertex_complexity,dense_factor_bytes)) |
        planned_rhs < 1 | iterations < 0 | observations < 1 |
        workers < 1 | firms < 2 | hybrid_vertices < 1 |
        hybrid_edges < 0 | edge_complexity <= 0 |
        vertex_complexity <= 0 | dense_factor_bytes < 0) return(.)
    schur_work = 8*observations+4*(workers+firms)
    factor_entries = dense_factor_bytes/8
    terminal_order = max((1,
        floor((sqrt(1+8*factor_entries)-1)/2)))
    vcycle_work = 8*edge_complexity*hybrid_edges+
        12*vertex_complexity*hybrid_vertices+2*factor_entries
    setup_work = 24*(edge_complexity*hybrid_edges+
        vertex_complexity*hybrid_vertices)+
        factor_entries*terminal_order/64
    return(setup_work+
        planned_rhs*iterations*(schur_work+vcycle_work))
}

real scalar kssbc_solver__fallback_work_ok(real scalar work)
{
    // Eight billion row-equivalent scalar actions is the bounded fallback
    // envelope.  It admits genuinely moderate B1 recovery, but prevents a
    // failed large-graph CMG setup from silently launching an implausible
    // repeated-RHS diagonal run.
    return(!missing(work) & work >= 0 & work <= 8e9)
}

real scalar kssbc_solver__cmg_work_wins(real scalar work_ratio)
{
    return(!missing(work_ratio) & work_ratio >= 0 & work_ratio <= 0.80)
}

// Retain all four deterministic pilot outcomes even when routing fails before
// estimator RNG.  The numeric columns are:
//
// backend, pilot, attempted, passed, iterations, complete residual,
// RHS Schur actions, RHS preconditioner applications, backend Schur actions,
// backend preconditioner applications, projected full-estimate work,
// status gate, iteration gate, residual gate, work gate, failure-reason code.
//
// Failure-reason codes are 0 PASS, 1 NOT_RUN, 2 MALFORMED_DIAGNOSTICS,
// 3 BACKEND_STATUS, 4 RHS_STATUS, 5 MISSING_ITERATIONS,
// 6 ITERATION_CAP, 7 MISSING_RESIDUAL, 8 RESIDUAL_GATE,
// 9 MISSING_WORK, and 10 WORK_GATE.  The companion string vectors retain the
// literal per-RHS status and a human-readable gate failure.
// RHS action counts are exact reconstructions for CONVERGED and
// PCG_NONCONVERGENCE states under the current lockstep kernel; other failed
// RHSs retain missing per-RHS counts and the exact backend aggregates.
struct kssbc_solver_pilot_evidence scalar kssbc_solver__pilot_empty(
    real scalar backend_code)
{
    struct kssbc_solver_pilot_evidence scalar out

    out.diagnostics = J(4,16,.)
    out.diagnostics[.,1] = J(4,1,backend_code)
    out.diagnostics[.,2] = 1::4
    out.diagnostics[.,3] = J(4,1,0)
    out.diagnostics[.,4] = J(4,1,0)
    out.diagnostics[.,9] = J(4,1,0)
    out.diagnostics[.,10] = J(4,1,0)
    out.diagnostics[.,12] = J(4,1,0)
    out.diagnostics[.,13] = J(4,1,0)
    out.diagnostics[.,14] = J(4,1,0)
    out.diagnostics[.,16] = J(4,1,1)
    out.status = J(1,4,"NOT_RUN")
    out.failure_reason = J(1,4,"not attempted")
    return(out)
}

struct kssbc_solver_pilot_evidence scalar kssbc_solver__pilot_evidence(
    struct kssbc_solve_result scalar solved,
    real scalar backend_code,
    real scalar planned_rhs,
    real scalar cap,
    real scalar tolerance,
    real scalar observations,
    real scalar workers,
    real scalar firms,
    real scalar hybrid_vertices,
    real scalar hybrid_edges,
    real scalar edge_complexity,
    real scalar vertex_complexity,
    real scalar dense_factor_bytes,
    real scalar exact_inverse)
{
    struct kssbc_solver_pilot_evidence scalar out
    real scalar pilot, valid_shape, residual_gate, iteration, work
    real scalar rhs_schur_actions, rhs_preconditioner_applications
    string scalar rhs_status

    out = kssbc_solver__pilot_empty(backend_code)
    out.diagnostics[.,3] = J(4,1,1)
    residual_gate = max((1e-11,10*tolerance))
    valid_shape = (cols(solved.rhs_status) == 4 &
        cols(solved.rhs_iterations) == 4 &
        cols(solved.rhs_relres) == 4)
    for (pilot=1; pilot<=4; pilot++) {
        out.diagnostics[pilot,9] = solved.schur_actions
        out.diagnostics[pilot,10] = solved.preconditioner_applications
        if (!valid_shape) {
            out.status[pilot] = "MALFORMED_DIAGNOSTICS"
            out.failure_reason[pilot] =
                "pilot result vectors do not each contain four entries"
            out.diagnostics[pilot,16] = 2
            continue
        }
        rhs_status = solved.rhs_status[pilot]
        iteration = solved.rhs_iterations[pilot]
        if (solved.status == "CONVERGED") {
            out.status[pilot] = rhs_status
        }
        else out.status[pilot] = solved.status + "/" + rhs_status
        out.diagnostics[pilot,5] = iteration
        out.diagnostics[pilot,6] = solved.rhs_relres[pilot]
        if (!missing(iteration) & iteration >= 0 &
            iteration == floor(iteration)) {
            if (exact_inverse) {
                rhs_schur_actions = 0
                rhs_preconditioner_applications = (iteration > 0)
            }
            else if (rhs_status == "CONVERGED" |
                rhs_status == "SOLVER_RESIDUAL_FAILED") {
                rhs_schur_actions = iteration+floor(iteration/100)
                rhs_preconditioner_applications = iteration
            }
            else if (rhs_status == "PCG_NONCONVERGENCE") {
                rhs_schur_actions = iteration+floor(iteration/100)
                rhs_preconditioner_applications = iteration+1
            }
            else {
                rhs_schur_actions = .
                rhs_preconditioner_applications = .
            }
            out.diagnostics[pilot,7] = rhs_schur_actions
            out.diagnostics[pilot,8] = rhs_preconditioner_applications
            if (backend_code == 1) {
                work = kssbc_solver__diagonal_work(
                    planned_rhs,iteration,observations,workers,firms)
            }
            else if (backend_code == 2) {
                work = kssbc_solver__cmg_work(
                    planned_rhs,iteration,observations,workers,firms,
                    hybrid_vertices,hybrid_edges,edge_complexity,
                    vertex_complexity,dense_factor_bytes)
            }
            else work = .
            out.diagnostics[pilot,11] = work
        }
        else work = .
        out.diagnostics[pilot,12] =
            (solved.status == "CONVERGED" & rhs_status == "CONVERGED")
        out.diagnostics[pilot,13] =
            (!missing(iteration) & iteration >= 0 &
             iteration == floor(iteration) & iteration <= cap)
        out.diagnostics[pilot,14] =
            (!missing(solved.rhs_relres[pilot]) &
             solved.rhs_relres[pilot] <= residual_gate)
        if (backend_code == 1) {
            out.diagnostics[pilot,15] =
                kssbc_solver__fallback_work_ok(work)
        }
        else out.diagnostics[pilot,15] =
            (!missing(work) & work >= 0)

        if (rhs_status == "SOLVER_RESIDUAL_FAILED" &
            missing(solved.rhs_relres[pilot])) {
            out.failure_reason[pilot] =
                "complete original-system residual is missing"
            out.diagnostics[pilot,16] = 7
        }
        else if (rhs_status == "SOLVER_RESIDUAL_FAILED" &
            solved.rhs_relres[pilot] > residual_gate) {
            out.failure_reason[pilot] =
                "complete original-system residual exceeds the gate"
            out.diagnostics[pilot,16] = 8
        }
        else if (rhs_status != "CONVERGED") {
            out.failure_reason[pilot] = "backend status " + solved.status +
                "; RHS status " + rhs_status
            out.diagnostics[pilot,16] = 4
        }
        else if (solved.status != "CONVERGED") {
            out.failure_reason[pilot] =
                "backend status " + solved.status
            out.diagnostics[pilot,16] = 3
        }
        else if (missing(iteration) | iteration < 0 |
            iteration != floor(iteration)) {
            out.failure_reason[pilot] =
                "RHS iteration diagnostic is missing or invalid"
            out.diagnostics[pilot,16] = 5
        }
        else if (iteration > cap) {
            out.failure_reason[pilot] =
                "RHS iterations exceed the deterministic pilot cap"
            out.diagnostics[pilot,16] = 6
        }
        else if (missing(solved.rhs_relres[pilot])) {
            out.failure_reason[pilot] =
                "complete original-system residual is missing"
            out.diagnostics[pilot,16] = 7
        }
        else if (solved.rhs_relres[pilot] > residual_gate) {
            out.failure_reason[pilot] =
                "complete original-system residual exceeds the gate"
            out.diagnostics[pilot,16] = 8
        }
        else if (missing(work)) {
            out.failure_reason[pilot] =
                "projected deterministic work is unavailable"
            out.diagnostics[pilot,16] = 9
        }
        else if (!out.diagnostics[pilot,15]) {
            out.failure_reason[pilot] =
                "projected deterministic work exceeds the route gate"
            out.diagnostics[pilot,16] = 10
        }
        else {
            out.failure_reason[pilot] = "passed all pilot gates"
            out.diagnostics[pilot,16] = 0
        }
        out.diagnostics[pilot,4] =
            (out.diagnostics[pilot,16] == 0)
    }
    return(out)
}

struct kssbc_solver_cmg_context
{
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy
    struct kssbc_cmg__workspace scalar workspace
    real scalar use_workspace
}

struct kssbc_solver_cmg_context scalar kssbc_solver__cmg_context(
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy,
    real scalar batch_capacity,
    real scalar memory_envelope_bytes,
    real scalar enable_workspace)
{
    struct kssbc_solver_cmg_context scalar out
    real scalar workspace_cap

    out.hierarchy = hierarchy
    out.workspace = kssbc_cmg__empty_workspace()
    out.use_workspace = 0
    if (hierarchy == NULL | (*hierarchy).status != "CONVERGED" |
        missing(batch_capacity) | batch_capacity < 1 |
        batch_capacity != floor(batch_capacity) |
        missing(memory_envelope_bytes) | memory_envelope_bytes <= 0 |
        !(enable_workspace == 0 | enable_workspace == 1)) {
        return(out)
    }
    if (!enable_workspace) return(out)
    workspace_cap = memory_envelope_bytes-
        (*hierarchy).structural_bytes-(*hierarchy).dense_factor_bytes-
        (*hierarchy).options.action_scratch_bytes
    if (missing(workspace_cap) | workspace_cap <= 0) return(out)
    out.workspace = kssbc_cmg__workspace_init(
        *hierarchy,batch_capacity,workspace_cap)
    out.use_workspace = (out.workspace.status == "CONVERGED" &
        out.workspace.predicted_peak_bytes+
            (*hierarchy).dense_factor_bytes <= memory_envelope_bytes)
    return(out)
}

struct kssbc_cmg__apply_result scalar kssbc_solver__cmg_apply_ws(
    struct kssbc_solver_cmg_context scalar context,
    real matrix firm_rhs)
{
    struct kssbc_cmg__apply_result scalar out, core
    struct kssbc_cmg__level scalar fine
    real matrix compatible, injected
    real colvector firm_component

    out = kssbc_cmg__empty_apply_result()
    if (context.hierarchy == NULL |
        (*context.hierarchy).status != "CONVERGED" |
        (*context.hierarchy).n_level < 1 |
        context.workspace.status != "CONVERGED" |
        cols(firm_rhs) < 1 |
        cols(firm_rhs) > context.workspace.batch_capacity |
        hasmissing(firm_rhs)) return(out)
    fine = *(*context.hierarchy).level[1]
    if (rows(firm_rhs) != fine.graph.n_firm) {
        out.status = "INVALID_RHS"
        out.message = "KSS firm RHS has the wrong shape"
        return(out)
    }
    firm_component = fine.component[(1::fine.graph.n_firm)]
    compatible = kssbc_cmg__project_labels(firm_component,firm_rhs)
    if (rows(compatible) != fine.graph.n_firm) return(out)
    injected = J(fine.graph.n_vertex,cols(firm_rhs),0)
    injected[(1::fine.graph.n_firm),.] = compatible
    core = kssbc_cmg__workspace_apply(
        *context.hierarchy,context.workspace,injected)
    if (core.status != "CONVERGED") return(core)
    out = core
    out.value = kssbc_cmg__project_labels(
        firm_component,core.value[(1::fine.graph.n_firm),.])
    if (rows(out.value) != fine.graph.n_firm | hasmissing(out.value)) {
        out.status = "PULLBACK_BREAKDOWN"
        out.message = "KSS quotient pullback failed"
        out.value = J(0,0,.)
    }
    return(out)
}

struct kssbc_preconditioner_result scalar kssbc_solver__cmg_apply(
    pointer scalar context,
    struct kssbc_fe_design scalar design,
    real matrix residual)
{
    struct kssbc_preconditioner_result scalar out
    struct kssbc_cmg__apply_result scalar applied
    pointer(struct kssbc_solver_cmg_context scalar) scalar cmg_context
    pointer(struct kssbc_cmg__hierarchy scalar) scalar hierarchy

    out.status = "INVALID_SOLVER_BACKEND"
    out.message = "invalid CMG preconditioner input"
    out.value = J(0,0,.)
    cmg_context = context
    if (context == NULL | design.status != "CONVERGED" |
        rows(residual) != design.firm_levels | cols(residual) < 1 |
        hasmissing(residual)) return(out)
    hierarchy = (*cmg_context).hierarchy
    if (hierarchy == NULL) return(out)
    if ((*cmg_context).use_workspace &
        cols(residual) <= (*cmg_context).workspace.batch_capacity) {
        applied = kssbc_solver__cmg_apply_ws(
            *cmg_context,residual)
    }
    else applied = kssbc_cmg__apply_kss(*hierarchy,residual)
    out.status = applied.status
    out.message = applied.message
    out.value = applied.value
    return(out)
}

struct kssbc_solver_backend scalar kssbc_solver__cmg_backend(
    pointer(struct kssbc_solver_cmg_context scalar) scalar context)
{
    struct kssbc_solver_backend scalar out

    out.route = "CMG"
    out.context = context
    out.apply = &kssbc_solver__cmg_apply()
    out.exact_inverse = ((*context).hierarchy != NULL &
        (*(*context).hierarchy).status == "CONVERGED" &
        (*(*context).hierarchy).n_level == 1)
    return(out)
}

// Internal hierarchy_from_cells path.  Its caller owns cells preparation and
// preflight validation, so graph construction never rescans the retained
// observation arrays.
struct kssbc_cmg__hierarchy scalar kssbc_solver__hierarchy_cells(
    struct kssbc_cmg__cells scalar cells,
    real scalar memory_envelope_bytes,
    real scalar planned_rhs)
{
    struct kssbc_cmg__hierarchy scalar out
    struct kssbc_cmg__graph scalar graph
    struct kssbc_cmg__options scalar options

    out = kssbc_cmg__empty_hierarchy()
    if (cells.status != "CONVERGED" |
        missing(memory_envelope_bytes) | memory_envelope_bytes <= 0 |
        missing(planned_rhs) | planned_rhs < 1 |
        planned_rhs != floor(planned_rhs)) {
        out.message = "validated CMG cells and resources are required"
        return(out)
    }
    graph = kssbc_cmg__hybrid_build(cells)
    if (graph.status != "CONVERGED") {
        out.status = graph.status
        out.message = graph.message
        return(out)
    }
    options = kssbc_cmg__options_resource(
        memory_envelope_bytes,graph.n_vertex,planned_rhs)
    return(kssbc_cmg__hierarchy_build(graph,options))
}

struct kssbc_cmg__hierarchy scalar kssbc_solver__hierarchy(
    struct kssbc_fe_design scalar design,
    real colvector worker_key,
    real colvector firm_key,
    real scalar memory_envelope_bytes,
    real scalar planned_rhs)
{
    struct kssbc_cmg__hierarchy scalar out
    struct kssbc_cmg__cells scalar cells
    struct kssbc_cmg__options scalar options
    struct kssbc_cmg__preflight_result scalar preflight

    out = kssbc_cmg__empty_hierarchy()
    if (design.status != "CONVERGED") {
        out.message = "KSS design is not prepared"
        return(out)
    }
    cells = kssbc_cmg__cells_prepare(
        design.worker,design.firm,design.frequency,worker_key,firm_key)
    if (cells.status != "CONVERGED") {
        out.status = cells.status
        out.message = cells.message
        return(out)
    }
    options = kssbc_cmg__options_resource(
        memory_envelope_bytes,design.firm_levels,planned_rhs)
    preflight = kssbc_cmg__preflight(
        cells,planned_rhs,1,memory_envelope_bytes,options)
    if (preflight.status != "CONVERGED") {
        out.status = preflight.status
        out.message = preflight.message
        return(out)
    }
    return(kssbc_solver__hierarchy_cells(
        cells,memory_envelope_bytes,planned_rhs))
}

real matrix kssbc_solver__pilot_rhs(
    struct kssbc_fe_design scalar design,
    real colvector firm_key)
{
    real matrix firm_rhs
    real colvector component

    // Pilot RHSs live on firm coordinates, not hybrid coordinates.  The
    // retained worker-firm graph is connected after production pruning, so
    // the firm quotient has one component.
    component = J(design.firm_levels,1,1)
    firm_rhs = kssbc_cmg__pilot_rhs(firm_key,component)
    if (rows(firm_rhs) != design.firm_levels) return(J(0,0,.))
    return(J(design.worker_levels,4,0) \
        firm_rhs)
}

real scalar kssbc_solver__pilots_valid(
    struct kssbc_solve_result scalar solved,
    real scalar cap,
    real scalar tolerance)
{
    real scalar residual_gate

    residual_gate = max((1e-11,10*tolerance))
    if (solved.status != "CONVERGED" | cols(solved.rhs_status) != 4 |
        cols(solved.rhs_iterations) != 4 |
        cols(solved.rhs_relres) != 4 | hasmissing(solved.rhs_iterations) |
        hasmissing(solved.rhs_relres) | max(solved.rhs_iterations) > cap |
        max(solved.rhs_relres) > residual_gate |
        min(solved.rhs_status :== "CONVERGED") != 1) {
        return(0)
    }
    return(1)
}

struct kssbc_route_result scalar kssbc_solver__jla_routed(
    real colvector y,
    real colvector worker,
    real colvector firm,
    real matrix controls,
    real colvector frequency,
    real colvector target_weight,
    real colvector deletion_id,
    string scalar deletion,
    string scalar nuisance,
    real scalar probes,
    real scalar batch,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit,
    string scalar requested_route,
    real scalar memory_envelope_bytes,
    | pointer scalar estimator_callback,
    pointer scalar estimator_context,
    real colvector semantic_rank,
    real scalar semantic_atom_mode)
{
    struct kssbc_route_result scalar out
    struct kssbc_fe_design scalar base
    struct kssbc_cmg__cells scalar cells
    struct kssbc_cmg__options scalar options
    struct kssbc_cmg__preflight_result scalar preflight
    struct kssbc_cmg__hierarchy scalar hierarchy
    struct kssbc_cmg__level scalar fine
    struct kssbc_solver_cmg_context scalar cmg_context
    struct kssbc_solver_backend scalar diagonal_backend, cmg_backend, backend
    struct kssbc_solve_result scalar diagonal_pilots, cmg_pilots
    struct kssbc_solver_pilot_evidence scalar diagonal_evidence, cmg_evidence
    real colvector worker_key, firm_key
    real matrix pilot_rhs
    real scalar planned_rhs, setup_seconds, hierarchy_seconds
    real scalar diagonal_seconds, cmg_seconds, pilot_cap
    real scalar diagonal_work, cmg_work, work_ratio
    real scalar hybrid_vertices, hybrid_edges, forecast_peak, route_code
    real scalar use_cmg, diagonal_valid, diagonal_realistic, cmg_valid
    real scalar diagonal_max_iterations, cmg_max_iterations
    real scalar workspace_capacity, solver_memory_bytes
    real scalar hierarchy_memory_bytes
    real scalar base_bytes, hierarchy_peak
    real scalar terminal_vertices, use_callback

    out = kssbc_solver__empty_route()
    use_callback = 0
    if (args() >= 20) use_callback = (estimator_callback != NULL)
    requested_route = strupper(strtrim(requested_route))
    planned_rhs = kssbc_solver__planned_rhs(
        probes,cols(controls),nuisance)
    if (missing(planned_rhs) |
        !(requested_route == "AUTO" | requested_route == "DIAGONAL" |
          requested_route == "CMG") |
        missing(memory_envelope_bytes) |
        memory_envelope_bytes < 1024^3 |
        memory_envelope_bytes > 56*1024^3) return(out)
    if (requested_route != "DIAGONAL" &
        !kssbc_solver__cmg_runtime_ok()) {
        out.estimator = kssbc__failure(
            "CMG_VERSION_MISMATCH",
            "CMG API 5 robust-hierarchy runtime is required")
        out.status = out.estimator.status
        out.message = out.estimator.message
        return(out)
    }
    // Legacy direct calls retain the registered 65% solver reservation.  A
    // configured whole-command gate instead derives the exact routed budget
    // from hard_memory/1.30 minus non-solver numerical overlap; the final
    // route reconciliation below remains authoritative.
    solver_memory_bytes =
        kssbc_solver__resource_budget(memory_envelope_bytes)
    if (missing(solver_memory_bytes) | solver_memory_bytes <= 0) {
        out.estimator = kssbc__failure(
            "SOLVER_MEMORY_LIMIT",
            "whole-command allocations leave no positive solver budget")
        out.status = out.estimator.status
        out.message = out.estimator.message
        return(out)
    }

    timer_clear(88)
    timer_clear(89)
    timer_clear(90)
    timer_on(88)
    base = kssbc__fe_prepare(worker,firm,frequency,rank_tolerance)
    timer_off(88)
    if (base.status != "CONVERGED") {
        out.estimator = kssbc__failure(base.status,base.message)
        out.status = base.status
        out.message = base.message
        return(out)
    }
    setup_seconds = kssbc__timer_seconds(88)
    base_bytes = 8*(8*base.n+
        5*(base.worker_levels+base.firm_levels))
    hierarchy_memory_bytes = solver_memory_bytes-base_bytes
    hierarchy = kssbc_cmg__empty_hierarchy()
    diagonal_backend = kssbc__diagonal_backend()
    backend = diagonal_backend
    // Route diagnostic slot 22 is cumulative CMG structural-preparation
    // time: the one cells/preflight pass, graph/hierarchy construction if
    // attempted, and bounded backend-context initialization.  It excludes
    // deterministic B1 and CMG pilot solves.
    hierarchy_seconds = 0
    preflight = kssbc_cmg__empty_preflight()
    preflight.planned_rhs = planned_rhs
    if (max((planned_rhs,8)) < 32) preflight.pilot_cap = 128
    else if (max((planned_rhs,8)) < 128) preflight.pilot_cap = 64
    else preflight.pilot_cap = 32
    preflight.predicted_vertices = 0
    preflight.predicted_edges = 0
    preflight.predicted_structural_bytes = 0
    preflight.predicted_scratch_bytes = 0
    if (requested_route == "DIAGONAL") {
        // Explicit B1 does not depend on canonical CMG keys, construction
        // scratch, hierarchy feasibility, or any CMG preflight status.
        preflight.status = "NOT_RUN"
        preflight.route = "DIAGONAL"
        preflight.message = "CMG preflight bypassed for explicit diagonal B1"
    }
    else {
        if (missing(hierarchy_memory_bytes) |
            hierarchy_memory_bytes <= 0) {
            out.estimator = kssbc__failure(
                "SOLVER_MEMORY_LIMIT",
                "persistent FE design exhausts the solver memory reservation")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        worker_key = kssbc_solver__canonical_keys(
            base.worker,base.worker_levels)
        firm_key = kssbc_solver__canonical_keys(
            base.firm,base.firm_levels)
        if (rows(worker_key) != base.worker_levels |
            rows(firm_key) != base.firm_levels) {
            out.estimator = kssbc__failure(
                "CANONICAL_KEYS_UNAVAILABLE",
                "ID-free canonical CMG vertex keys are unavailable")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        timer_on(90)
        cells = kssbc_cmg__cells_prepare(
            base.worker,base.firm,base.frequency,worker_key,firm_key)
        if (cells.status != "CONVERGED") {
            timer_off(90)
            out.estimator = kssbc__failure(cells.status,cells.message)
            out.status = cells.status
            out.message = cells.message
            return(out)
        }
        options = kssbc_cmg__options_resource(
            hierarchy_memory_bytes,base.firm_levels,planned_rhs)
        preflight = kssbc_cmg__preflight(
            cells,planned_rhs,1,hierarchy_memory_bytes,options)
        if (preflight.status != "CONVERGED") {
            timer_off(90)
            // Without bounded B1 pilots there is no evidence that fallback is
            // realistic.  AUTO therefore fails closed before RNG rather than
            // treating a CMG preflight error as permission to launch B1.
            out.estimator = kssbc__failure(
                preflight.status,preflight.message)
            out.status = preflight.status
            out.message = preflight.message
            return(out)
        }
        if (preflight.predicted_structural_bytes+
            preflight.predicted_scratch_bytes > hierarchy_memory_bytes) {
            timer_off(90)
            out.estimator = kssbc__failure(
                "SOLVER_MEMORY_LIMIT",
                "CMG preflight plus persistent FE design exceeds the solver memory reservation")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        timer_off(90)
        hierarchy_seconds = kssbc__timer_seconds(90)
    }
    pilot_cap = preflight.pilot_cap
    hybrid_vertices = preflight.predicted_vertices
    hybrid_edges = preflight.predicted_edges
    diagonal_seconds = 0
    cmg_seconds = 0
    diagonal_work = .
    cmg_work = .
    work_ratio = .
    diagonal_max_iterations = .
    cmg_max_iterations = .
    route_code = 1
    terminal_vertices = .

    if (requested_route == "DIAGONAL") {
        out.route = "DIAGONAL"
        out.reason = "diagonal B1 was explicitly requested"
    }
    else if (requested_route == "AUTO" & preflight.route == "DIAGONAL") {
        out.route = "DIAGONAL"
        out.reason = preflight.message
    }
    else {
        pilot_rhs = kssbc_solver__pilot_rhs(base,firm_key)
        if (rows(pilot_rhs) == 0) {
            out.estimator = kssbc__failure(
                "ROUTING_PILOT_FAILED",
                "deterministic routing right-hand sides are unavailable")
            out.status = out.estimator.status
            out.message = out.estimator.message
            return(out)
        }
        timer_on(89)
        diagonal_pilots = kssbc__fe_solve_matrix_backend(
            base,pilot_rhs,tolerance,maxiter,diagonal_backend)
        timer_off(89)
        diagonal_seconds = kssbc__timer_seconds(89)
        diagonal_evidence = kssbc_solver__pilot_evidence(
            diagonal_pilots,1,planned_rhs,pilot_cap,tolerance,base.n,
            base.worker_levels,base.firm_levels,.,.,.,.,.,0)
        out.pilot_diagnostics[1..4,.] = diagonal_evidence.diagnostics
        out.pilot_status[1..4] = diagonal_evidence.status
        out.pilot_failure_reason[1..4] =
            diagonal_evidence.failure_reason
        diagonal_valid = kssbc_solver__pilots_valid(
            diagonal_pilots,pilot_cap,tolerance)
        if (diagonal_pilots.status == "CONVERGED") {
            diagonal_max_iterations = max(diagonal_pilots.rhs_iterations)
            diagonal_work = kssbc_solver__diagonal_work(
                planned_rhs,diagonal_max_iterations,base.n,
                base.worker_levels,base.firm_levels)
        }
        diagonal_realistic = diagonal_valid &
            kssbc_solver__fallback_work_ok(diagonal_work)
        if (requested_route == "AUTO" & diagonal_realistic &
            max(diagonal_pilots.rhs_iterations) <= 4) {
            out.route = "DIAGONAL"
            out.reason =
                "four deterministic pilots converged in at most four B1 steps"
        }
        else {
            timer_on(90)
            hierarchy = kssbc_solver__hierarchy_cells(
                cells,hierarchy_memory_bytes,planned_rhs)
            timer_off(90)
            hierarchy_seconds = kssbc__timer_seconds(90)
            if (hierarchy.n_level >= 1) {
                fine = *hierarchy.level[1]
                hybrid_vertices = fine.graph.n_vertex
                hybrid_edges = fine.graph.n_edge
            }
            if (hierarchy.status == "CONVERGED") {
                terminal_vertices =
                    (*hierarchy.level[hierarchy.n_level]).graph.n_vertex
            }
            if (hierarchy.status != "CONVERGED") {
                if (requested_route == "AUTO" & diagonal_realistic) {
                    out.route = "DIAGONAL"
                    out.reason = "CMG setup failed before RNG; B1 passed bounded iteration, complete-residual, and work gates"
                    out.fallback_status = hierarchy.status
                    out.fallback_message = hierarchy.message
                }
                else {
                    if (requested_route == "CMG") {
                        out.estimator = kssbc__failure(
                            "FORCED_CMG_FAILED",hierarchy.message)
                    }
                    else out.estimator = kssbc__failure(
                        hierarchy.status,hierarchy.message)
                    out.status = out.estimator.status
                    out.message = out.estimator.message
                    out.route = "CMG"
                    out.reason = hierarchy.message
                    hierarchy_peak = max((
                        preflight.predicted_structural_bytes+
                            preflight.predicted_scratch_bytes,
                        hierarchy.structural_bytes+
                            hierarchy.dense_factor_bytes))
                    forecast_peak = base_bytes+hierarchy_peak
                    out.diagnostics = (planned_rhs,memory_envelope_bytes,
                        setup_seconds,hierarchy.n_level,
                        hierarchy.edge_complexity,hierarchy.vertex_complexity,
                        hierarchy.structural_bytes,hierarchy.dense_factor_bytes,
                        base.worker_levels,base.firm_levels,hybrid_vertices,
                        hybrid_edges,2,preflight.predicted_vertices,
                        preflight.predicted_edges,
                        preflight.predicted_structural_bytes,
                        preflight.predicted_scratch_bytes,pilot_cap,
                        diagonal_max_iterations,.,diagonal_seconds,
                        hierarchy_seconds,.,.,forecast_peak,
                        terminal_vertices)
                    return(out)
                }
            }
            else {
                workspace_capacity = max((4,batch,cols(controls)))
                timer_on(90)
                // Scale profiling rejects the reusable-workspace path: it was
                // 34--74% slower at 32,768 vertices (and about 63% slower at
                // 100,000 vertices).  Retain the certified API for equality
                // tests, but use the faster ordinary batched CMG application.
                cmg_context = kssbc_solver__cmg_context(
                    &hierarchy,workspace_capacity,hierarchy_memory_bytes,0)
                timer_off(90)
                hierarchy_seconds = kssbc__timer_seconds(90)
                cmg_backend = kssbc_solver__cmg_backend(&cmg_context)
                timer_clear(89)
                timer_on(89)
                cmg_pilots = kssbc__fe_solve_matrix_backend(
                    base,pilot_rhs,tolerance,maxiter,cmg_backend)
                timer_off(89)
                cmg_seconds = kssbc__timer_seconds(89)
                cmg_evidence = kssbc_solver__pilot_evidence(
                    cmg_pilots,2,planned_rhs,min((pilot_cap,250)),
                    tolerance,base.n,base.worker_levels,base.firm_levels,
                    hybrid_vertices,hybrid_edges,hierarchy.edge_complexity,
                    hierarchy.vertex_complexity,hierarchy.dense_factor_bytes,
                    cmg_backend.exact_inverse)
                out.pilot_diagnostics[5..8,.] = cmg_evidence.diagnostics
                out.pilot_status[5..8] = cmg_evidence.status
                out.pilot_failure_reason[5..8] =
                    cmg_evidence.failure_reason
                cmg_valid = kssbc_solver__pilots_valid(cmg_pilots,
                    min((pilot_cap,250)),tolerance)
                if (cmg_pilots.status == "CONVERGED") {
                    cmg_max_iterations = max(cmg_pilots.rhs_iterations)
                    cmg_work = kssbc_solver__cmg_work(
                        planned_rhs,cmg_max_iterations,base.n,
                        base.worker_levels,base.firm_levels,
                        hybrid_vertices,hybrid_edges,
                        hierarchy.edge_complexity,
                        hierarchy.vertex_complexity,
                        hierarchy.dense_factor_bytes)
                }
                if (!missing(diagonal_work) & diagonal_work > 0 &
                    !missing(cmg_work)) {
                    work_ratio = cmg_work/diagonal_work
                }
                use_cmg = cmg_valid &
                    (requested_route == "CMG" | !diagonal_realistic |
                     kssbc_solver__cmg_work_wins(work_ratio))
                if (requested_route == "CMG" & !cmg_valid) {
                    out.estimator = kssbc__failure(
                        "FORCED_CMG_FAILED",
                        "forced CMG pilots did not pass the bounded iteration and complete-residual gate")
                    out.status = out.estimator.status
                    out.message = out.estimator.message
                    out.route = "CMG"
                    out.reason = out.estimator.message
                    hierarchy_peak = max((
                        preflight.predicted_structural_bytes+
                            preflight.predicted_scratch_bytes,
                        hierarchy.structural_bytes+
                            hierarchy.dense_factor_bytes+
                            hierarchy.options.action_scratch_bytes))
                    if (cmg_context.use_workspace) {
                        hierarchy_peak = max((hierarchy_peak,
                            cmg_context.workspace.predicted_peak_bytes+
                                hierarchy.dense_factor_bytes))
                    }
                    forecast_peak = base_bytes+hierarchy_peak
                    out.diagnostics = (planned_rhs,memory_envelope_bytes,
                        setup_seconds+diagonal_seconds+hierarchy_seconds+
                            cmg_seconds,
                        hierarchy.n_level,hierarchy.edge_complexity,
                        hierarchy.vertex_complexity,
                        hierarchy.structural_bytes,
                        hierarchy.dense_factor_bytes,base.worker_levels,
                        base.firm_levels,hybrid_vertices,hybrid_edges,2,
                        preflight.predicted_vertices,
                        preflight.predicted_edges,
                        preflight.predicted_structural_bytes,
                        preflight.predicted_scratch_bytes,pilot_cap,
                        diagonal_max_iterations,cmg_max_iterations,
                        diagonal_seconds,hierarchy_seconds,cmg_seconds,
                        work_ratio,forecast_peak,terminal_vertices)
                    return(out)
                }
                if (use_cmg) {
                    backend = cmg_backend
                    out.route = "CMG"
                    route_code = 2
                    if (!diagonal_valid) {
                        out.reason =
                            "B1 pilots exceeded the bounded iteration or complete-residual gate"
                    }
                    else if (!diagonal_realistic) {
                        out.reason =
                            "B1 projected work exceeds the bounded fallback envelope"
                    }
                    else out.reason =
                        "CMG passed deterministic convergence and amortized-work gates"
                }
                else if (diagonal_realistic) {
                    out.route = "DIAGONAL"
                    out.reason =
                        "B1 passed bounded gates and CMG did not improve deterministic repeated-RHS work"
                    if (!cmg_valid) {
                        out.fallback_status = "CMG_PILOT_REJECTED"
                        out.fallback_message =
                            "CMG pilots did not pass the bounded iteration and complete-residual gate"
                    }
                }
                else {
                    out.estimator = kssbc__failure(
                        "NO_REALISTIC_SOLVER_ROUTE",
                        "neither B1 nor CMG passed bounded convergence, complete-residual, and deterministic-work gates")
                    out.status = out.estimator.status
                    out.message = out.estimator.message
                    out.route = "CMG"
                    out.reason = out.estimator.message
                    hierarchy_peak = max((
                        preflight.predicted_structural_bytes+
                            preflight.predicted_scratch_bytes,
                        hierarchy.structural_bytes+
                            hierarchy.dense_factor_bytes+
                            hierarchy.options.action_scratch_bytes))
                    if (cmg_context.use_workspace) {
                        hierarchy_peak = max((hierarchy_peak,
                            cmg_context.workspace.predicted_peak_bytes+
                                hierarchy.dense_factor_bytes))
                    }
                    forecast_peak = base_bytes+hierarchy_peak
                    out.diagnostics = (planned_rhs,memory_envelope_bytes,
                        setup_seconds+diagonal_seconds+hierarchy_seconds+
                            cmg_seconds,
                        hierarchy.n_level,hierarchy.edge_complexity,
                        hierarchy.vertex_complexity,
                        hierarchy.structural_bytes,
                        hierarchy.dense_factor_bytes,base.worker_levels,
                        base.firm_levels,hybrid_vertices,hybrid_edges,2,
                        preflight.predicted_vertices,
                        preflight.predicted_edges,
                        preflight.predicted_structural_bytes,
                        preflight.predicted_scratch_bytes,pilot_cap,
                        diagonal_max_iterations,cmg_max_iterations,
                        diagonal_seconds,hierarchy_seconds,cmg_seconds,
                        work_ratio,forecast_peak,terminal_vertices)
                    return(out)
                }
            }
        }
    }

    setup_seconds = setup_seconds+diagonal_seconds+hierarchy_seconds+cmg_seconds
    // Enforce the solver reservation before kssbc__jla_backend initializes
    // the registered random stream.  A direct B1 route does not reserve a
    // hypothetical CMG graph; its persistent FE-design storage is linear in
    // the retained rows and coordinates.
    forecast_peak = base_bytes
    if (hierarchy.status == "CONVERGED") {
        hierarchy_peak = hierarchy.structural_bytes+
            hierarchy.dense_factor_bytes+
            hierarchy.options.action_scratch_bytes
        if (cmg_context.use_workspace) {
            hierarchy_peak = max((hierarchy_peak,
                cmg_context.workspace.predicted_peak_bytes+
                    hierarchy.dense_factor_bytes))
        }
        forecast_peak = base_bytes+hierarchy_peak
    }
    else if (hierarchy.attempted_n_level > 0) {
        hierarchy_peak = max((
            preflight.predicted_structural_bytes+
                preflight.predicted_scratch_bytes,
            hierarchy.structural_bytes+hierarchy.dense_factor_bytes))
        forecast_peak = base_bytes+hierarchy_peak
    }
    out.diagnostics = (planned_rhs,memory_envelope_bytes,setup_seconds,
        hierarchy.n_level,hierarchy.edge_complexity,
        hierarchy.vertex_complexity,hierarchy.structural_bytes,
        hierarchy.dense_factor_bytes,base.worker_levels,base.firm_levels,
        hybrid_vertices,hybrid_edges,route_code,
        preflight.predicted_vertices,preflight.predicted_edges,
        preflight.predicted_structural_bytes,
        preflight.predicted_scratch_bytes,pilot_cap,
        diagonal_max_iterations,cmg_max_iterations,
        diagonal_seconds,hierarchy_seconds,cmg_seconds,work_ratio,
        forecast_peak,terminal_vertices)
    if (missing(forecast_peak)) {
        out.estimator = kssbc__failure(
            "SOLVER_MEMORY_LIMIT",
            "solver resource forecast is nonfinite")
        out.status = out.estimator.status
        out.message = out.estimator.message
        out.reason = out.estimator.message
        return(out)
    }
    if (kssbc_solver__resource_active() &
        !kssbc_solver__resource_apply(forecast_peak)) {
        out.estimator = kssbc__failure(
            kssbc_solver__resource_status(),
            kssbc_solver__resource_message())
        out.status = out.estimator.status
        out.message = out.estimator.message
        out.reason = out.estimator.message
        return(out)
    }
    if (forecast_peak > solver_memory_bytes) {
        out.estimator = kssbc__failure(
            "SOLVER_MEMORY_LIMIT",
            "solver forecast exceeds its routed memory budget")
        out.status = out.estimator.status
        out.message = out.estimator.message
        out.reason = out.estimator.message
        return(out)
    }
    if (use_callback) {
        out.estimator = (*estimator_callback)(
            estimator_context,base,backend,setup_seconds)
    }
    else {
        if (args() >= 23) {
            out.estimator = kssbc__jla_backend(
                y,worker,firm,controls,frequency,target_weight,deletion_id,
                deletion,nuisance,probes,batch,seed,tolerance,maxiter,
                rank_tolerance,block_tolerance,blocksize_limit,
                base,backend,setup_seconds,semantic_rank,semantic_atom_mode)
        }
        else {
            out.estimator = kssbc__jla_backend(
                y,worker,firm,controls,frequency,target_weight,deletion_id,
                deletion,nuisance,probes,batch,seed,tolerance,maxiter,
                rank_tolerance,block_tolerance,blocksize_limit,
                base,backend,setup_seconds)
        }
    }
    out.status = out.estimator.status
    out.message = out.estimator.message
    return(out)
}

void kssbc__stata_jla_routed(
    string scalar y_name,
    string scalar worker_name,
    string scalar firm_name,
    string scalar controls_names,
    string scalar frequency_name,
    string scalar target_name,
    string scalar deletion_name,
    string scalar sample_name,
    string scalar semantic_rank_name,
    real scalar semantic_atom_mode,
    string scalar deletion,
    string scalar nuisance,
    real scalar probes,
    real scalar batch,
    real scalar seed,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance,
    real scalar blocksize_limit,
    string scalar requested_route,
    real scalar memory_envelope_bytes,
    string scalar results_name,
    string scalar status_local,
    string scalar message_local,
    string scalar diagnostics_name,
    string scalar solver_diagnostics_name,
    string scalar route_diagnostics_name,
    string scalar route_local,
    string scalar reason_local,
    string scalar fallback_status_local,
    string scalar fallback_message_local,
    | string scalar pilot_diagnostics_name,
    string scalar pilot_status_local,
    string scalar pilot_failure_local)
{
    struct kssbc_route_result scalar routed
    struct kssbc_result scalar out
    real colvector y, worker, firm, frequency, target, deletion_id
    real colvector semantic_rank
    real matrix controls, results, diagnostics

    y = st_data(.,y_name,sample_name)
    worker = st_data(.,worker_name,sample_name)
    firm = st_data(.,firm_name,sample_name)
    if (strtrim(controls_names) == "") controls = J(rows(y),0,.)
    else controls = st_data(.,tokens(controls_names),sample_name)
    frequency = st_data(.,frequency_name,sample_name)
    target = st_data(.,target_name,sample_name)
    deletion_id = st_data(.,deletion_name,sample_name)
    semantic_rank = st_data(.,semantic_rank_name,sample_name)
    routed = kssbc_solver__jla_routed(
        y,worker,firm,controls,frequency,target,deletion_id,
        deletion,nuisance,probes,batch,seed,tolerance,maxiter,
        rank_tolerance,block_tolerance,blocksize_limit,
        requested_route,memory_envelope_bytes,NULL,NULL,
        semantic_rank,semantic_atom_mode)
    out = routed.estimator
    results = out.plugin \ out.correction \ out.corrected \
        out.numerical_mcse
    diagnostics = (out.n_stored,out.n_physical,out.worker_levels,
        out.firm_levels,out.parameters,out.deletion_units,
        out.target_weight_sum,out.max_leverage,out.information_rcond,
        out.inverse_relres,out.solver_iterations,
        out.solver_max_residual,out.probes,out.weighted_rss,
        out.fit_seconds,out.leverage_seconds,out.target_seconds,
        out.correction_seconds,out.preconditioner_seconds,
        out.preconditioner_ratio,out.control_schur_rcond,
        out.deletion_rank_gap,out.full_parameters,
        out.correction_parameters,out.schur_seconds,
        out.preconditioner_apply_seconds,out.pcg_seconds,
        out.solver_backend_seconds,out.solver_schur_actions,
        out.solver_schur_batches,out.solver_precond_applications,
        out.solver_precond_batches)
    st_matrix(results_name,results)
    st_matrix(diagnostics_name,diagnostics)
    st_matrix(solver_diagnostics_name,out.solver_rhs_diagnostics)
    st_matrix(route_diagnostics_name,routed.diagnostics)
    if (args() >= 33) {
        st_matrix(pilot_diagnostics_name,routed.pilot_diagnostics)
    }
    if (args() >= 34) {
        st_local(pilot_status_local,
            invtokens(routed.pilot_status,"|"))
    }
    if (args() >= 35) {
        st_local(pilot_failure_local,
            invtokens(routed.pilot_failure_reason,"|"))
    }
    st_local(status_local,out.status)
    st_local(message_local,out.message)
    st_local(route_local,routed.route)
    st_local(reason_local,routed.reason)
    st_local(fallback_status_local,routed.fallback_status)
    st_local(fallback_message_local,routed.fallback_message)
}

KSSBC_SOLVER_RESOURCE_GATE = kssbc_solver__resource_empty()

end
