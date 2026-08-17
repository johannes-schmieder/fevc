*! kss_bc KSS-SCALE-1 resource admission
*! version 0.2.0-dev 16aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

real scalar kssbc_resource__api_level()
{
    return(3)
}

string scalar kssbc_resource__build_id()
{
    return("kss-bc-resource-api3-routed-component-receipt")
}

real scalar kssbc_resource__gib()
{
    return(1024^3)
}

real scalar kssbc_resource__hard_mem_bytes()
{
    return(56*kssbc_resource__gib())
}

real scalar kssbc_resource__hard_wall_secs()
{
    return(12*60*60)
}

real scalar kssbc_resource__wall_margin()
{
    return(0.50)
}

real scalar kssbc_resource__rng_call_upper()
{
    // Provisional K1 upper charge for one registered scalar/vector binomial
    // call.  A measured upper bound may be supplied to the model instead.
    // The deliberately conservative millisecond charge makes a many-chunk
    // exact draw visible to pre-RNG wall admission.
    return(0.001)
}

struct kssbc_resource_components
{
    real scalar raw_stata_bytes
    real scalar cell_bytes
    real scalar deletion_unit_bytes
    real scalar target_stratum_bytes
    real scalar cmg_hierarchy_bytes
    real scalar phase_scratch_bytes
    real scalar sorting_compression_bytes
    real scalar solve_ahead_bytes
    real scalar output_certificate_bytes
    real scalar preservation_transition_bytes
}

struct kssbc_resource_forecast
{
    string scalar status
    string scalar message
    string scalar route
    real scalar admitted
    real scalar before_rng
    struct kssbc_resource_components scalar components
    real rowvector component_bytes
    real scalar persistent_compressed_bytes
    real scalar selection_peak_bytes
    real scalar transition_peak_bytes
    real scalar numerical_peak_bytes
    real scalar restoration_peak_bytes
    real scalar peak_bytes
    string scalar peak_phase
    real scalar memory_headroom_fraction
    real scalar memory_admission_bytes
    real scalar wall_forecast_upper_seconds
    real scalar wall_headroom_fraction
    real scalar wall_admission_seconds
    real scalar hard_memory_bytes
    real scalar hard_wall_seconds
    real scalar memory_admitted
    real scalar wall_admitted
    real scalar non_solver_numerical_bytes
    real scalar routed_solver_peak_bytes
    real scalar route_reconciled
}

struct kssbc_resource_selection
{
    string scalar status
    string scalar message
    string scalar route
    string scalar compressed_status
    string scalar generic_status
    real scalar admitted
    real scalar before_rng
    real scalar compressed_eligible
}

struct kssbc_resource_reconciliation
{
    string scalar status
    string scalar message
    real rowvector actual_phase_bytes
    real rowvector phase_forecast_ratios
    real scalar process_peak_bytes
    real scalar qacct_maxvmem_bytes
    real scalar comparison_peak_bytes
    real scalar memory_forecast_ratio
    real scalar actual_wall_seconds
    real scalar wall_forecast_ratio
    real scalar within_phase_forecasts
    real scalar within_memory_forecast
    real scalar within_wall_forecast
    real scalar within_hard_limits
    real scalar next_scale_allowed
}

struct kssbc_resource_model
{
    string scalar status
    string scalar message
    real scalar row_scale
    real scalar structure_scale
    real scalar physical_scale
    real scalar n_physical
    real scalar leverage_rng_calls_per_probe
    real scalar target_rng_calls_per_probe
    real scalar rng_call_seconds_upper
    real scalar rng_total_calls
    real scalar rng_wall_upper_seconds
    real scalar compressed_wall_upper_seconds
    real scalar generic_wall_upper_seconds
    struct kssbc_resource_components scalar compressed_components
    struct kssbc_resource_components scalar generic_components
    struct kssbc_resource_forecast scalar compressed
    struct kssbc_resource_forecast scalar generic
}

struct kssbc_resource_components scalar kssbc_resource__empty_components()
{
    struct kssbc_resource_components scalar out

    out.raw_stata_bytes = 0
    out.cell_bytes = 0
    out.deletion_unit_bytes = 0
    out.target_stratum_bytes = 0
    out.cmg_hierarchy_bytes = 0
    out.phase_scratch_bytes = 0
    out.sorting_compression_bytes = 0
    out.solve_ahead_bytes = 0
    out.output_certificate_bytes = 0
    out.preservation_transition_bytes = 0
    return(out)
}

real rowvector kssbc_resource__component_vector(
    struct kssbc_resource_components scalar components)
{
    return((
        components.raw_stata_bytes,
        components.cell_bytes,
        components.deletion_unit_bytes,
        components.target_stratum_bytes,
        components.cmg_hierarchy_bytes,
        components.phase_scratch_bytes,
        components.sorting_compression_bytes,
        components.solve_ahead_bytes,
        components.output_certificate_bytes,
        components.preservation_transition_bytes))
}

string rowvector kssbc_resource__component_names()
{
    return((
        "raw_stata_bytes",
        "cell_bytes",
        "deletion_unit_bytes",
        "target_stratum_bytes",
        "cmg_hierarchy_bytes",
        "phase_scratch_bytes",
        "sorting_compression_bytes",
        "solve_ahead_bytes",
        "output_certificate_bytes",
        "preservation_transition_bytes"))
}

real scalar kssbc_resource__valid_components(
    struct kssbc_resource_components scalar components)
{
    real rowvector values

    values = kssbc_resource__component_vector(components)
    return(!missing(values) & min(values) >= 0)
}

struct kssbc_resource_components scalar kssbc_resource__components_from(
    real rowvector values)
{
    struct kssbc_resource_components scalar out

    out = kssbc_resource__empty_components()
    if (rows(values) != 1 | cols(values) != 10 |
        missing(values) | min(values) < 0) {
        out.raw_stata_bytes = .
        return(out)
    }
    out.raw_stata_bytes = values[1]
    out.cell_bytes = values[2]
    out.deletion_unit_bytes = values[3]
    out.target_stratum_bytes = values[4]
    out.cmg_hierarchy_bytes = values[5]
    out.phase_scratch_bytes = values[6]
    out.sorting_compression_bytes = values[7]
    out.solve_ahead_bytes = values[8]
    out.output_certificate_bytes = values[9]
    out.preservation_transition_bytes = values[10]
    return(out)
}

// This is the numerical allocation that overlaps the actual routed solver.
// It intentionally excludes CMG/base/hierarchy memory, which the final route
// helper receives from the solver's accepted route diagnostic.
real scalar kssbc_resource__nonsolver_peak(
    string scalar route,
    struct kssbc_resource_components scalar components)
{
    real scalar persistent, raw_during_numerical, out

    if (!(route == "compressed" | route == "generic") |
        !kssbc_resource__valid_components(components)) return(.)
    persistent = components.cell_bytes+
        components.deletion_unit_bytes+
        components.target_stratum_bytes
    raw_during_numerical =
        (route == "generic")*components.raw_stata_bytes
    out = raw_during_numerical+
        persistent+
        components.phase_scratch_bytes+
        components.solve_ahead_bytes+
        components.output_certificate_bytes
    if (missing(out) | out < 0) return(.)
    return(out)
}

struct kssbc_resource_forecast scalar kssbc_resource__empty_forecast(
    string scalar route)
{
    struct kssbc_resource_forecast scalar out

    out.status = "INVALID_RESOURCE_FORECAST"
    out.message = "resource forecast input is invalid"
    out.route = route
    out.admitted = 0
    out.before_rng = 1
    out.components = kssbc_resource__empty_components()
    out.component_bytes = J(1,10,.)
    out.persistent_compressed_bytes = .
    out.selection_peak_bytes = .
    out.transition_peak_bytes = .
    out.numerical_peak_bytes = .
    out.restoration_peak_bytes = .
    out.peak_bytes = .
    out.peak_phase = ""
    out.memory_headroom_fraction = .
    out.memory_admission_bytes = .
    out.wall_forecast_upper_seconds = .
    out.wall_headroom_fraction = kssbc_resource__wall_margin()
    out.wall_admission_seconds = .
    out.hard_memory_bytes = kssbc_resource__hard_mem_bytes()
    out.hard_wall_seconds = kssbc_resource__hard_wall_secs()
    out.memory_admitted = 0
    out.wall_admitted = 0
    out.non_solver_numerical_bytes = .
    out.routed_solver_peak_bytes = .
    out.route_reconciled = 0
    return(out)
}

// Every input component is an upper-bound estimate for the allocation family
// named by the field.  The overlap schedule is part of the API contract:
//
//   selection  = raw + sorting/compression + output/certificates
//   transition = raw + persistent compression + sorting/compression +
//                preservation transition + output/certificates
//   numerical  = persistent compression + CMG + phase scratch + solve-ahead
//                + output/certificates (+ raw for the generic engine)
//   restoration= raw + preservation transition + output/certificates
//
// The compressed lifecycle must release raw row state before CMG and phase
// scratch reach their peaks, and it must free numerical allocations before
// restoring the caller dataset.  A caller whose lifecycle overlaps more
// families must charge that overlap to the relevant component before calling
// this function.  The generic engine conservatively retains the raw dataset
// through its numerical peak.
struct kssbc_resource_forecast scalar kssbc_resource__forecast(
    string scalar route,
    struct kssbc_resource_components scalar components,
    real scalar wall_forecast_upper_seconds,
    real scalar memory_headroom_fraction,
    | real scalar hard_memory_bytes,
    real scalar hard_wall_seconds)
{
    struct kssbc_resource_forecast scalar out
    real rowvector peaks

    out = kssbc_resource__empty_forecast(route)
    out.components = components
    out.component_bytes = kssbc_resource__component_vector(components)
    out.memory_headroom_fraction = memory_headroom_fraction
    out.wall_forecast_upper_seconds = wall_forecast_upper_seconds

    if (args() >= 5) out.hard_memory_bytes = hard_memory_bytes
    if (args() >= 6) out.hard_wall_seconds = hard_wall_seconds

    if (!(route == "compressed" | route == "generic") |
        !kssbc_resource__valid_components(components) |
        missing(wall_forecast_upper_seconds) |
        wall_forecast_upper_seconds <= 0 |
        missing(memory_headroom_fraction) |
        memory_headroom_fraction < 0.25 |
        memory_headroom_fraction > 0.30 |
        missing(out.hard_memory_bytes) | out.hard_memory_bytes <= 0 |
        out.hard_memory_bytes > kssbc_resource__hard_mem_bytes() |
        missing(out.hard_wall_seconds) | out.hard_wall_seconds <= 0 |
        out.hard_wall_seconds > kssbc_resource__hard_wall_secs()) {
        return(out)
    }

    out.persistent_compressed_bytes =
        components.cell_bytes+
        components.deletion_unit_bytes+
        components.target_stratum_bytes
    out.selection_peak_bytes =
        components.raw_stata_bytes+
        components.sorting_compression_bytes+
        components.output_certificate_bytes
    out.transition_peak_bytes =
        components.raw_stata_bytes+
        out.persistent_compressed_bytes+
        components.sorting_compression_bytes+
        components.preservation_transition_bytes+
        components.output_certificate_bytes
    out.non_solver_numerical_bytes =
        kssbc_resource__nonsolver_peak(route,components)
    out.routed_solver_peak_bytes = components.cmg_hierarchy_bytes
    out.numerical_peak_bytes =
        out.non_solver_numerical_bytes+out.routed_solver_peak_bytes
    out.restoration_peak_bytes =
        components.raw_stata_bytes+
        components.preservation_transition_bytes+
        components.output_certificate_bytes
    peaks = (
        out.selection_peak_bytes,
        out.transition_peak_bytes,
        out.numerical_peak_bytes,
        out.restoration_peak_bytes)
    out.peak_bytes = max(peaks)
    if (out.peak_bytes == out.selection_peak_bytes) {
        out.peak_phase = "selection"
    }
    else if (out.peak_bytes == out.transition_peak_bytes) {
        out.peak_phase = "transition"
    }
    else if (out.peak_bytes == out.numerical_peak_bytes) {
        out.peak_phase = "numerical"
    }
    else {
        out.peak_phase = "restoration"
    }

    out.memory_admission_bytes =
        ceil(out.peak_bytes*(1+memory_headroom_fraction))
    out.wall_admission_seconds =
        ceil(wall_forecast_upper_seconds*
            (1+kssbc_resource__wall_margin()))
    if (missing((peaks,out.memory_admission_bytes,
                 out.wall_admission_seconds)) | out.peak_bytes <= 0) {
        out.message = "resource forecast is nonfinite or has no allocation"
        return(out)
    }

    out.memory_admitted =
        out.memory_admission_bytes <= out.hard_memory_bytes
    out.wall_admitted =
        out.wall_admission_seconds <= out.hard_wall_seconds
    if (out.memory_admitted & out.wall_admitted) {
        out.status = "ADMITTED"
        out.message = "route passes pre-RNG memory and wall admission"
        out.admitted = 1
        return(out)
    }

    if (route == "compressed") {
        out.status = "RESOURCE_ADMISSION_FAILED"
    }
    else {
        out.status = "GENERIC_RESOURCE_ADMISSION_FAILED"
    }
    if (!out.memory_admitted & !out.wall_admitted) {
        out.message =
            "forecast plus required memory and wall headroom exceeds hard limits"
    }
    else if (!out.memory_admitted) {
        out.message =
            "forecast plus required memory headroom exceeds the hard limit"
    }
    else {
        out.message =
            "forecast plus required wall headroom exceeds the hard limit"
    }
    return(out)
}

// Replace the provisional fifth component with the accepted route's complete
// solver allocation.  For DIAGONAL this is the FE/base allocation; for CMG it
// also includes the hierarchy, terminal factors, and action scratch.  The
// historical cmg_hierarchy_bytes field name is retained for receipt-schema
// compatibility, but the routed value replaces rather than adds to its
// provisional upper bound.  The final check always applies 30 percent memory
// headroom against the same caller/hard envelope and remains pre-RNG.
struct kssbc_resource_forecast scalar kssbc_resource__route_reconcile(
    struct kssbc_resource_forecast scalar forecast,
    real scalar routed_solver_peak_bytes)
{
    struct kssbc_resource_forecast scalar out
    real rowvector peaks

    out = forecast
    out.admitted = 0
    out.memory_admitted = 0
    out.wall_admitted = 0
    out.route_reconciled = 0
    out.non_solver_numerical_bytes =
        kssbc_resource__nonsolver_peak(out.route,out.components)
    if (!(out.route == "compressed" | out.route == "generic") |
        missing(out.non_solver_numerical_bytes) |
        missing(routed_solver_peak_bytes) | routed_solver_peak_bytes <= 0 |
        missing(out.hard_memory_bytes) | out.hard_memory_bytes <= 0 |
        out.hard_memory_bytes > kssbc_resource__hard_mem_bytes() |
        missing(out.hard_wall_seconds) | out.hard_wall_seconds <= 0 |
        out.hard_wall_seconds > kssbc_resource__hard_wall_secs() |
        missing(out.wall_forecast_upper_seconds) |
        out.wall_forecast_upper_seconds <= 0) {
        out.status = "INVALID_ROUTE_RESOURCE_FORECAST"
        out.message = "routed solver resource forecast is invalid"
        return(out)
    }

    out.routed_solver_peak_bytes = routed_solver_peak_bytes
    out.components.cmg_hierarchy_bytes = routed_solver_peak_bytes
    out.component_bytes = kssbc_resource__component_vector(out.components)
    out.numerical_peak_bytes =
        out.non_solver_numerical_bytes+routed_solver_peak_bytes
    peaks = (
        out.selection_peak_bytes,
        out.transition_peak_bytes,
        out.numerical_peak_bytes,
        out.restoration_peak_bytes)
    out.peak_bytes = max(peaks)
    if (out.peak_bytes == out.selection_peak_bytes) {
        out.peak_phase = "selection"
    }
    else if (out.peak_bytes == out.transition_peak_bytes) {
        out.peak_phase = "transition"
    }
    else if (out.peak_bytes == out.numerical_peak_bytes) {
        out.peak_phase = "numerical"
    }
    else out.peak_phase = "restoration"
    out.memory_headroom_fraction = 0.30
    out.memory_admission_bytes = ceil(1.30*out.peak_bytes)
    out.wall_headroom_fraction = kssbc_resource__wall_margin()
    out.wall_admission_seconds = ceil(
        (1+out.wall_headroom_fraction)*out.wall_forecast_upper_seconds)
    if (missing((peaks,out.memory_admission_bytes,
                 out.wall_admission_seconds)) | out.peak_bytes <= 0) {
        out.status = "INVALID_ROUTE_RESOURCE_FORECAST"
        out.message = "routed solver resource forecast is nonfinite"
        return(out)
    }
    out.memory_admitted =
        out.memory_admission_bytes <= out.hard_memory_bytes
    out.wall_admitted =
        out.wall_admission_seconds <= out.hard_wall_seconds
    out.route_reconciled = 1
    if (out.memory_admitted & out.wall_admitted) {
        out.status = "ADMITTED"
        out.message =
            "actual routed solver peak passes final pre-RNG admission"
        out.admitted = 1
    }
    else if (out.route == "compressed") {
        out.status = "RESOURCE_ADMISSION_FAILED"
        out.message =
            "actual compressed route exceeds final memory or wall admission"
    }
    else {
        out.status = "GENERIC_RESOURCE_ADMISSION_FAILED"
        out.message =
            "actual generic route exceeds final memory or wall admission"
    }
    return(out)
}

/*
The initial scale model is deliberately conservative and source registered.
Its wall constants come from the final KSS-PROD-1 CZ18 command at 4dfc416:
4,652 seconds cold wall, including 160 seconds of input preparation and 349
seconds of estimator setup.  The compressed projection charges those 509
seconds by retained-row scale, charges one quarter of the remaining command
by structural scale, and then doubles the result for unmeasured I/O,
connectivity, and iteration uncertainty.  The forecast API adds the separate
50 percent admission headroom.  SCC measurements must replace these constants
before any claim of fitted scaling is made.

The registered leverage and target binomial-call counts add an explicit wall
charge.  The default 0.001-second charge is a provisional upper bound, not a
timing claim; K1 measurements may replace it through the model argument.
Generic memory and wall scaling also include literal physical mass.

Every memory coefficient is an upper-bound allocation count, not an RSS fit.
The formulas expose all required families and their maximum overlap through
kssbc_resource__forecast().
*/
struct kssbc_resource_model scalar kssbc_resource__model(
    real scalar n_rows,
    real scalar n_physical,
    real scalar coefficient_cells,
    real scalar deletion_units,
    real scalar target_strata,
    real scalar workers,
    real scalar firms,
    real scalar parameters,
    real scalar probes,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar leverage_rng_calls_per_probe,
    real scalar target_rng_calls_per_probe,
    real scalar rng_call_seconds_upper,
    real scalar raw_stata_bytes,
    real scalar hard_memory_bytes,
    real scalar hard_wall_seconds)
{
    struct kssbc_resource_model scalar out
    real scalar persistent_coordinates, leverage_scratch, target_scratch
    real scalar generic_batch, rng_calls_per_probe
    real rowvector counts

    out.status = "INVALID_RESOURCE_MODEL"
    out.message = "resource-model dimensions are invalid"
    out.row_scale = .
    out.structure_scale = .
    out.physical_scale = .
    out.n_physical = .
    out.leverage_rng_calls_per_probe = .
    out.target_rng_calls_per_probe = .
    out.rng_call_seconds_upper = .
    out.rng_total_calls = .
    out.rng_wall_upper_seconds = .
    out.compressed_wall_upper_seconds = .
    out.generic_wall_upper_seconds = .
    out.compressed_components = kssbc_resource__empty_components()
    out.generic_components = kssbc_resource__empty_components()
    out.compressed = kssbc_resource__empty_forecast("compressed")
    out.generic = kssbc_resource__empty_forecast("generic")
    counts = (n_rows,n_physical,coefficient_cells,deletion_units,target_strata,
        workers,firms,parameters,probes,leverage_batch,target_batch)
    if (missing((n_rows,n_physical,coefficient_cells,deletion_units,target_strata,
            workers,firms,parameters,probes,leverage_batch,target_batch,
            leverage_rng_calls_per_probe,target_rng_calls_per_probe,
            rng_call_seconds_upper,
            raw_stata_bytes,hard_memory_bytes,hard_wall_seconds)) |
        min((n_rows,n_physical,coefficient_cells,deletion_units,target_strata,
             workers,parameters,probes,leverage_batch,target_batch)) < 1 |
        firms < 2 | any(counts :!= floor(counts)) |
        n_physical < n_rows |
        n_physical > 2^53-1 |
        leverage_rng_calls_per_probe < 1 |
        target_rng_calls_per_probe < 1 |
        leverage_rng_calls_per_probe !=
            floor(leverage_rng_calls_per_probe) |
        target_rng_calls_per_probe != floor(target_rng_calls_per_probe) |
        leverage_rng_calls_per_probe+target_rng_calls_per_probe >
            floor((2^53-1)/probes) |
        rng_call_seconds_upper <= 0 |
        raw_stata_bytes <= 0 | hard_memory_bytes <= 0 |
        hard_memory_bytes > kssbc_resource__hard_mem_bytes() |
        hard_wall_seconds <= 0 |
        hard_wall_seconds > kssbc_resource__hard_wall_secs()) return(out)

    out.row_scale = n_rows/8201888
    out.physical_scale = n_physical/8201888
    out.structure_scale = max((
        deletion_units/311730,
        workers/117529,
        firms/10603))
    out.n_physical = n_physical
    out.leverage_rng_calls_per_probe = leverage_rng_calls_per_probe
    out.target_rng_calls_per_probe = target_rng_calls_per_probe
    out.rng_call_seconds_upper = rng_call_seconds_upper
    rng_calls_per_probe =
        leverage_rng_calls_per_probe+target_rng_calls_per_probe
    out.rng_total_calls = probes*rng_calls_per_probe
    out.rng_wall_upper_seconds =
        out.rng_total_calls*rng_call_seconds_upper
    out.compressed_wall_upper_seconds = 2*(
        509*out.row_scale + (4652-509)*0.25*out.structure_scale)+
        out.rng_wall_upper_seconds
    out.generic_wall_upper_seconds =
        1.25*4652*max((out.row_scale,out.physical_scale,
                       out.structure_scale))+
        out.rng_wall_upper_seconds

    out.compressed_components.raw_stata_bytes = raw_stata_bytes
    out.compressed_components.cell_bytes = 8*(
        13*coefficient_cells+3*workers+5*firms)
    out.compressed_components.deletion_unit_bytes =
        8*(9*deletion_units+2*coefficient_cells)+
        32*deletion_units
    out.compressed_components.target_stratum_bytes =
        8*(6*target_strata+2*coefficient_cells)+32*target_strata
    out.compressed_components.cmg_hierarchy_bytes =
        512*(coefficient_cells+workers+firms)
    leverage_scratch = 8*leverage_batch*(
        6*deletion_units+4*coefficient_cells+3*parameters)
    target_scratch = 8*target_batch*(
        2*target_strata+8*coefficient_cells+6*parameters)
    out.compressed_components.phase_scratch_bytes =
        max((leverage_scratch,target_scratch))
    out.compressed_components.sorting_compression_bytes = 8*(
        18*n_rows+6*coefficient_cells+4*deletion_units+4*target_strata)
    out.compressed_components.solve_ahead_bytes = 0
    out.compressed_components.output_certificate_bytes = 8*(
        2*parameters+3*coefficient_cells+9*deletion_units+
        4*probes+6*(3*probes+1))
    out.compressed_components.preservation_transition_bytes =
        max((64*1024^2,0.05*raw_stata_bytes))

    persistent_coordinates = 8*(5*(workers+firms)+8*n_rows)
    out.generic_components.raw_stata_bytes = raw_stata_bytes
    out.generic_components.cell_bytes = 0
    out.generic_components.deletion_unit_bytes = 0
    out.generic_components.target_stratum_bytes = 0
    out.generic_components.cmg_hierarchy_bytes =
        max((persistent_coordinates,
            512*(coefficient_cells+workers+firms)))
    generic_batch = max((leverage_batch,target_batch))
    out.generic_components.phase_scratch_bytes =
        8*generic_batch*(14*n_rows+12*parameters+n_physical)
    out.generic_components.sorting_compression_bytes = 8*8*n_rows
    out.generic_components.solve_ahead_bytes = 0
    out.generic_components.output_certificate_bytes =
        out.compressed_components.output_certificate_bytes
    out.generic_components.preservation_transition_bytes = 0

    out.compressed = kssbc_resource__forecast(
        "compressed",out.compressed_components,
        out.compressed_wall_upper_seconds,0.30,
        hard_memory_bytes,hard_wall_seconds)
    out.generic = kssbc_resource__forecast(
        "generic",out.generic_components,out.generic_wall_upper_seconds,
        0.30,hard_memory_bytes,hard_wall_seconds)
    if (out.compressed.status == "INVALID_RESOURCE_FORECAST" |
        out.generic.status == "INVALID_RESOURCE_FORECAST") return(out)
    out.status = "MODELED"
    out.message =
        "KSS-PROD-1 anchored conservative resource forecasts constructed"
    return(out)
}

real rowvector kssbc_resource__forecast_vector(
    struct kssbc_resource_forecast scalar forecast)
{
    return((
        forecast.selection_peak_bytes,
        forecast.transition_peak_bytes,
        forecast.numerical_peak_bytes,
        forecast.restoration_peak_bytes,
        forecast.peak_bytes,
        forecast.memory_headroom_fraction,
        forecast.memory_admission_bytes,
        forecast.wall_forecast_upper_seconds,
        forecast.wall_headroom_fraction,
        forecast.wall_admission_seconds,
        forecast.hard_memory_bytes,
        forecast.hard_wall_seconds,
        forecast.memory_admitted,
        forecast.wall_admitted,
        forecast.admitted))
}

struct kssbc_resource_forecast scalar kssbc_resource__forecast_default(
    string scalar route,
    struct kssbc_resource_components scalar components,
    real scalar wall_forecast_upper_seconds)
{
    return(kssbc_resource__forecast(
        route,components,wall_forecast_upper_seconds,0.30))
}

struct kssbc_resource_selection scalar kssbc_resource__empty_selection()
{
    struct kssbc_resource_selection scalar out

    out.status = "INVALID_RESOURCE_SELECTION"
    out.message = "automatic resource selection input is invalid"
    out.route = ""
    out.compressed_status = ""
    out.generic_status = ""
    out.admitted = 0
    out.before_rng = 1
    out.compressed_eligible = .
    return(out)
}

// Automatic generic fallback is considered only when the compressed engine
// is scientifically ineligible.  It is never launched unless its own full
// raw-resident forecast passes both hard limits before estimator RNG.
struct kssbc_resource_selection scalar kssbc_resource__select_auto(
    real scalar compressed_eligible,
    struct kssbc_resource_forecast scalar compressed,
    struct kssbc_resource_forecast scalar generic)
{
    struct kssbc_resource_selection scalar out

    out = kssbc_resource__empty_selection()
    out.compressed_eligible = compressed_eligible
    out.compressed_status = compressed.status
    out.generic_status = generic.status
    if (!(compressed_eligible == 0 | compressed_eligible == 1) |
        compressed.route != "compressed" | generic.route != "generic" |
        compressed.before_rng != 1 | generic.before_rng != 1) {
        return(out)
    }

    if (compressed_eligible) {
        out.route = "compressed"
        out.status = compressed.status
        out.message = compressed.message
        out.admitted = compressed.admitted
        return(out)
    }

    out.route = "generic"
    out.status = generic.status
    out.message = generic.message
    out.admitted = generic.admitted
    return(out)
}

struct kssbc_resource_reconciliation scalar kssbc_resource__empty_recon()
{
    struct kssbc_resource_reconciliation scalar out

    out.status = "INVALID_RESOURCE_RECONCILIATION"
    out.message = "forecast reconciliation input is invalid"
    out.actual_phase_bytes = J(1,4,.)
    out.phase_forecast_ratios = J(1,4,.)
    out.process_peak_bytes = .
    out.qacct_maxvmem_bytes = .
    out.comparison_peak_bytes = .
    out.memory_forecast_ratio = .
    out.actual_wall_seconds = .
    out.wall_forecast_ratio = .
    out.within_phase_forecasts = 0
    out.within_memory_forecast = 0
    out.within_wall_forecast = 0
    out.within_hard_limits = 0
    out.next_scale_allowed = 0
    return(out)
}

// Reconciliation is deliberately fail-closed for scale progression.  Any
// phase, qacct/process peak, or wall time above its registered upper forecast
// requires the model to be updated and requalified before the next scale.
struct kssbc_resource_reconciliation scalar kssbc_resource__reconcile(
    struct kssbc_resource_forecast scalar forecast,
    real rowvector actual_phase_bytes,
    real scalar process_peak_bytes,
    real scalar qacct_maxvmem_bytes,
    real scalar actual_wall_seconds)
{
    struct kssbc_resource_reconciliation scalar out
    real rowvector forecast_phase_bytes

    out = kssbc_resource__empty_recon()
    if (cols(actual_phase_bytes) != 4 |
        rows(actual_phase_bytes) != 1 |
        missing(actual_phase_bytes) | min(actual_phase_bytes) < 0 |
        missing((process_peak_bytes,qacct_maxvmem_bytes,
                 actual_wall_seconds)) |
        process_peak_bytes < 0 | qacct_maxvmem_bytes < 0 |
        actual_wall_seconds <= 0 |
        missing(forecast.peak_bytes) | forecast.peak_bytes <= 0 |
        missing(forecast.wall_forecast_upper_seconds) |
        forecast.wall_forecast_upper_seconds <= 0) {
        return(out)
    }

    forecast_phase_bytes = (
        forecast.selection_peak_bytes,
        forecast.transition_peak_bytes,
        forecast.numerical_peak_bytes,
        forecast.restoration_peak_bytes)
    out.actual_phase_bytes = actual_phase_bytes
    out.phase_forecast_ratios = actual_phase_bytes:/forecast_phase_bytes
    out.process_peak_bytes = process_peak_bytes
    out.qacct_maxvmem_bytes = qacct_maxvmem_bytes
    out.comparison_peak_bytes = max((
        max(actual_phase_bytes),process_peak_bytes,qacct_maxvmem_bytes))
    out.memory_forecast_ratio =
        out.comparison_peak_bytes/forecast.peak_bytes
    out.actual_wall_seconds = actual_wall_seconds
    out.wall_forecast_ratio =
        actual_wall_seconds/forecast.wall_forecast_upper_seconds
    out.within_phase_forecasts =
        max(actual_phase_bytes:-forecast_phase_bytes) <= 0
    out.within_memory_forecast =
        out.comparison_peak_bytes <= forecast.peak_bytes
    out.within_wall_forecast =
        actual_wall_seconds <= forecast.wall_forecast_upper_seconds
    out.within_hard_limits =
        out.comparison_peak_bytes <= forecast.hard_memory_bytes &
        actual_wall_seconds <= forecast.hard_wall_seconds
    out.next_scale_allowed =
        out.within_phase_forecasts &
        out.within_memory_forecast &
        out.within_wall_forecast &
        out.within_hard_limits
    if (out.next_scale_allowed) {
        out.status = "RECONCILED"
        out.message = "actual resource use remains within registered forecasts"
    }
    else if (!out.within_hard_limits) {
        out.status = "ACTUAL_RESOURCE_LIMIT_EXCEEDED"
        out.message = "actual memory or wall time exceeds a hard limit"
    }
    else {
        out.status = "FORECAST_UNDERESTIMATED"
        out.message =
            "actual resource use exceeds a registered phase or route forecast"
    }
    return(out)
}

void kssbc_resource__stata_model(
    real scalar n_rows,
    real scalar n_physical,
    real scalar coefficient_cells,
    real scalar deletion_units,
    real scalar target_strata,
    real scalar workers,
    real scalar firms,
    real scalar parameters,
    real scalar probes,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar leverage_rng_calls_per_probe,
    real scalar target_rng_calls_per_probe,
    real scalar rng_call_seconds_upper,
    real scalar raw_stata_bytes,
    real scalar hard_memory_bytes,
    real scalar hard_wall_seconds,
    string scalar components_name,
    string scalar forecasts_name,
    string scalar scaling_name,
    string scalar status_local,
    string scalar message_local,
    string scalar compressed_status_local,
    string scalar compressed_message_local,
    string scalar generic_status_local,
    string scalar generic_message_local,
    string scalar compressed_phase_local,
    string scalar generic_phase_local)
{
    struct kssbc_resource_model scalar out

    out = kssbc_resource__model(
        n_rows,n_physical,coefficient_cells,deletion_units,target_strata,
        workers,firms,parameters,probes,leverage_batch,target_batch,
        leverage_rng_calls_per_probe,target_rng_calls_per_probe,
        rng_call_seconds_upper,
        raw_stata_bytes,hard_memory_bytes,hard_wall_seconds)
    st_matrix(components_name,
        kssbc_resource__component_vector(out.compressed_components) \
        kssbc_resource__component_vector(out.generic_components))
    st_matrix(forecasts_name,
        kssbc_resource__forecast_vector(out.compressed) \
        kssbc_resource__forecast_vector(out.generic))
    st_matrix(scaling_name,(
        out.row_scale,out.structure_scale,
        out.compressed_wall_upper_seconds,out.generic_wall_upper_seconds,
        out.physical_scale,out.n_physical,
        out.leverage_rng_calls_per_probe,
        out.target_rng_calls_per_probe,out.rng_call_seconds_upper,
        out.rng_total_calls,out.rng_wall_upper_seconds))
    st_local(status_local,out.status)
    st_local(message_local,out.message)
    st_local(compressed_status_local,out.compressed.status)
    st_local(compressed_message_local,out.compressed.message)
    st_local(generic_status_local,out.generic.status)
    st_local(generic_message_local,out.generic.message)
    st_local(compressed_phase_local,out.compressed.peak_phase)
    st_local(generic_phase_local,out.generic.peak_phase)
}

void kssbc_resource__stata_route(
    string scalar route,
    string scalar components_name,
    real scalar component_row,
    real scalar wall_forecast_upper_seconds,
    real scalar hard_memory_bytes,
    real scalar hard_wall_seconds,
    real scalar routed_solver_peak_bytes,
    string scalar forecast_name,
    string scalar status_local,
    string scalar message_local,
    string scalar phase_local)
{
    struct kssbc_resource_components scalar components
    struct kssbc_resource_forecast scalar forecast
    real matrix registered_components
    real scalar valid_component_row

    registered_components = st_matrix(components_name)
    valid_component_row =
        !missing(component_row) & component_row >= 1 &
        component_row == floor(component_row) &
        component_row <= rows(registered_components) &
        cols(registered_components) == 10
    if (!valid_component_row) {
        components = kssbc_resource__components_from(J(1,10,.))
    }
    else components = kssbc_resource__components_from(
        registered_components[component_row,.])
    forecast = kssbc_resource__forecast(
        route,components,wall_forecast_upper_seconds,0.30,
        hard_memory_bytes,hard_wall_seconds)
    forecast = kssbc_resource__route_reconcile(
        forecast,routed_solver_peak_bytes)
    if (valid_component_row & forecast.route_reconciled) {
        registered_components[component_row,.] = forecast.component_bytes
        st_matrix(components_name,registered_components)
    }
    st_matrix(forecast_name,kssbc_resource__forecast_vector(forecast))
    st_local(status_local,forecast.status)
    st_local(message_local,forecast.message)
    st_local(phase_local,forecast.peak_phase)
}

end
