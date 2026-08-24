*! vckss compressed estimator engine 0.4.0-dev 18aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum off

/*
This file is the numerical, no-control, match-deletion KSS-SCALE fast path.
It consumes a prepared vckss_scale_design.  The retained Stata rows are not
needed after that object has been built.

Randomness is deliberately injected.  A provider returns, for an inclusive
logical probe range, either

  * one literal-copy sign sum per deletion unit (leverage domain), or
  * one literal-copy sign sum per exact target-scale stratum (target domain).

The provider, rather than this engine, owns the registered RNG contract.  A
matrix-backed provider below exists for deterministic oracle tests.  It is
not a production RNG and creates no random atoms.
*/

string scalar vckss_scale_engine__version()
{
    return("0.2.0-experimental")
}

real scalar vckss_scale_engine__api_level()
{
    return(4)
}

string scalar vckss_scale_engine__build_id()
{
    return("vckss-scale-engine-api4-fe-buf1-buffered")
}

struct vckss_scale_engine_atom_batch
{
    string scalar status
    string scalar message
    real matrix value
}

struct vckss_scale_atom_provider
{
    string scalar status
    string scalar message
    string scalar contract
    real scalar certified_trials
    real colvector leverage_trials
    real colvector target_trials
    pointer scalar context
    pointer scalar leverage
    pointer scalar target
}

struct vckss_scale_engine_matrix_atoms
{
    real matrix leverage_unit
    real matrix target_stratum
}

struct vckss_scale_rng_context
{
    string scalar status
    string scalar message
    string scalar candidate
    real scalar master_seed
    real colvector unit_rank
    real colvector unit_trials
    real colvector stratum_rank
    real colvector stratum_trials
    real scalar leverage_initialized
    real scalar target_initialized
    struct vckss_rng__cursor scalar leverage_cursor
    struct vckss_rng__cursor scalar target_cursor
}

/* A scatter plan is immutable numerical preparation state.  It records the
   stable order/panel already certified by scale preparation, so execution
   can reduce new RHS matrices without sorting the same group map again. */
struct vckss_scatter_plan
{
    string scalar status
    string scalar message
    real scalar rows
    real scalar groups
    real scalar identity
    real colvector first_group
    real colvector row_order
    real matrix panel
}

struct vckss_scale_unit_adjust
{
    string scalar status
    string scalar message
    real scalar projection_share
    real scalar residual_share
    real scalar finite_bias
    real scalar finite_variance
    real scalar residual_mass
    real scalar deleted_mass
    real scalar reciprocal_residual
}

struct vckss_scale_unit_adjust_batch
{
    string scalar status
    string scalar message
    real colvector deleted_mass
    real scalar max_reciprocal_residual
}

struct vckss_scale_engine_result
{
    string scalar status
    string scalar message
    string scalar rng_contract
    string scalar solver_route

    real rowvector plugin
    real rowvector correction
    real rowvector corrected
    real rowvector numerical_mcse
    real matrix target_draws

    real colvector coefficient
    real colvector fitted_cell
    real colvector unit_projection_share
    real colvector unit_residual_share
    real colvector unit_finite_bias
    real colvector unit_finite_variance
    real colvector unit_residual_mass
    real colvector unit_d
    real colvector cell_correction_weight

    real scalar n_stored
    real scalar n_physical
    real scalar coefficient_cells
    real scalar deletion_units
    real scalar target_strata
    real scalar worker_levels
    real scalar firm_levels
    real scalar probes
    real scalar target_weight_sum
    real scalar weighted_rss
    real scalar max_leverage
    real scalar max_reciprocal_residual
    real scalar max_complete_residual
    real scalar residual_gate
    real scalar target_identity_residual
    real scalar preconditioner_ratio

    real scalar fit_seconds
    real scalar rng_seconds
    real scalar leverage_seconds
    real scalar correction_seconds
    real scalar target_seconds
    real scalar total_seconds
    real scalar solver_iterations
    real scalar solver_schur_actions
    real scalar solver_schur_batches
    real scalar solver_precond_applications
    real scalar solver_precond_batches
    real scalar solver_schur_seconds
    real scalar solver_precond_seconds
    real scalar solver_pcg_seconds
    real matrix solver_rhs_diagnostics
    real scalar fe_workspace_applicable
    real scalar fe_workspace_builds
    real scalar fe_buffered_schur_batches
    real scalar fe_legacy_schur_batches
    real scalar fe_buffered_schur_columns
    real scalar fe_legacy_schur_columns
    real scalar fe_packed_fallback_batches
    real scalar fe_max_buffer_width
    real scalar fe_workspace_peak_bytes
    real scalar fe_cell_bytes_avoided
}

/* The route context is numerical only.  Stata data lifecycle, eligibility,
   and resource admission stay outside this engine.  A routed solver can
   prepare the cell FE design and backend once, then invoke the callback
   below without reconstructing either object. */
struct vckss_scale_route_context
{
    string scalar status
    string scalar message
    pointer(struct vckss_scale_design scalar) scalar design
    struct vckss_scale_atom_provider scalar provider
    real scalar probes
    real scalar leverage_batch
    real scalar target_batch
    real scalar tolerance
    real scalar maxiter
    real scalar rank_tolerance
    real scalar block_tolerance
    struct vckss_scale_engine_result scalar last
}

struct vckss_scale_engine_atom_batch scalar vckss_scale_eng__empty_atoms()
{
    struct vckss_scale_engine_atom_batch scalar out

    out.status = "INVALID_PROBE_PROVIDER"
    out.message = "the probe provider returned no atoms"
    out.value = J(0,0,.)
    return(out)
}

struct vckss_scale_engine_result scalar vckss_scale_eng__empty_result()
{
    struct vckss_scale_engine_result scalar out

    out.status = "INVALID_INPUT"
    out.message = "invalid compressed estimator input"
    out.rng_contract = ""
    out.solver_route = ""
    out.plugin = J(1,4,.)
    out.correction = J(1,4,.)
    out.corrected = J(1,4,.)
    out.numerical_mcse = J(1,4,.)
    out.target_draws = J(0,4,.)
    out.coefficient = J(0,1,.)
    out.fitted_cell = J(0,1,.)
    out.unit_projection_share = J(0,1,.)
    out.unit_residual_share = J(0,1,.)
    out.unit_finite_bias = J(0,1,.)
    out.unit_finite_variance = J(0,1,.)
    out.unit_residual_mass = J(0,1,.)
    out.unit_d = J(0,1,.)
    out.cell_correction_weight = J(0,1,.)
    out.n_stored = .
    out.n_physical = .
    out.coefficient_cells = .
    out.deletion_units = .
    out.target_strata = .
    out.worker_levels = .
    out.firm_levels = .
    out.probes = .
    out.target_weight_sum = .
    out.weighted_rss = .
    out.max_leverage = .
    out.max_reciprocal_residual = .
    out.max_complete_residual = .
    out.residual_gate = .
    out.target_identity_residual = .
    out.preconditioner_ratio = .
    out.fit_seconds = .
    out.rng_seconds = .
    out.leverage_seconds = .
    out.correction_seconds = .
    out.target_seconds = .
    out.total_seconds = .
    out.solver_iterations = 0
    out.solver_schur_actions = 0
    out.solver_schur_batches = 0
    out.solver_precond_applications = 0
    out.solver_precond_batches = 0
    out.solver_schur_seconds = 0
    out.solver_precond_seconds = 0
    out.solver_pcg_seconds = 0
    out.solver_rhs_diagnostics = J(0,6,.)
    out.fe_workspace_applicable = 1
    out.fe_workspace_builds = 0
    out.fe_buffered_schur_batches = 0
    out.fe_legacy_schur_batches = 0
    out.fe_buffered_schur_columns = 0
    out.fe_legacy_schur_columns = 0
    out.fe_packed_fallback_batches = 0
    out.fe_max_buffer_width = 0
    out.fe_workspace_peak_bytes = 0
    out.fe_cell_bytes_avoided = 0
    return(out)
}

struct vckss_scale_engine_atom_batch scalar vckss_scale_eng__mat_leverage(
    pointer scalar context,
    real scalar first_probe,
    real scalar last_probe)
{
    struct vckss_scale_engine_atom_batch scalar out
    pointer(struct vckss_scale_engine_matrix_atoms scalar) scalar source

    out = vckss_scale_eng__empty_atoms()
    source = context
    if (context == NULL | first_probe < 1 |
        last_probe < first_probe | first_probe != floor(first_probe) |
        last_probe != floor(last_probe) |
        last_probe > cols((*source).leverage_unit)) return(out)
    out.value = (*source).leverage_unit[.,first_probe..last_probe]
    if (rows(out.value) == 0 | hasmissing(out.value)) return(out)
    out.status = "CONVERGED"
    out.message = "matrix-backed deletion-unit atoms returned"
    return(out)
}

struct vckss_scale_engine_atom_batch scalar vckss_scale_eng__mat_target(
    pointer scalar context,
    real scalar first_probe,
    real scalar last_probe)
{
    struct vckss_scale_engine_atom_batch scalar out
    pointer(struct vckss_scale_engine_matrix_atoms scalar) scalar source

    out = vckss_scale_eng__empty_atoms()
    source = context
    if (context == NULL | first_probe < 1 |
        last_probe < first_probe | first_probe != floor(first_probe) |
        last_probe != floor(last_probe) |
        last_probe > cols((*source).target_stratum)) return(out)
    out.value = (*source).target_stratum[.,first_probe..last_probe]
    if (rows(out.value) == 0 | hasmissing(out.value)) return(out)
    out.status = "CONVERGED"
    out.message = "matrix-backed target-stratum atoms returned"
    return(out)
}

struct vckss_scale_atom_provider scalar vckss_scale_eng__mat_provider(
    pointer(struct vckss_scale_engine_matrix_atoms scalar) scalar source)
{
    struct vckss_scale_atom_provider scalar out

    out.status = "INVALID_PROBE_PROVIDER"
    out.message = "matrix-backed atom source is unavailable"
    out.contract = "TEST_MATRIX_ATOMS"
    out.certified_trials = 0
    out.leverage_trials = J(0,1,.)
    out.target_trials = J(0,1,.)
    out.context = source
    out.leverage = &vckss_scale_eng__mat_leverage()
    out.target = &vckss_scale_eng__mat_target()
    if (source != NULL) {
        out.status = "CONVERGED"
        out.message = "matrix-backed test atom provider prepared"
    }
    return(out)
}

struct vckss_scale_rng_context scalar vckss_scale_eng__rng_context(
    string scalar candidate,
    real scalar master_seed,
    real colvector unit_rank,
    real colvector unit_trials,
    real colvector stratum_rank,
    real colvector stratum_trials)
{
    struct vckss_scale_rng_context scalar out

    out.status = "INVALID_PROBE_PROVIDER"
    out.message = "registered RNG provider input is invalid"
    out.candidate = candidate
    out.master_seed = master_seed
    out.unit_rank = unit_rank
    out.unit_trials = unit_trials
    out.stratum_rank = stratum_rank
    out.stratum_trials = stratum_trials
    out.leverage_initialized = 0
    out.target_initialized = 0
    out.leverage_cursor.status = "PENDING"
    out.leverage_cursor.next_probe = 1
    out.target_cursor.status = "PENDING"
    out.target_cursor.next_probe = 1
    if (vckss_rng__api_level() != 4 |
        vckss_rng__production_contract() == "") {
        out.status = "RNG_RUNTIME_UNREGISTERED"
        out.message = "runtime has no registered production RNG contract"
        return(out)
    }
    if (candidate != vckss_rng__k1_recommendation() |
        missing(master_seed) | master_seed < 0 |
        master_seed > 2147483647 | master_seed != floor(master_seed) |
        rows(unit_rank) != rows(unit_trials) |
        rows(stratum_rank) != rows(stratum_trials) |
        rows(vckss_rng__canonical_order(unit_rank)) != rows(unit_rank) |
        rows(vckss_rng__canonical_order(stratum_rank)) != rows(stratum_rank) |
        !vckss_rng__trials_ok(unit_trials) |
        !vckss_rng__trials_ok(stratum_trials)) return(out)
    out.status = "CONVERGED"
    out.message = "registered RNG cursor inputs validated; cursors are lazy"
    return(out)
}

struct vckss_scale_engine_atom_batch scalar vckss_scale_eng__rng_lev(
    pointer scalar context,
    real scalar first_probe,
    real scalar last_probe)
{
    struct vckss_scale_engine_atom_batch scalar out
    struct vckss_rng__result scalar generated
    pointer(struct vckss_scale_rng_context scalar) scalar source

    out = vckss_scale_eng__empty_atoms()
    source = context
    if (context == NULL | (*source).status != "CONVERGED" |
        first_probe < 1 | last_probe < first_probe |
        first_probe != floor(first_probe) |
        last_probe != floor(last_probe)) return(out)
    if (!(*source).leverage_initialized) {
        if (first_probe != 1) {
            out.status = "RNG_SEQUENCE_MISMATCH"
            out.message = "leverage cursor must begin at logical probe one"
            return(out)
        }
        (*source).leverage_cursor = vckss_rng__open_cursor(
            (*source).master_seed,"leverage",(*source).unit_rank,
            (*source).unit_trials)
        if ((*source).leverage_cursor.status != "OK") {
            out.status = (*source).leverage_cursor.status
            out.message = (*source).leverage_cursor.message
            return(out)
        }
        (*source).unit_rank = J(0,1,.)
        (*source).leverage_initialized = 1
    }
    if (first_probe != (*source).leverage_cursor.next_probe) {
        out.status = "RNG_SEQUENCE_MISMATCH"
        out.message = "leverage cursor requires a contiguous logical probe range"
        return(out)
    }
    generated = vckss_rng__cursor_next(
        &((*source).leverage_cursor),last_probe-first_probe+1)
    if (generated.status != "OK") {
        out.status = generated.status
        out.message = generated.message
        return(out)
    }
    out.value = J(rows((*source).unit_trials),cols(generated.atoms),.)
    out.value[generated.canonical_order,.] = generated.atoms
    if (hasmissing(out.value)) {
        out.status = "RNG_SEMANTIC_KEY_INVALID"
        out.message = "leverage atoms cannot be restored to design order"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "registered deletion-unit atoms returned"
    return(out)
}

struct vckss_scale_engine_atom_batch scalar vckss_scale_eng__rng_target(
    pointer scalar context,
    real scalar first_probe,
    real scalar last_probe)
{
    struct vckss_scale_engine_atom_batch scalar out
    struct vckss_rng__result scalar generated
    pointer(struct vckss_scale_rng_context scalar) scalar source

    out = vckss_scale_eng__empty_atoms()
    source = context
    if (context == NULL | (*source).status != "CONVERGED" |
        first_probe < 1 | last_probe < first_probe |
        first_probe != floor(first_probe) |
        last_probe != floor(last_probe)) return(out)
    if (!(*source).target_initialized) {
        if (first_probe != 1) {
            out.status = "RNG_SEQUENCE_MISMATCH"
            out.message = "target cursor must begin at logical probe one"
            return(out)
        }
        (*source).target_cursor = vckss_rng__open_cursor(
            (*source).master_seed,"target",(*source).stratum_rank,
            (*source).stratum_trials)
        if ((*source).target_cursor.status != "OK") {
            out.status = (*source).target_cursor.status
            out.message = (*source).target_cursor.message
            return(out)
        }
        (*source).stratum_rank = J(0,1,.)
        (*source).target_initialized = 1
    }
    if (first_probe != (*source).target_cursor.next_probe) {
        out.status = "RNG_SEQUENCE_MISMATCH"
        out.message = "target cursor requires a contiguous logical probe range"
        return(out)
    }
    generated = vckss_rng__cursor_next(
        &((*source).target_cursor),last_probe-first_probe+1)
    if (generated.status != "OK") {
        out.status = generated.status
        out.message = generated.message
        return(out)
    }
    out.value = J(rows((*source).stratum_trials),cols(generated.atoms),.)
    out.value[generated.canonical_order,.] = generated.atoms
    if (hasmissing(out.value)) {
        out.status = "RNG_SEMANTIC_KEY_INVALID"
        out.message = "target atoms cannot be restored to design order"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "registered target-stratum atoms returned"
    return(out)
}

struct vckss_scale_atom_provider scalar vckss_scale_eng__rng_provider(
    pointer(struct vckss_scale_rng_context scalar) scalar source)
{
    struct vckss_scale_atom_provider scalar out

    out.status = "INVALID_PROBE_PROVIDER"
    out.message = "registered RNG context is unavailable"
    out.contract = ""
    out.certified_trials = 0
    out.leverage_trials = J(0,1,.)
    out.target_trials = J(0,1,.)
    out.context = source
    out.leverage = &vckss_scale_eng__rng_lev()
    out.target = &vckss_scale_eng__rng_target()
    if (source != NULL & (*source).status == "CONVERGED") {
        out.contract = vckss_rng__production_contract()
        out.leverage_trials = (*source).unit_trials
        out.target_trials = (*source).stratum_trials
        if (out.contract != "") {
            out.certified_trials = 1
            out.status = "CONVERGED"
            out.message = "stateful registered RNG provider prepared"
        }
    }
    else if (source != NULL) {
        out.status = (*source).status
        out.message = (*source).message
    }
    if (vckss_rng__production_contract() == "") {
        out.status = "RNG_RUNTIME_UNREGISTERED"
        out.message = "runtime has no registered production RNG contract"
    }
    return(out)
}

/* Stable scatter-add in semantic input order.  The group vector is dense and
   one based. */
struct vckss_scatter_plan scalar vckss_scale_eng__scatter_plan(
    real colvector group,
    real scalar groups,
    real colvector row_order,
    real matrix panel)
{
    struct vckss_scatter_plan scalar out

    out.status = "INVALID_SCATTER_PLAN"
    out.message = "scatter plan dimensions or dense group map are invalid"
    out.rows = rows(group)
    out.groups = groups
    out.identity = 0
    out.first_group = J(0,1,.)
    out.row_order = J(0,1,.)
    out.panel = J(0,2,.)

    if (out.rows == 0 | groups < 1 | groups != floor(groups) |
        rows(row_order) != out.rows | cols(row_order) != 1 |
        rows(panel) != groups | cols(panel) != 2 |
        hasmissing(group) | hasmissing(row_order) | hasmissing(panel) |
        min(group) < 1 | max(group) > groups |
        max(abs(group-floor(group))) != 0 |
        min(row_order) < 1 | max(row_order) > out.rows |
        max(abs(row_order-floor(row_order))) != 0) return(out)
    out.first_group = group[row_order[panel[.,1]]]
    if (rows(out.first_group) != groups |
        max(abs(sort(out.first_group,1)-(1::groups))) != 0) return(out)
    if (out.rows == groups) out.identity = all(group :== (1::groups))
    out.row_order = row_order
    out.panel = panel
    out.status = "CONVERGED"
    out.message = "stable scatter plan prepared"
    return(out)
}

void vckss_scale_eng__scatter_into(
    real matrix values,
    struct vckss_scatter_plan scalar plan,
    pointer(real matrix) scalar destination)
{
    if (plan.status != "CONVERGED" | rows(values) != plan.rows |
        cols(values) == 0 | hasmissing(values)) {
        (*destination) = J(0,0,.)
        return
    }
    if (plan.identity) {
        (*destination) = values
        return
    }
    (*destination) = J(plan.groups,cols(values),0)
    (*destination)[plan.first_group,.] = vckss_scale__stable_groupsum(
        values,plan.row_order,plan.panel)
}

real matrix vckss_scale_eng__scatter_planned(
    real matrix values,
    struct vckss_scatter_plan scalar plan)
{
    real matrix out

    vckss_scale_eng__scatter_into(values,plan,&out)
    return(out)
}

/* Compatibility wrapper for callers that do not yet retain preparation
   state.  Production PREP-RHS-1 paths use vckss_scale_eng__scatter_planned(). */
real matrix vckss_scale_engine__scatter_sum(
    real matrix values,
    real colvector group,
    real scalar groups)
{
    struct vckss_scatter_plan scalar plan
    real colvector row_order
    real matrix panel

    if (rows(values) == 0 | cols(values) == 0 |
        rows(group) != rows(values) | groups < 1 |
        groups != floor(groups) | hasmissing(values) | hasmissing(group) |
        min(group) < 1 | max(group) > groups |
        max(abs(group-floor(group))) != 0) return(J(0,0,.))
    row_order = order(group,1)
    panel = panelsetup(group[row_order],1)
    plan = vckss_scale_eng__scatter_plan(group,groups,row_order,panel)
    return(vckss_scale_eng__scatter_planned(values,plan))
}

real rowvector vckss_scale_engine__column_sum(real matrix values)
{
    if (rows(values) == 0 | cols(values) == 0 |
        hasmissing(values)) return(J(1,0,.))
    return(quadcolsum(values))
}

/* Apply the mathematically exact target centering projection.  Compatibility
   of its separately solved worker and firm score blocks is repaired below;
   no near-null direction is classified as zero here. */
real matrix vckss_scale_eng__center_target(
    real matrix first,
    real colvector mass_share)
{
    real matrix out
    real rowvector total

    if (rows(first) == 0 | cols(first) == 0 |
        rows(mass_share) != rows(first) | cols(mass_share) != 1 |
        hasmissing(first) | hasmissing(mass_share) |
        min(mass_share) < 0) return(J(0,0,.))
    total = vckss_scale_engine__column_sum(first)
    out = first-mass_share*total
    return(out)
}

/* A centered direction has zero-sum worker and firm scores in exact
   arithmetic.  Form the final coordinate from all preceding coordinates so
   the supplied full RHS is compatible by construction.  The change must be
   only registered regrouping roundoff relative to the uncentered draw. */
real matrix vckss_scale_eng__balance_score(
    real matrix score,
    real rowvector reference_scale)
{
    real scalar column, original_last
    real matrix out
    real rowvector preceding

    if (rows(score) == 0 | cols(score) == 0 |
        cols(reference_scale) != cols(score) |
        hasmissing(score) | hasmissing(reference_scale) |
        min(reference_scale) < 0) return(J(0,0,.))
    out = score
    if (rows(out) > 1) preceding =
        vckss_scale_engine__column_sum(out[1..(rows(out)-1),.])
    for (column=1; column<=cols(score); column++) {
        original_last = out[rows(out),column]
        if (rows(out) == 1) out[1,column] = 0
        else out[rows(out),column] = -preceding[column]
        if ((reference_scale[column] == 0 &
             out[rows(out),column] != original_last) |
            (reference_scale[column] > 0 &
             abs(out[rows(out),column]-original_last) >
                vckss_scale_eng__roundoff_gate()*
                reference_scale[column])) return(J(0,0,.))
    }
    return(out)
}

/* Numerical acceptance here is a floating-point regrouping contract, not a
   statistical-rank decision.  Normalize aggregate discrepancies by an
   absolute-mass scale and use one fixed multiple of binary64 roundoff. */
real scalar vckss_scale_eng__roundoff_gate()
{
    return(4096*2.2204460492503131e-16)
}

real scalar vckss_scale_eng__aggregate_ok(
    real colvector reconstructed,
    real colvector reference,
    real colvector absolute_mass)
{
    real colvector scale

    if (rows(reconstructed) == 0 | cols(reconstructed) != 1 |
        rows(reference) != rows(reconstructed) |
        rows(absolute_mass) != rows(reconstructed) |
        cols(reference) != 1 | cols(absolute_mass) != 1 |
        hasmissing(reconstructed) | hasmissing(reference) |
        hasmissing(absolute_mass) | min(absolute_mass) < 0) return(0)
    scale = rowmax((J(rows(reference),1,1),abs(reconstructed),
        abs(reference),absolute_mass))
    return(max(abs(reconstructed-reference):/scale) <=
        vckss_scale_eng__roundoff_gate())
}

/* Merge one contiguous logical tile into a matrix accumulator.  Pointers
   ensure the vectorized Neumaier update changes the caller's full matrices;
   unlike the retired scalar loop, no indexed scalar expression is passed. */
void vckss_scale_eng__merge(
    pointer(real matrix) scalar subtotal,
    pointer(real matrix) scalar compensation,
    real matrix values)
{
    real matrix correction, larger, updated

    updated = (*subtotal)+values
    larger = abs((*subtotal)):>=abs(values)
    correction = larger:*(((*subtotal)-updated)+values) +
        (1:-larger):*((values-updated)+(*subtotal))
    (*compensation) = (*compensation)+correction
    (*subtotal) = updated
}

/* Column-at-a-time form for the G x 5 moment accumulator.  It retains
   vector arithmetic while avoiding a G x 5 contribution and three G x 5
   merge temporaries at every tile. */
void vckss_scale_eng__merge_col(
    pointer(real matrix) scalar subtotal,
    pointer(real matrix) scalar compensation,
    real scalar column,
    real colvector values)
{
    real colvector correction, larger, old, updated

    old = (*subtotal)[.,column]
    updated = old+values
    larger = abs(old):>=abs(values)
    correction = larger:*((old-updated)+values) +
        (1:-larger):*((values-updated)+old)
    (*compensation)[.,column] =
        (*compensation)[.,column]+correction
    (*subtotal)[.,column] = updated
}

real scalar vckss_scale_eng__moment_width()
{
    /* Bound the extra square/product scratch independently of solve width. */
    return(8)
}

/* Add five match moments matrix-wide over fixed, contiguous probe tiles.
   The G x 5 accumulator is independent of total probe width in its column
   dimension and retains logical probe order across solver batches. */
void vckss_scale_eng__moment_add(
    pointer(real matrix) scalar subtotal,
    pointer(real matrix) scalar compensation,
    real matrix projection,
    real matrix residual)
{
    real scalar first, last, width
    real matrix projection_square, residual_square
    real colvector values

    if (rows(projection) == 0 | cols(projection) == 0 |
        rows(residual) != rows(projection) |
        cols(residual) != cols(projection) |
        rows((*subtotal)) != rows(projection) |
        cols((*subtotal)) != 5 |
        rows((*compensation)) != rows(projection) |
        cols((*compensation)) != 5) {
        (*subtotal) = J(0,0,.)
        (*compensation) = J(0,0,.)
        return
    }
    width = vckss_scale_eng__moment_width()
    for (first=1; first<=cols(projection); first=first+width) {
        last = min((cols(projection),first+width-1))
        projection_square = projection[.,first..last]:^2
        residual_square = residual[.,first..last]:^2
        values = rowsum(projection_square)
        vckss_scale_eng__merge_col(
            subtotal,compensation,1,values)
        values = rowsum(residual_square)
        vckss_scale_eng__merge_col(
            subtotal,compensation,2,values)
        values = rowsum(projection_square:*residual_square)
        vckss_scale_eng__merge_col(
            subtotal,compensation,5,values)
        projection_square = projection_square:^2
        residual_square = residual_square:^2
        values = rowsum(projection_square)
        vckss_scale_eng__merge_col(
            subtotal,compensation,3,values)
        values = rowsum(residual_square)
        vckss_scale_eng__merge_col(
            subtotal,compensation,4,values)
    }
}

/* Convenience wrapper retained for direct equality and performance tests. */
real matrix vckss_scale_eng__moment_reduce(
    real matrix projection,
    real matrix residual)
{
    real matrix compensation, subtotal

    if (rows(projection) == 0 | cols(projection) == 0 |
        rows(residual) != rows(projection) |
        cols(residual) != cols(projection)) return(J(0,0,.))
    subtotal = J(rows(projection),5,0)
    compensation = J(rows(projection),5,0)
    vckss_scale_eng__moment_add(
        &subtotal,&compensation,projection,residual)
    return(subtotal+compensation)
}

real scalar vckss_scale_eng__target_rows()
{
    /* At B=32 this keeps each additional dense tile near one MiB. */
    return(4096)
}

/* Contract all worker, firm, and cross columns in one bounded cell-tile
   traversal.  Column pairs in prediction are (worker, firm).  Total is
   derived from the three contractions, avoiding a third row-sized total
   prediction and enforcing the accounting identity by construction. */
real matrix vckss_scale_eng__target_contract(
    real colvector cell_weight,
    real matrix prediction)
{
    real scalar batches, first, last, width
    real rowvector firm_columns, worker_columns
    real matrix compensation, firm, moment, subtotal, weighted_firm
    real matrix weighted_worker, worker, components, out

    if (rows(cell_weight) == 0 | cols(cell_weight) != 1 |
        rows(prediction) != rows(cell_weight) | cols(prediction) == 0 |
        mod(cols(prediction),2) != 0) return(J(0,0,.))
    batches = cols(prediction)/2
    worker_columns = 2:*(1..batches):-1
    firm_columns = 2:*(1..batches)
    subtotal = J(3,batches,0)
    compensation = J(3,batches,0)
    width = vckss_scale_eng__target_rows()
    for (first=1; first<=rows(prediction); first=first+width) {
        last = min((rows(prediction),first+width-1))
        worker = prediction[first..last,worker_columns]
        firm = prediction[first..last,firm_columns]
        weighted_worker = cell_weight[first..last]:*worker
        weighted_firm = cell_weight[first..last]:*firm
        moment = (quadcolsum(weighted_worker:*worker) \
            quadcolsum(weighted_firm:*firm) \
            quadcolsum(weighted_worker:*firm))
        vckss_scale_eng__merge(
            &subtotal,&compensation,moment)
    }
    components = (subtotal+compensation)'
    out = J(batches,4,.)
    out[.,1..3] = components
    out[.,4] = out[.,1]+out[.,2]+2:*out[.,3]
    return(out)
}

real scalar vckss_scale_engine__dot(
    real colvector left,
    real colvector right)
{
    if (rows(left) == 0 | cols(left) != 1 |
        rows(right) != rows(left) | cols(right) != 1 |
        hasmissing(left) | hasmissing(right)) return(.)
    return(quadcross(left,right))
}

real scalar vckss_scale_eng__atom_rows()
{
    return(4096)
}

real scalar vckss_scale_eng__valid_atoms(
    real matrix atoms,
    real colvector trials)
{
    real scalar first, last, width
    real matrix tile
    real colvector tile_trials

    if (rows(atoms) == 0 | cols(atoms) == 0 |
        rows(trials) != rows(atoms) | cols(trials) != 1 |
        hasmissing(trials) | min(trials) < 1 |
        max(abs(trials-floor(trials))) != 0) return(0)
    /* Validate all columns matrix-wide in bounded row tiles.  Comparing the
       parity of absolute values avoids forming value+trials, whose exact
       integer range would be narrower even though sign-sum support is not. */
    width = vckss_scale_eng__atom_rows()
    for (first=1; first<=rows(atoms); first=first+width) {
        last = min((rows(atoms),first+width-1))
        tile = atoms[first..last,.]
        tile_trials = trials[first..last]
        if (hasmissing(tile) |
            max(abs(tile-floor(tile))) != 0 |
            max(abs(tile):-tile_trials) > 0 |
            max(abs(mod(abs(tile),2):-mod(tile_trials,2))) != 0) {
            return(0)
        }
    }
    return(1)
}

real rowvector vckss_scale_engine__plugin(
    struct vckss_scale_design scalar design,
    struct vckss_fe_design scalar base,
    real colvector coefficient)
{
    real colvector alpha, gamma, worker_effect, firm_effect
    real scalar covariance, firm_mean, firm_variance, mass
    real scalar worker_mean, worker_variance

    mass = design.target_weight_sum
    alpha = coefficient[1..design.worker_levels]
    gamma = coefficient[(design.worker_levels+1)..rows(coefficient)] \ 0
    worker_effect = alpha[design.cell_worker]
    firm_effect = gamma[design.cell_firm]
    worker_mean = vckss_scale_engine__dot(
        design.cell_target_mass,worker_effect)/mass
    firm_mean = vckss_scale_engine__dot(
        design.cell_target_mass,firm_effect)/mass
    worker_variance = vckss_scale_engine__dot(
        design.cell_target_mass,(worker_effect:-worker_mean):^2)/mass
    firm_variance = vckss_scale_engine__dot(
        design.cell_target_mass,(firm_effect:-firm_mean):^2)/mass
    covariance = vckss_scale_engine__dot(
        design.cell_target_mass,
        (worker_effect:-worker_mean):*(firm_effect:-firm_mean))/mass
    return((worker_variance,firm_variance,covariance,
        worker_variance+firm_variance+2*covariance))
}

real rowvector vckss_scale_engine__mcse(real matrix draws)
{
    real scalar column, correction, one, probe, subtotal, updated
    real rowvector mean_draw, out

    if (rows(draws) < 2 | cols(draws) == 0 |
        hasmissing(draws)) return(J(1,cols(draws),.))
    mean_draw = vckss_scale_engine__column_sum(draws):/rows(draws)
    out = J(1,cols(draws),.)
    for (column=1; column<=cols(draws); column++) {
        subtotal = 0
        correction = 0
        for (probe=1; probe<=rows(draws); probe++) {
            one = (draws[probe,column]-mean_draw[column])^2
            updated = subtotal+one
            if (abs(subtotal) >= abs(one)) {
                correction = correction+(subtotal-updated)+one
            }
            else correction = correction+(one-updated)+subtotal
            subtotal = updated
        }
        out[column] = sqrt((subtotal+correction)/
            (rows(draws)-1)/rows(draws))
    }
    return(out)
}

real scalar vckss_scale_eng__identity_resid(
    real matrix values)
{
    real colvector residual
    real scalar scale

    if (rows(values) == 0 | cols(values) != 4 |
        hasmissing(values)) return(.)
    residual = values[.,4]-values[.,1]-values[.,2]-2:*values[.,3]
    scale = max((1,max(abs(values))))
    return(max(abs(residual))/scale)
}

struct vckss_scale_unit_adjust scalar vckss_scale_eng__unit_adjust(
    real scalar projection_share,
    real scalar residual_share,
    real scalar finite_bias,
    real scalar finite_variance,
    real scalar residual_mass,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_scale_unit_adjust scalar out
    real scalar maker_residual_share, multiplier, reciprocal

    out.status = "INVALID_INPUT"
    out.message = "invalid scalar match-adjustment input"
    out.projection_share = projection_share
    out.residual_share = residual_share
    out.finite_bias = finite_bias
    out.finite_variance = finite_variance
    out.residual_mass = residual_mass
    out.deleted_mass = .
    out.reciprocal_residual = .
    if (hasmissing((projection_share,residual_share,finite_bias,
            finite_variance,residual_mass,rank_tolerance,block_tolerance)) |
        rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        block_tolerance <= 0 | block_tolerance >= 1) return(out)
    if (finite_variance < -100*rank_tolerance) {
        out.status = "JLA_MOMENT_FAILED"
        out.message = "finite-projection variance estimate is negative"
        return(out)
    }
    if (finite_variance < 0) {
        finite_variance = 0
        out.finite_variance = 0
    }
    maker_residual_share = 1-projection_share
    if (maker_residual_share <= block_tolerance) {
        out.status = "NONESTIMABLE_DELETION"
        out.message = "a match residual block is singular"
        return(out)
    }
    reciprocal = 1/residual_share
    out.reciprocal_residual = abs(residual_share*reciprocal-1)
    if (missing(reciprocal) | missing(out.reciprocal_residual) |
        out.reciprocal_residual > max((1e-10,100*rank_tolerance))) {
        out.status = "BLOCK_INVERSE_FAILED"
        out.message = "a scalar match residual inverse failed its reciprocal-residual gate"
        return(out)
    }
    multiplier = reciprocal + finite_bias*reciprocal^2 -
        finite_variance*reciprocal^3
    out.deleted_mass = residual_mass*multiplier
    if (missing(multiplier) | missing(out.deleted_mass)) {
        out.status = "NONFINITE_CORRECTION"
        out.message = "compressed match correction is nonfinite"
        return(out)
    }
    out.status = "CONVERGED"
    out.message = "scalar no-control match adjustment converged"
    return(out)
}

/* Vector form of the scalar adjustment with the same first-failure ordering.
   Every status is selected from the first offending logical deletion unit;
   no timing or data-dependent route choice is introduced. */
struct vckss_scale_unit_adjust_batch scalar vckss_scale_eng__unit_adjust_all(
    real colvector projection_share,
    real colvector residual_share,
    real colvector finite_bias,
    real colvector finite_variance,
    real colvector residual_mass,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_scale_unit_adjust_batch scalar out
    real colvector bad, maker_residual_share, multiplier, reciprocal
    real colvector reciprocal_residual
    real scalar groups

    out.status = "INVALID_INPUT"
    out.message = "invalid vector match-adjustment input"
    out.deleted_mass = J(0,1,.)
    out.max_reciprocal_residual = .
    groups = rows(projection_share)
    if (groups == 0 | cols(projection_share) != 1 |
        rows(residual_share) != groups | cols(residual_share) != 1 |
        rows(finite_bias) != groups | cols(finite_bias) != 1 |
        rows(finite_variance) != groups | cols(finite_variance) != 1 |
        rows(residual_mass) != groups | cols(residual_mass) != 1 |
        hasmissing((projection_share,residual_share,finite_bias,
            finite_variance,residual_mass)) |
        missing((rank_tolerance,block_tolerance)) |
        rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        block_tolerance <= 0 | block_tolerance >= 1) return(out)

    bad = selectindex(finite_variance :< -100*rank_tolerance)
    if (rows(bad)) {
        out.status = "JLA_MOMENT_FAILED"
        out.message = "finite-projection variance estimate is negative"
        return(out)
    }
    finite_variance = finite_variance:*(finite_variance:>0)
    maker_residual_share = 1:-projection_share
    bad = selectindex(maker_residual_share :<= block_tolerance)
    if (rows(bad)) {
        out.status = "NONESTIMABLE_DELETION"
        out.message = "a match residual block is singular"
        return(out)
    }
    reciprocal = 1:/residual_share
    reciprocal_residual = abs(residual_share:*reciprocal:-1)
    bad = selectindex(reciprocal :>= . :|
        reciprocal_residual :>= . :|
        reciprocal_residual :> max((1e-10,100*rank_tolerance)))
    if (rows(bad)) {
        out.status = "BLOCK_INVERSE_FAILED"
        out.message = "a scalar match residual inverse failed its reciprocal-residual gate"
        return(out)
    }
    multiplier = reciprocal + finite_bias:*reciprocal:^2 -
        finite_variance:*reciprocal:^3
    out.deleted_mass = residual_mass:*multiplier
    bad = selectindex(multiplier :>= . :| out.deleted_mass :>= .)
    if (rows(bad)) {
        out.status = "NONFINITE_CORRECTION"
        out.message = "compressed match correction is nonfinite"
        out.deleted_mass = J(0,1,.)
        return(out)
    }
    out.max_reciprocal_residual = max(reciprocal_residual)
    out.status = "CONVERGED"
    out.message = "vector no-control match adjustments converged"
    return(out)
}

struct vckss_scale_engine_result scalar vckss_scale_eng__record(
    struct vckss_scale_engine_result scalar out,
    struct vckss_solve_result scalar solved,
    real scalar stage,
    real scalar batch_id)
{
    out.solver_iterations = max((out.solver_iterations,solved.iterations))
    out.max_complete_residual =
        max((out.max_complete_residual,solved.relres))
    out.solver_schur_actions =
        out.solver_schur_actions+solved.schur_actions
    out.solver_schur_batches =
        out.solver_schur_batches+solved.schur_batches
    out.solver_precond_applications =
        out.solver_precond_applications+solved.preconditioner_applications
    out.solver_precond_batches =
        out.solver_precond_batches+solved.preconditioner_batches
    out.solver_schur_seconds =
        out.solver_schur_seconds+solved.schur_seconds
    out.solver_precond_seconds =
        out.solver_precond_seconds+solved.preconditioner_seconds
    out.solver_pcg_seconds = out.solver_pcg_seconds+solved.pcg_seconds
    out.fe_workspace_builds = out.fe_workspace_builds+solved.workspace_builds
    out.fe_buffered_schur_batches = out.fe_buffered_schur_batches+
        solved.buffered_schur_batches
    out.fe_legacy_schur_batches = out.fe_legacy_schur_batches+
        solved.legacy_schur_batches
    out.fe_buffered_schur_columns = out.fe_buffered_schur_columns+
        solved.buffered_schur_columns
    out.fe_legacy_schur_columns = out.fe_legacy_schur_columns+
        solved.legacy_schur_columns
    out.fe_packed_fallback_batches = out.fe_packed_fallback_batches+
        solved.packed_fallback_batches
    out.fe_max_buffer_width = max((out.fe_max_buffer_width,
        solved.max_buffer_width))
    out.fe_workspace_peak_bytes = max((out.fe_workspace_peak_bytes,
        solved.workspace_peak_bytes))
    out.fe_cell_bytes_avoided = out.fe_cell_bytes_avoided+
        solved.cell_bytes_avoided
    out.solver_rhs_diagnostics = out.solver_rhs_diagnostics \
        vckss__solver_trace_rows(
            stage,batch_id,solved.rhs_iterations,solved.rhs_relres)
    return(out)
}

struct vckss_scale_engine_result scalar vckss_scale_eng__run_prepared(
    struct vckss_scale_design scalar design,
    struct vckss_fe_design scalar base,
    struct vckss_solver_backend scalar backend,
    struct vckss_scale_atom_provider scalar provider,
    real scalar probes,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_scale_engine_result scalar out
    struct vckss_scale_engine_atom_batch scalar atom_batch
    struct vckss_scatter_plan scalar strata_plan, unit_plan
    struct vckss_scale_unit_adjust_batch scalar unit_adjustments
    struct vckss_solve_result scalar solved
    real scalar base_match, batch_columns, batch_finish, batch_start
    real scalar groups, strata, target_mass
    real matrix batch_moments, cell_atoms, direction_cell
    real matrix full_target_score, leverage_rhs, moment_compensation
    real matrix moment_subtotal, projected, target_atoms
    real matrix target_contractions, target_first, target_rhs
    real matrix target_prediction, unit_atoms
    real colvector cell_frequency_from_units, cell_frequency_from_strata
    real colvector cell_outcome_abs_from_units, cell_outcome_from_units
    real colvector finite_bias, finite_variance, m_first, m_second
    real colvector mixed_second
    real colvector p_first, p_second
    real colvector residual_mass, unit_projection
    real colvector unit_random_residual, unit_random_scaled
    real rowvector correction, corrected, identity_residual, plugin
    real rowvector target_reference_scale

    out = vckss_scale_eng__empty_result()
    out.rng_contract = provider.contract
    out.solver_route = backend.route
    out.residual_gate = max((1e-11,10*tolerance))
    out.max_complete_residual = 0
    out.max_reciprocal_residual = 0
    if (provider.status != "CONVERGED") {
        out.status = provider.status
        out.message = provider.message
        return(out)
    }
    if (design.status != "CONVERGED" |
        design.diagnostic.status != "CONVERGED" |
        design.diagnostic.cross_cell_deletion_units != 0 |
        design.coefficient_cells < 1 | design.deletion_units < 1 |
        design.worker_levels < 1 | design.firm_levels < 2 |
        rows(design.cell_worker) != design.coefficient_cells |
        rows(design.cell_firm) != design.coefficient_cells |
        rows(design.cell_frequency) != design.coefficient_cells |
        rows(design.cell_outcome_sum) != design.coefficient_cells |
        rows(design.cell_outcome_mean) != design.coefficient_cells |
        rows(design.cell_outcome_centered_ss) != design.coefficient_cells |
        rows(design.cell_target_mass) != design.coefficient_cells |
        rows(design.unit_cell) != design.deletion_units |
        rows(design.unit_frequency) != design.deletion_units |
        rows(design.unit_outcome_sum) != design.deletion_units |
        design.strata.status != "CONVERGED" |
        base.status != "CONVERGED" |
        provider.leverage == NULL |
        provider.target == NULL | backend.apply == NULL) return(out)
    if (probes < 2 | probes != floor(probes) |
        leverage_batch < 1 | leverage_batch != floor(leverage_batch) |
        target_batch < 1 | target_batch != floor(target_batch) |
        maxiter < 1 | maxiter != floor(maxiter) |
        tolerance <= 0 | tolerance >= 1 |
        rank_tolerance <= 0 | rank_tolerance >= 0.1 |
        block_tolerance <= 0 | block_tolerance >= 1) {
        out.status = "INVALID_TUNING"
        out.message = "invalid compressed estimator tuning parameter"
        return(out)
    }
    groups = design.deletion_units
    strata = design.strata.count
    target_mass = design.target_weight_sum
    if (strata < 1 | rows(design.strata.cell) != strata |
        rows(design.strata.per_copy_mass) != strata |
        rows(design.strata.physical_count) != strata |
        min(design.strata.per_copy_mass) < 0 |
        missing(target_mass) | target_mass <= 0) return(out)
    unit_plan = vckss_scale_eng__scatter_plan(
        design.unit_cell,design.coefficient_cells,
        design.unit_cell_order,design.unit_cell_panel)
    strata_plan = vckss_scale_eng__scatter_plan(
        design.strata.cell,design.coefficient_cells,
        design.strata.cell_order,design.strata.cell_panel)
    if (unit_plan.status != "CONVERGED" |
        strata_plan.status != "CONVERGED") {
        out.status = "FASTPATH_SCATTER_PLAN_INVALID"
        out.message = "retained compressed aggregation plan is invalid"
        return(out)
    }
    if (!(provider.certified_trials == 0 |
        provider.certified_trials == 1)) {
        out.status = "INVALID_PROBE_PROVIDER"
        out.message = "probe provider has no valid trial-count contract"
        return(out)
    }
    if (provider.certified_trials == 0 &
        provider.contract != "TEST_MATRIX_ATOMS") {
        out.status = "INVALID_PROBE_PROVIDER"
        out.message = "non-test probe provider must certify trial counts"
        return(out)
    }
    if (provider.certified_trials == 1) {
        if (rows(provider.leverage_trials) != groups |
            rows(provider.target_trials) != strata) {
            out.status = "PROBE_TRIAL_MISMATCH"
            out.message = "probe trial-count dimensions do not match compressed atoms"
            return(out)
        }
        if (max(abs(provider.leverage_trials-design.unit_frequency)) != 0 |
            max(abs(provider.target_trials-
                design.strata.physical_count)) != 0) {
            out.status = "PROBE_TRIAL_MISMATCH"
            out.message = "probe trial counts do not match literal-copy masses"
            return(out)
        }
    }

    /* These equalities certify that neither deletion units nor target strata
       silently replace coefficient cells. */
    cell_frequency_from_units = vckss_scale_eng__scatter_planned(
        design.unit_frequency,unit_plan)
    cell_outcome_from_units = vckss_scale_eng__scatter_planned(
        design.unit_outcome_sum,unit_plan)
    cell_outcome_abs_from_units = vckss_scale_eng__scatter_planned(
        abs(design.unit_outcome_sum),unit_plan)
    cell_frequency_from_strata = vckss_scale_eng__scatter_planned(
        design.strata.physical_count,strata_plan)
    if (rows(cell_frequency_from_units) != design.coefficient_cells |
        rows(cell_frequency_from_strata) != design.coefficient_cells |
        max(abs(cell_frequency_from_units-design.cell_frequency)) != 0 |
        max(abs(cell_frequency_from_strata-design.cell_frequency)) != 0 |
        !vckss_scale_eng__aggregate_ok(
            cell_outcome_from_units,design.cell_outcome_sum,
            cell_outcome_abs_from_units)) {
        out.status = "FASTPATH_AGGREGATE_MISMATCH"
        out.message = "compressed units or strata do not reproduce cell totals"
        return(out)
    }

    base_match = (base.n == design.coefficient_cells &
        base.worker_levels == design.worker_levels &
        base.firm_levels == design.firm_levels &
        (base.external_operator == 0 | base.external_operator == 1))
    if (base_match & base.external_operator == 0) {
        base_match = (rows(base.worker) == design.coefficient_cells &
            rows(base.firm) == design.coefficient_cells &
            rows(base.frequency) == design.coefficient_cells)
        if (base_match) {
            base_match = (max(abs(base.worker-design.cell_worker)) == 0 &
                max(abs(base.firm-design.cell_firm)) == 0 &
                max(abs(base.frequency-design.cell_frequency)) == 0)
        }
    }
    if (base_match & base.external_operator == 1) {
        base_match = (base.operator_context != NULL &
            base.operator_transpose_full != NULL &
            base.operator_predict != NULL &
            base.operator_schur_action != NULL &
            base.operator_schur_into != NULL)
    }
    if (!base_match) {
        out.status = "FASTPATH_BASE_MISMATCH"
        out.message = "routed FE design does not match compressed coefficient cells"
        return(out)
    }
    out.n_stored = design.n_rows
    out.n_physical = design.n_physical
    out.coefficient_cells = design.coefficient_cells
    out.deletion_units = groups
    out.target_strata = strata
    out.worker_levels = base.worker_levels
    out.firm_levels = base.firm_levels
    out.probes = probes
    out.target_weight_sum = target_mass
    out.preconditioner_ratio = base.preconditioner_ratio

    timer_clear(91)
    timer_clear(92)
    timer_clear(93)
    timer_clear(94)
    timer_clear(99)
    timer_on(91)
    solved = vckss__fe_solve_matrix_backend(
        base,vckss__fe_transpose_full(base,design.cell_outcome_sum),
        tolerance,maxiter,backend)
    if (solved.status != "CONVERGED") {
        timer_off(91)
        out.status = solved.status
        out.message = solved.message
        return(out)
    }
    /* Stage 2 is the fitted-model RHS in the public diagnostics contract.
       Stage 1 is reserved for nuisance/preparation RHSs, of which this
       no-control specialization has none. */
    out = vckss_scale_eng__record(out,solved,2,0)
    out.coefficient = solved.coefficient
    out.fitted_cell = solved.prediction
    plugin = vckss_scale_engine__plugin(design,base,out.coefficient)
    out.weighted_rss = vckss_scale_engine__column_sum(
        design.cell_outcome_centered_ss + design.cell_frequency:*
        (design.cell_outcome_mean-out.fitted_cell):^2)[1]
    timer_off(91)
    out.fit_seconds = vckss__timer_seconds(91)

    /* Match leverage moments.  Fixed contiguous tiles and compensated batch
       merges retain logical probe order.  A different solve width may alter
       floating-point grouping, but it cannot alter the supplied atoms. */
    timer_on(92)
    moment_subtotal = J(groups,5,0)
    moment_compensation = J(groups,5,0)
    for (batch_start=1; batch_start<=probes;
        batch_start=batch_start+leverage_batch) {
        batch_finish = min((probes,batch_start+leverage_batch-1))
        batch_columns = batch_finish-batch_start+1
        timer_on(94)
        atom_batch = (*provider.leverage)(
            provider.context,batch_start,batch_finish)
        timer_off(94)
        if (atom_batch.status != "CONVERGED") {
            timer_off(92)
            out.status = atom_batch.status
            out.message = atom_batch.message
            return(out)
        }
        unit_atoms = atom_batch.value
        if (rows(unit_atoms) != groups |
            cols(unit_atoms) != batch_columns |
            !vckss_scale_eng__valid_atoms(
                unit_atoms,design.unit_frequency)) {
            timer_off(92)
            out.status = "INVALID_PROBE_ATOM"
            out.message = "deletion-unit atoms violate literal-copy sign-sum support"
            return(out)
        }
        cell_atoms = vckss_scale_eng__scatter_planned(
            unit_atoms,unit_plan)
        leverage_rhs = vckss__fe_transpose_full(base,cell_atoms)
        solved = vckss__fe_solve_matrix_backend(
            base,leverage_rhs,tolerance,maxiter,backend)
        if (solved.status != "CONVERGED") {
            timer_off(92)
            out.status = solved.status
            out.message = solved.message
            return(out)
        }
        out = vckss_scale_eng__record(out,solved,4,batch_start)
        projected = solved.prediction
        unit_projection = sqrt(design.unit_frequency):*
            projected[design.unit_cell,.]
        unit_random_scaled = unit_atoms:/sqrt(design.unit_frequency)
        unit_random_residual = unit_random_scaled-unit_projection
        vckss_scale_eng__moment_add(
            &moment_subtotal,&moment_compensation,
            unit_projection,unit_random_residual)
    }
    batch_moments = moment_subtotal+moment_compensation
    if (rows(batch_moments) != groups | cols(batch_moments) != 5 |
        hasmissing(batch_moments)) {
        timer_off(92)
        out.status = "JLA_MOMENT_FAILED"
        out.message = "matrix-wide finite-projection moments are nonfinite"
        return(out)
    }
    p_first = batch_moments[.,1]
    m_first = batch_moments[.,2]
    p_second = batch_moments[.,3]
    m_second = batch_moments[.,4]
    mixed_second = batch_moments[.,5]
    out.unit_projection_share = (p_first:/probes) :/
        ((p_first+m_first):/probes)
    out.unit_residual_share = (m_first:/probes) :/
        ((p_first+m_first):/probes)
    if (hasmissing(out.unit_projection_share) |
        hasmissing(out.unit_residual_share) |
        min((p_first+m_first):/probes) <= block_tolerance) {
        timer_off(92)
        out.status = "JLA_CONSTRAINT_FAILED"
        out.message = "JLA projection and residual masses do not have positive sum"
        return(out)
    }
    p_second = p_second:/probes
    m_second = m_second:/probes
    mixed_second = mixed_second:/probes
    finite_variance = (out.unit_residual_share:^2:*p_second +
        out.unit_projection_share:^2:*m_second -
        2:*out.unit_projection_share:*out.unit_residual_share:*
            mixed_second):/probes
    finite_bias = (out.unit_residual_share:*p_second -
        out.unit_projection_share:*m_second +
        (out.unit_residual_share-out.unit_projection_share):*
            mixed_second):/probes
    if (hasmissing(finite_bias) | hasmissing(finite_variance)) {
        timer_off(92)
        out.status = "JLA_MOMENT_FAILED"
        out.message = "finite-projection moments are nonfinite"
        return(out)
    }
    if (min(finite_variance) < -100*rank_tolerance) {
        timer_off(92)
        out.status = "JLA_MOMENT_FAILED"
        out.message = "finite-projection variance estimate is negative"
        return(out)
    }
    finite_variance = finite_variance:*(finite_variance:>0)
    out.unit_finite_bias = finite_bias
    out.unit_finite_variance = finite_variance
    /* Timer 99 is nested inside leverage/target stage timers.  It records
       only construction of D_g/K_c and target contraction/reduction; it is
       diagnostic attribution and must not be added to total_seconds. */
    timer_on(99)
    residual_mass = design.unit_outcome_sum -
        design.unit_frequency:*out.fitted_cell[design.unit_cell]
    out.unit_residual_mass = residual_mass
    /* D_g = E_g(m_g^-1+B_g m_g^-2-V_g m_g^-3). */
    unit_adjustments = vckss_scale_eng__unit_adjust_all(
        out.unit_projection_share,out.unit_residual_share,
        finite_bias,finite_variance,residual_mass,
        rank_tolerance,block_tolerance)
    if (unit_adjustments.status != "CONVERGED") {
        timer_off(99)
        timer_off(92)
        out.status = unit_adjustments.status
        out.message = unit_adjustments.message
        return(out)
    }
    out.unit_d = unit_adjustments.deleted_mass
    out.max_reciprocal_residual = max((
        out.max_reciprocal_residual,
        unit_adjustments.max_reciprocal_residual))
    out.cell_correction_weight = vckss_scale_eng__scatter_planned(
        design.unit_outcome_sum:*out.unit_d,unit_plan)
    if (hasmissing(out.unit_d) |
        hasmissing(out.cell_correction_weight)) {
        timer_off(99)
        timer_off(92)
        out.status = "NONFINITE_CORRECTION"
        out.message = "compressed match correction is nonfinite"
        return(out)
    }
    timer_off(99)
    out.max_leverage = max(out.unit_projection_share)
    timer_off(92)
    out.leverage_seconds = vckss__timer_seconds(92)

    /* End the leverage scratch lifetime before allocating the target's 2B
       coefficient and prediction matrices.  The result retains only the
       registered unit diagnostics and K_c sufficient statistic. */
    cell_atoms = leverage_rhs = projected = unit_atoms = J(0,0,.)
    unit_projection = unit_random_scaled = unit_random_residual = J(0,1,.)
    p_first = m_first = p_second = m_second = mixed_second = J(0,1,.)
    batch_moments = moment_subtotal = moment_compensation = J(0,0,.)
    finite_bias = finite_variance = J(0,1,.)
    residual_mass = J(0,1,.)

    /* Target directions are formed at exact (cell, per-copy target mass)
       strata.  No tolerance-based grouping is permitted. */
    timer_on(93)
    out.target_draws = J(probes,4,.)
    for (batch_start=1; batch_start<=probes;
        batch_start=batch_start+target_batch) {
        batch_finish = min((probes,batch_start+target_batch-1))
        batch_columns = batch_finish-batch_start+1
        timer_on(94)
        atom_batch = (*provider.target)(
            provider.context,batch_start,batch_finish)
        timer_off(94)
        if (atom_batch.status != "CONVERGED") {
            timer_off(93)
            out.status = atom_batch.status
            out.message = atom_batch.message
            return(out)
        }
        target_atoms = atom_batch.value
        if (rows(target_atoms) != strata |
            cols(target_atoms) != batch_columns |
            !vckss_scale_eng__valid_atoms(
                target_atoms,design.strata.physical_count)) {
            timer_off(93)
            out.status = "INVALID_PROBE_ATOM"
            out.message = "target-stratum atoms violate literal-copy sign-sum support"
            return(out)
        }
        target_first = sqrt(design.strata.per_copy_mass:/target_mass):*
            target_atoms
        target_first = vckss_scale_eng__scatter_planned(
            target_first,strata_plan)
        target_reference_scale =
            vckss_scale_engine__column_sum(abs(target_first)) +
            abs(vckss_scale_engine__column_sum(target_first))
        direction_cell = vckss_scale_eng__center_target(
            target_first,design.cell_target_mass:/target_mass)
        if (rows(direction_cell) != design.coefficient_cells |
            cols(direction_cell) != batch_columns |
            hasmissing(direction_cell)) {
            timer_off(93)
            out.status = "TARGET_CENTERING_FAILED"
            out.message = "compressed target directions could not be centered"
            return(out)
        }
        full_target_score = vckss__fe_transpose_full(base,direction_cell)
        full_target_score[1..base.worker_levels,.] =
            vckss_scale_eng__balance_score(
                full_target_score[1..base.worker_levels,.],
                target_reference_scale)
        full_target_score[(base.worker_levels+1)..rows(full_target_score),.] =
            vckss_scale_eng__balance_score(
                full_target_score[
                    (base.worker_levels+1)..rows(full_target_score),.],
                target_reference_scale)
        if (hasmissing(full_target_score) |
            rows(full_target_score) !=
                base.worker_levels+base.firm_levels) {
            timer_off(93)
            out.status = "TARGET_CENTERING_FAILED"
            out.message = "target score compatibility repair exceeded roundoff"
            return(out)
        }
        target_rhs = J(base.worker_levels+base.firm_levels,
            2*batch_columns,0)
        target_rhs[1..base.worker_levels,
            2:*(1..batch_columns):-1] =
            full_target_score[1..base.worker_levels,.]
        target_rhs[(base.worker_levels+1)..rows(target_rhs),
            2:*(1..batch_columns)] =
            full_target_score[(base.worker_levels+1)..rows(target_rhs),.]
        solved = vckss__fe_solve_matrix_backend(
            base,target_rhs,tolerance,maxiter,backend)
        if (solved.status != "CONVERGED") {
            timer_off(93)
            out.status = solved.status
            out.message = solved.message
            return(out)
        }
        out = vckss_scale_eng__record(out,solved,5,batch_start)
        timer_on(99)
        target_prediction = solved.prediction
        target_contractions = vckss_scale_eng__target_contract(
            out.cell_correction_weight,target_prediction)
        if (rows(target_contractions) != batch_columns |
            cols(target_contractions) != 4 |
            hasmissing(target_contractions)) {
            timer_off(99)
            timer_off(93)
            out.status = "NONFINITE_CORRECTION"
            out.message = "matrix-wide target contractions are nonfinite"
            return(out)
        }
        out.target_draws[batch_start..batch_finish,.] =
            target_contractions
        timer_off(99)
    }
    timer_on(99)
    correction = vckss_scale_engine__column_sum(out.target_draws):/probes
    corrected = plugin-correction
    out.plugin = plugin
    out.correction = correction
    out.corrected = corrected
    out.numerical_mcse = vckss_scale_engine__mcse(out.target_draws)
    identity_residual = (
        vckss_scale_eng__identity_resid(plugin),
        vckss_scale_eng__identity_resid(correction),
        vckss_scale_eng__identity_resid(corrected),
        vckss_scale_eng__identity_resid(out.target_draws))
    out.target_identity_residual = max(identity_residual)
    if (hasmissing(plugin) | hasmissing(correction) |
        hasmissing(corrected) | hasmissing(out.numerical_mcse) |
        hasmissing(out.target_identity_residual)) {
        timer_off(99)
        timer_off(93)
        out.status = "NONFINITE_CORRECTION"
        out.message = "compressed target correction is nonfinite"
        return(out)
    }
    if (out.target_identity_residual >
        vckss_scale_eng__roundoff_gate()) {
        timer_off(99)
        timer_off(93)
        out.status = "TARGET_IDENTITY_FAILED"
        out.message = "compressed target accounting identities failed"
        return(out)
    }
    timer_off(99)
    timer_off(93)
    out.target_seconds = vckss__timer_seconds(93)
    out.rng_seconds = vckss__timer_seconds(94)
    out.correction_seconds = vckss__timer_seconds(99)
    out.total_seconds =
        out.fit_seconds+out.leverage_seconds+out.target_seconds
    out.status = "CONVERGED"
    out.message = "experimental compressed no-control match KSS calculation converged"
    return(out)
}

struct vckss_scale_engine_result scalar vckss_scale_engine__run(
    struct vckss_scale_design scalar design,
    struct vckss_solver_backend scalar backend,
    struct vckss_scale_atom_provider scalar provider,
    real scalar probes,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_fe_design scalar base
    struct vckss_scale_engine_result scalar out

    base = vckss__fe_prepare(
        design.cell_worker,design.cell_firm,design.cell_frequency,
        rank_tolerance)
    if (base.status != "CONVERGED") {
        out = vckss_scale_eng__empty_result()
        out.status = base.status
        out.message = base.message
        return(out)
    }
    return(vckss_scale_eng__run_prepared(
        design,base,backend,provider,probes,leverage_batch,target_batch,
        tolerance,maxiter,rank_tolerance,block_tolerance))
}

/* Convert the richer experimental receipt to the installed estimator result
   contract expected by the existing routed-solver bridge. */
struct vckss_result scalar vckss_scale_eng__as_result(
    struct vckss_scale_engine_result scalar source,
    real scalar setup_seconds)
{
    struct vckss_result scalar out

    if (source.status != "CONVERGED") {
        return(vckss__failure(source.status,source.message))
    }
    if (missing(setup_seconds) | setup_seconds < 0) {
        return(vckss__failure(
            "INVALID_SOLVER_BACKEND","compressed setup time is invalid"))
    }
    out = vckss__empty_result()
    out.status = source.status
    out.message = source.message
    out.plugin = source.plugin
    out.correction = source.correction
    out.corrected = source.corrected
    out.numerical_mcse = source.numerical_mcse
    out.n_stored = source.n_stored
    out.n_physical = source.n_physical
    out.worker_levels = source.worker_levels
    out.firm_levels = source.firm_levels
    out.parameters = source.worker_levels+source.firm_levels-1
    out.full_parameters = out.parameters
    out.correction_parameters = out.parameters
    out.deletion_units = source.deletion_units
    out.target_weight_sum = source.target_weight_sum
    out.max_leverage = source.max_leverage
    out.information_rcond = .
    out.preconditioner_ratio = source.preconditioner_ratio
    out.control_schur_rcond = .
    out.deletion_rank_gap = .
    out.inverse_relres = max((source.max_complete_residual,
        source.max_reciprocal_residual))
    out.weighted_rss = source.weighted_rss
    out.fit_seconds = source.fit_seconds
    out.leverage_seconds = source.leverage_seconds
    out.target_seconds = source.target_seconds
    out.correction_seconds = source.correction_seconds
    out.preconditioner_seconds = setup_seconds
    out.schur_seconds = source.solver_schur_seconds
    out.preconditioner_apply_seconds = source.solver_precond_seconds
    out.pcg_seconds = source.solver_pcg_seconds
    out.solver_backend_seconds = setup_seconds+source.solver_pcg_seconds
    out.solver_iterations = source.solver_iterations
    out.solver_max_residual = source.max_complete_residual
    out.solver_schur_actions = source.solver_schur_actions
    out.solver_schur_batches = source.solver_schur_batches
    out.solver_precond_applications = source.solver_precond_applications
    out.solver_precond_batches = source.solver_precond_batches
    out.solver_rhs_diagnostics = source.solver_rhs_diagnostics
    out.fe_workspace_applicable = 1
    out.fe_workspace_builds = source.fe_workspace_builds
    out.fe_buffered_schur_batches = source.fe_buffered_schur_batches
    out.fe_legacy_schur_batches = source.fe_legacy_schur_batches
    out.fe_buffered_schur_columns = source.fe_buffered_schur_columns
    out.fe_legacy_schur_columns = source.fe_legacy_schur_columns
    out.fe_packed_fallback_batches = source.fe_packed_fallback_batches
    out.fe_max_buffer_width = source.fe_max_buffer_width
    out.fe_workspace_peak_bytes = source.fe_workspace_peak_bytes
    out.fe_cell_bytes_avoided = source.fe_cell_bytes_avoided
    out.probes = source.probes
    return(out)
}

struct vckss_scale_route_context scalar vckss_scale_eng__route_context(
    pointer(struct vckss_scale_design scalar) scalar design,
    struct vckss_scale_atom_provider scalar provider,
    real scalar probes,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_scale_route_context scalar out

    if (provider.status == "CONVERGED") {
        out.status = "INVALID_INPUT"
        out.message = "compressed route context is invalid"
    }
    else {
        out.status = provider.status
        out.message = provider.message
    }
    out.design = design
    out.provider = provider
    out.probes = probes
    out.leverage_batch = leverage_batch
    out.target_batch = target_batch
    out.tolerance = tolerance
    out.maxiter = maxiter
    out.rank_tolerance = rank_tolerance
    out.block_tolerance = block_tolerance
    out.last = vckss_scale_eng__empty_result()
    if (design != NULL) {
        if ((*design).status == "CONVERGED" &
            provider.status == "CONVERGED" & probes >= 2 &
            probes == floor(probes) & leverage_batch >= 1 &
            leverage_batch == floor(leverage_batch) & target_batch >= 1 &
            target_batch == floor(target_batch) & tolerance > 0 &
            tolerance < 1 & maxiter >= 1 & maxiter == floor(maxiter) &
            rank_tolerance > 0 & rank_tolerance < 0.1 &
            block_tolerance > 0 & block_tolerance < 1) {
            out.status = "CONVERGED"
            out.message = "compressed route context prepared"
        }
    }
    return(out)
}

struct vckss_result scalar vckss_scale_eng__route_callback(
    pointer scalar context,
    struct vckss_fe_design scalar base,
    struct vckss_solver_backend scalar backend,
    real scalar setup_seconds)
{
    pointer(struct vckss_scale_route_context scalar) scalar source

    if (context == NULL) {
        return(vckss__failure(
            "INVALID_INPUT","compressed route context is unavailable"))
    }
    source = context
    if ((*source).status != "CONVERGED") {
        return(vckss__failure(
            (*source).status,(*source).message))
    }
    if ((*source).design == NULL) {
        return(vckss__failure(
            "INVALID_INPUT","compressed design reference is unavailable"))
    }
    (*source).last = vckss_scale_eng__run_prepared(
        *(*source).design,base,backend,(*source).provider,
        (*source).probes,(*source).leverage_batch,(*source).target_batch,
        (*source).tolerance,(*source).maxiter,(*source).rank_tolerance,
        (*source).block_tolerance)
    return(vckss_scale_eng__as_result((*source).last,setup_seconds))
}

/* Pass this pointer and &vckss_scale_route_context to the optional callback
   arguments of vckss_solver__jla_routed().  The router must itself be called
   on the coefficient-cell worker, firm, and frequency arrays so its prepared
   base is exactly the object certified by run_prepared(). */
pointer scalar vckss_scale_eng__callback_ptr()
{
    return(&vckss_scale_eng__route_callback())
}

struct vckss_scale_engine_result scalar vckss_scale_eng__run_atoms(
    struct vckss_scale_design scalar design,
    struct vckss_solver_backend scalar backend,
    real matrix leverage_unit_atoms,
    real matrix target_stratum_atoms,
    real scalar leverage_batch,
    real scalar target_batch,
    real scalar tolerance,
    real scalar maxiter,
    real scalar rank_tolerance,
    real scalar block_tolerance)
{
    struct vckss_scale_engine_matrix_atoms scalar source
    struct vckss_scale_atom_provider scalar provider

    source.leverage_unit = leverage_unit_atoms
    source.target_stratum = target_stratum_atoms
    provider = vckss_scale_eng__mat_provider(&source)
    if (cols(leverage_unit_atoms) != cols(target_stratum_atoms)) {
        return(vckss_scale_eng__empty_result())
    }
    return(vckss_scale_engine__run(
        design,backend,provider,cols(leverage_unit_atoms),
        leverage_batch,target_batch,tolerance,maxiter,
        rank_tolerance,block_tolerance))
}

end
