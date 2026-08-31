*! fevc KSS-STREAMLINE-1 runtime-scoped RNG module
*! version 0.4.0-alpha.1 18aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum off

/*
The K1 checkpoint compares two execution-path-independent contracts.  Local
Stata 18 evidence selects the simpler qualifying fixed-domain cursor for the
installed experimental estimator.  Both candidates use the same canonical
semantic atom order and the registered vector-parameter call when every
trial count is within Stata's per-call limit.  Exact scalar chunking in that
same order is reserved for larger counts.
*/

struct vckss_rng__snapshot
{
    string scalar algorithm
    real scalar stream
    string scalar state
}

struct vckss_rng__full_snapshot
{
    string scalar status
    struct vckss_rng__snapshot scalar active
    string scalar sort_state
    string scalar mt64s_stream1_state
    string scalar mt64s_stream2_state
    string scalar mt64s_selected_stream_state
}

struct vckss_rng__stream_snapshot
{
    string scalar status
    struct vckss_rng__snapshot scalar active
    real colvector stream
    string colvector state
}

struct vckss_rng__result
{
    string scalar status
    string scalar message
    string scalar contract_version
    string scalar candidate
    string scalar runtime
    string scalar domain
    string scalar call_shape
    real scalar master_seed
    real scalar probe_start
    real scalar probe_count
    real scalar stream_first
    real scalar stream_last
    real scalar maximum_trials
    real scalar chunk_calls
    real colvector semantic_rank
    real colvector canonical_order
    real matrix atoms
}

struct vckss_rng__call_shape_result
{
    string scalar status
    string scalar message
    real colvector scalar_atoms
    real colvector vector_atoms
    string scalar scalar_state
    string scalar vector_state
    real scalar atoms_equal
    real scalar states_equal
}

struct vckss_rng__benchmark_result
{
    string scalar status
    string scalar message
    string scalar recommended_candidate
    real scalar repetitions
    real scalar per_probe_seconds
    real scalar per_domain_seconds
    real scalar per_probe_over_domain
}

struct vckss_rng__cursor
{
    string scalar status
    string scalar message
    string scalar contract_version
    string scalar runtime
    string scalar domain
    string scalar call_shape
    real scalar master_seed
    real scalar stream
    real scalar next_probe
    string scalar generator_state
    real colvector semantic_rank
    real colvector canonical_order
    real colvector trials
}

struct vckss_rng__caller_guard
{
    real scalar active
    struct vckss_rng__full_snapshot scalar saved
}

real scalar vckss_rng__api_level()
{
    return(4)
}

string scalar vckss_rng__build_id()
{
    return("vckss-rng-numeric-ranks-v4")
}

string scalar vckss_rng__invariant_version()
{
    return("KSS-RNG-K1-INVARIANT-V1")
}

string scalar vckss_rng__k1_recommendation()
{
    // The one-time K1 comparison selected the stateful fixed-domain cursor.
    // That result is reused; ordinary development never reruns the benchmark.
    return("per_domain_stream_cursor")
}

string scalar vckss_rng__production_contract()
{
    real scalar runtime

    runtime = st_numscalar("c(stata_version)")
    if (!missing(runtime) & runtime >= 18 & runtime < 19) {
        return("KSS-MT64S-DOMAIN-CURSOR-V3-STATA18")
    }
    if (!missing(runtime) & runtime >= 19 & runtime < 20) {
        return("KSS-MT64S-DOMAIN-CURSOR-V3-STATA19")
    }
    // JLA estimation fails closed on an unregistered runtime.  Exact
    // estimation does not need this production contract.
    return("")
}

real scalar vckss_rng__max_binomial_trials()
{
    // Stata 18 documents rbinomial() on 1 <= n <= 1e+11.
    return(100000000000)
}

real scalar vckss_rng__maximum_exact_integer()
{
    // Largest consecutive nonnegative integer exactly represented in double.
    return(2^53-1)
}

real scalar vckss_rng__maximum_probes()
{
    // Legacy K1 per-probe candidate limit.  Production uses streams one and
    // two and is not constrained by this experimental registry partition.
    return(16383)
}

string scalar vckss_rng__candidate_version(string scalar candidate)
{
    if (candidate == "per_probe_stream") {
        return("KSS-MT64S-PER-PROBE-CANDIDATE-V2")
    }
    if (candidate == "per_domain_stream") {
        return("KSS-MT64S-PER-DOMAIN-CANDIDATE-V2")
    }
    return("")
}

real scalar vckss_rng__candidate_ok(string scalar candidate)
{
    return(candidate == "per_probe_stream" |
        candidate == "per_domain_stream")
}

real scalar vckss_rng__domain_ok(string scalar domain)
{
    return(domain == "leverage" | domain == "target")
}

real scalar vckss_rng__runtime_registered()
{
    real scalar runtime

    runtime = st_numscalar("c(stata_version)")
    return(!missing(runtime) & runtime >= 18 & runtime < 20)
}

struct vckss_rng__snapshot scalar vckss_rng__capture()
{
    struct vckss_rng__snapshot scalar out

    out.algorithm = st_global("c(rng)")
    out.stream = st_numscalar("c(rngstream)")
    out.state = rngstate()
    return(out)
}

real scalar vckss_rng__restore(
    struct vckss_rng__snapshot scalar saved)
{
    real scalar rc

    if (!(saved.algorithm == "default" | saved.algorithm == "mt64" |
        saved.algorithm == "mt64s" | saved.algorithm == "kiss32") |
        saved.state == "") return(198)
    if (missing(saved.stream) | saved.stream < 1 |
        saved.stream > 32768 | saved.stream != floor(saved.stream)) {
        return(198)
    }
    // c(rngstream) is session state even while another algorithm is active.
    // Restore it first, then restore the caller's selected algorithm/state.
    rc = _stata("set rng mt64s",1,1)
    if (rc) return(rc)
    rc = _stata("set rngstream "+strofreal(saved.stream,"%9.0f"),1,1)
    if (rc) return(rc)
    rc = _stata("set rng "+saved.algorithm,1,1)
    if (rc) return(rc)
    rngstate(saved.state)
    return(0)
}

struct vckss_rng__stream_snapshot scalar vckss_rng__capture_streams(
    real colvector streams)
{
    struct vckss_rng__stream_snapshot scalar out
    real scalar index, rc, restore_rc

    out.status = "INVALID"
    out.active = vckss_rng__capture()
    out.stream = J(0,1,.)
    out.state = J(0,1,"")
    if (cols(streams) != 1 | rows(streams) < 1 | hasmissing(streams) |
        min(streams) < 1 | max(streams) > 32768 |
        any(streams :!= floor(streams)) |
        rows(uniqrows(sort(streams,1))) != rows(streams)) return(out)
    out.stream = streams
    out.state = J(rows(streams),1,"")
    rc = _stata("set rng mt64s",1,1)
    for (index=1; index<=rows(streams) & !rc; index++) {
        rc = _stata("set rngstream "+
            strofreal(streams[index],"%9.0f"),1,1)
        if (!rc) out.state[index] = rngstate()
        if (!rc & out.state[index] == "") rc = 498
    }
    restore_rc = vckss_rng__restore(out.active)
    if (rc | restore_rc) return(out)
    out.status = "OK"
    return(out)
}

real scalar vckss_rng__restore_streams(
    struct vckss_rng__stream_snapshot scalar saved)
{
    real scalar active_rc, first_rc, index, rc

    if (saved.status != "OK" | rows(saved.stream) < 1 |
        rows(saved.state) != rows(saved.stream) |
        any(saved.state :== "")) return(198)
    first_rc = _stata("set rng mt64s",1,1)
    if (!first_rc) {
        for (index=1; index<=rows(saved.stream); index++) {
            rc = _stata("set rngstream "+
                strofreal(saved.stream[index],"%9.0f"),1,1)
            if (!rc) rngstate(saved.state[index])
            if (rc & !first_rc) first_rc = rc
        }
    }
    active_rc = vckss_rng__restore(saved.active)
    if (!first_rc) first_rc = active_rc
    return(first_rc)
}

struct vckss_rng__full_snapshot scalar vckss_rng__capture_full()
{
    struct vckss_rng__full_snapshot scalar out
    real scalar rc, restore_rc

    out.status = "INVALID"
    out.active = vckss_rng__capture()
    out.sort_state = st_global("c(sortrngstate)")
    out.mt64s_stream1_state = ""
    out.mt64s_stream2_state = ""
    out.mt64s_selected_stream_state = ""
    if (out.active.algorithm == "" | out.active.state == "" |
        out.sort_state == "" | missing(out.active.stream)) return(out)
    rc = _stata("set rng mt64s",1,1)
    if (!rc) rc = _stata("set rngstream 1",1,1)
    if (!rc) out.mt64s_stream1_state = rngstate()
    if (!rc) rc = _stata("set rngstream 2",1,1)
    if (!rc) out.mt64s_stream2_state = rngstate()
    if (!rc) rc = _stata("set rngstream "+
        strofreal(out.active.stream,"%9.0f"),1,1)
    if (!rc) out.mt64s_selected_stream_state = rngstate()
    restore_rc = vckss_rng__restore(out.active)
    if (rc | restore_rc | out.mt64s_stream1_state == "" |
        out.mt64s_stream2_state == "" |
        out.mt64s_selected_stream_state == "") return(out)
    out.status = "OK"
    return(out)
}

real scalar vckss_rng__restore_full(
    struct vckss_rng__full_snapshot scalar saved)
{
    real scalar rc

    if (saved.status != "OK" | saved.sort_state == "" |
        saved.mt64s_stream1_state == "" |
        saved.mt64s_stream2_state == "" |
        saved.mt64s_selected_stream_state == "") return(198)
    rc = _stata("set rng mt64s",1,1)
    if (!rc) rc = _stata("set rngstream 1",1,1)
    if (!rc) rngstate(saved.mt64s_stream1_state)
    if (!rc) rc = _stata("set rngstream 2",1,1)
    if (!rc) rngstate(saved.mt64s_stream2_state)
    if (!rc) rc = _stata("set rngstream "+
        strofreal(saved.active.stream,"%9.0f"),1,1)
    if (!rc) rngstate(saved.mt64s_selected_stream_state)
    if (!rc) rc = vckss_rng__restore(saved.active)
    if (!rc) rc = _stata("set sortrngstate "+saved.sort_state,1,1)
    return(rc)
}

struct vckss_rng__caller_guard scalar vckss_rng__empty_guard()
{
    struct vckss_rng__caller_guard scalar out

    out.active = 0
    out.saved.status = "INVALID"
    out.saved.active.algorithm = ""
    out.saved.active.stream = .
    out.saved.active.state = ""
    out.saved.sort_state = ""
    out.saved.mt64s_stream1_state = ""
    out.saved.mt64s_stream2_state = ""
    out.saved.mt64s_selected_stream_state = ""
    return(out)
}

real scalar vckss_rng__guard_begin()
{
    external struct vckss_rng__caller_guard scalar VCKSS_RNG_CALLER_GUARD

    if (VCKSS_RNG_CALLER_GUARD.active) return(498)
    VCKSS_RNG_CALLER_GUARD.saved = vckss_rng__capture_full()
    if (VCKSS_RNG_CALLER_GUARD.saved.status != "OK") return(498)
    VCKSS_RNG_CALLER_GUARD.active = 1
    return(0)
}

real scalar vckss_rng__guard_restore()
{
    external struct vckss_rng__caller_guard scalar VCKSS_RNG_CALLER_GUARD
    real scalar rc

    if (!VCKSS_RNG_CALLER_GUARD.active) return(0)
    rc = vckss_rng__restore_full(VCKSS_RNG_CALLER_GUARD.saved)
    if (!rc) VCKSS_RNG_CALLER_GUARD = vckss_rng__empty_guard()
    return(rc)
}

real scalar vckss_rng__set_stream_seed(
    real scalar stream,
    real scalar master_seed)
{
    real scalar rc

    if (missing(stream) | stream < 1 | stream > 32768 |
        stream != floor(stream) | missing(master_seed) | master_seed < 0 |
        master_seed > 2147483647 | master_seed != floor(master_seed)) {
        return(198)
    }
    rc = _stata("set rng mt64s",1,1)
    if (rc) return(rc)
    rc = _stata("set rngstream "+strofreal(stream,"%9.0f"),1,1)
    if (rc) return(rc)
    rseed(master_seed)
    return(0)
}

real scalar vckss_rng__probe_stream(
    string scalar domain,
    real scalar probe)
{
    if (!vckss_rng__domain_ok(domain) | missing(probe) | probe < 1 |
        probe > vckss_rng__maximum_probes() | probe != floor(probe)) {
        return(.)
    }
    if (domain == "leverage") return(probe)
    return(vckss_rng__maximum_probes()+probe)
}

real scalar vckss_rng__domain_stream(string scalar domain)
{
    // Candidate namespaces are versioned separately.  Low stream numbers
    // avoid the measurable setup cost of jumping to the end of the registry.
    if (domain == "leverage") return(1)
    if (domain == "target") return(2)
    return(.)
}

real colvector vckss_rng__canonical_order(real colvector semantic_rank)
{
    real colvector sorted
    real colvector canonical

    if (cols(semantic_rank) != 1 | rows(semantic_rank) < 1 |
        hasmissing(semantic_rank) | min(semantic_rank) < 1 |
        any(semantic_rank :!= floor(semantic_rank)) |
        max(semantic_rank) > vckss_rng__maximum_exact_integer()) {
        return(J(0,1,.))
    }
    sorted = order(semantic_rank,1)
    canonical = semantic_rank[sorted]
    if (rows(canonical) > 1) {
        if (any(canonical[|2\rows(canonical)|] :==
            canonical[|1\rows(canonical)-1|])) return(J(0,1,.))
    }
    return(sorted)
}

real scalar vckss_rng__trials_ok(real colvector trials)
{
    real scalar atom, total

    if (cols(trials) != 1 | rows(trials) < 1 | hasmissing(trials) |
        any(trials :< 0) | any(trials :!= floor(trials)) |
        any(trials :> vckss_rng__maximum_exact_integer())) return(0)
    total = 0
    for (atom=1; atom<=rows(trials); atom++) {
        // Subtract before adding so a rounded over-limit sum cannot pass.
        if (trials[atom] > vckss_rng__maximum_exact_integer()-total) {
            return(0)
        }
        total = total+trials[atom]
    }
    return(1)
}

real rowvector vckss_rng__binomial_atom_limit(
    real scalar trials,
    real scalar maximum_trials)
{
    real scalar successes, remaining, chunk, calls

    if (missing(trials) | trials < 0 | trials != floor(trials) |
        trials > vckss_rng__maximum_exact_integer() |
        missing(maximum_trials) | maximum_trials < 1 |
        maximum_trials > vckss_rng__max_binomial_trials() |
        maximum_trials != floor(maximum_trials)) return((.,.))
    successes = 0
    remaining = trials
    calls = 0
    while (remaining > 0) {
        chunk = min((remaining,maximum_trials))
        successes = successes+rbinomial(1,1,chunk,0.5)
        remaining = remaining-chunk
        calls++
    }
    return((2*successes-trials,calls))
}

real rowvector vckss_rng__binomial_atom_scalar(real scalar trials)
{
    return(vckss_rng__binomial_atom_limit(
        trials,vckss_rng__max_binomial_trials()))
}

real matrix vckss_rng__draw_probe_scalar(real colvector trials)
{
    real scalar atom
    real rowvector draw
    real matrix out

    out = J(rows(trials),2,0)
    for (atom=1; atom<=rows(trials); atom++) {
        draw = vckss_rng__binomial_atom_scalar(trials[atom])
        out[atom,.] = draw
    }
    return(out)
}

real colvector vckss_rng__draw_probe_vector(real colvector trials)
{
    real colvector positive, out

    if (!vckss_rng__trials_ok(trials) |
        max(trials) > vckss_rng__max_binomial_trials()) {
        return(J(0,1,.))
    }
    out = J(rows(trials),1,0)
    positive = selectindex(trials :> 0)
    if (rows(positive)) {
        out[positive] = 2:*rbinomial(1,1,trials[positive],0.5):-
            trials[positive]
    }
    return(out)
}

real matrix vckss_rng__draw_probe_registered(real colvector trials)
{
    real colvector atoms
    real matrix out

    if (!vckss_rng__trials_ok(trials)) return(J(0,0,.))
    if (max(trials) > vckss_rng__max_binomial_trials()) {
        return(vckss_rng__draw_probe_scalar(trials))
    }
    atoms = vckss_rng__draw_probe_vector(trials)
    if (rows(atoms) != rows(trials) | hasmissing(atoms)) {
        return(J(0,0,.))
    }
    out = atoms,J(rows(trials),1,0)
    if (any(trials :> 0)) out[1,2] = 1
    return(out)
}

struct vckss_rng__result scalar vckss_rng__empty_result()
{
    struct vckss_rng__result scalar out

    out.status = "RNG_INPUT_INVALID"
    out.message = "RNG input is invalid"
    out.contract_version = ""
    out.candidate = ""
    out.runtime = strofreal(st_numscalar("c(stata_version)"),"%9.0g")
    out.domain = ""
    out.call_shape = "vector-parameter-or-scalar-chunk-canonical-atoms-v2"
    out.master_seed = .
    out.probe_start = .
    out.probe_count = .
    out.stream_first = .
    out.stream_last = .
    out.maximum_trials = vckss_rng__max_binomial_trials()
    out.chunk_calls = 0
    out.semantic_rank = J(0,1,.)
    out.canonical_order = J(0,1,.)
    out.atoms = J(0,0,.)
    return(out)
}

struct vckss_rng__result scalar vckss_rng__failure(
    string scalar status,
    string scalar message)
{
    struct vckss_rng__result scalar out

    out = vckss_rng__empty_result()
    out.status = status
    out.message = message
    return(out)
}

struct vckss_rng__result scalar vckss_rng__generate(
    string scalar candidate,
    real scalar master_seed,
    string scalar domain,
    real scalar probe_start,
    real scalar probe_count,
    real colvector semantic_rank,
    real colvector trials)
{
    struct vckss_rng__result scalar out
    struct vckss_rng__stream_snapshot scalar saved
    real scalar probe, finish, output_column, stream, rc, restore_rc
    real matrix generated
    real colvector canonical_order, canonical_trials, touched_streams

    out = vckss_rng__empty_result()
    if (!vckss_rng__candidate_ok(candidate)) {
        return(vckss_rng__failure(
            "RNG_CANDIDATE_INVALID","unknown K1 RNG candidate"))
    }
    if (!vckss_rng__runtime_registered()) {
        return(vckss_rng__failure(
            "RNG_RUNTIME_UNREGISTERED","Stata runtime is not registered"))
    }
    if (!vckss_rng__domain_ok(domain)) {
        return(vckss_rng__failure(
            "RNG_DOMAIN_INVALID","RNG domain must be leverage or target"))
    }
    if (missing(master_seed) | master_seed < 0 |
        master_seed > 2147483647 | master_seed != floor(master_seed)) {
        return(vckss_rng__failure(
            "RNG_SEED_INVALID","master seed is outside Stata's domain"))
    }
    if (missing(probe_start) | missing(probe_count) | probe_start < 1 |
        probe_count < 1 | probe_start != floor(probe_start) |
        probe_count != floor(probe_count)) {
        return(vckss_rng__failure(
            "RNG_PROBE_RANGE_INVALID","probe range is invalid"))
    }
    finish = probe_start+probe_count-1
    if (finish > vckss_rng__maximum_probes()) {
        return(vckss_rng__failure(
            "RNG_PROBE_RANGE_INVALID","probe range exceeds K1 registry"))
    }
    if (rows(semantic_rank) != rows(trials) |
        !vckss_rng__trials_ok(trials)) {
        return(vckss_rng__failure(
            "BINOMIAL_CONTRACT_UNSUPPORTED",
            "trial counts violate the exact chunk contract"))
    }
    canonical_order = vckss_rng__canonical_order(semantic_rank)
    if (rows(canonical_order) != rows(semantic_rank)) {
        return(vckss_rng__failure(
            "RNG_SEMANTIC_KEY_INVALID",
            "semantic atom ranks must be exact positive and unique"))
    }
    canonical_trials = trials[canonical_order]
    out.contract_version = vckss_rng__candidate_version(candidate)
    out.candidate = candidate
    out.runtime = strofreal(st_numscalar("c(stata_version)"),"%9.0g")
    out.domain = domain
    out.master_seed = master_seed
    out.probe_start = probe_start
    out.probe_count = probe_count
    out.semantic_rank = semantic_rank[canonical_order]
    out.canonical_order = canonical_order
    out.atoms = J(rows(trials),probe_count,.)

    if (candidate == "per_domain_stream") {
        stream = vckss_rng__domain_stream(domain)
        touched_streams = J(1,1,stream)
        out.stream_first = stream
        out.stream_last = stream
    }
    else {
        touched_streams = J(probe_count,1,.)
        for (probe=probe_start; probe<=finish; probe++) {
            touched_streams[probe-probe_start+1] =
                vckss_rng__probe_stream(domain,probe)
        }
        out.stream_first = touched_streams[1]
        out.stream_last = touched_streams[rows(touched_streams)]
    }
    saved = vckss_rng__capture_streams(touched_streams)
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream states could not be captured"
        return(out)
    }

    if (candidate == "per_domain_stream") {
        rc = vckss_rng__set_stream_seed(stream,master_seed)
        if (rc) {
            restore_rc = vckss_rng__restore_streams(saved)
            out.status = "RNG_SETUP_FAILED"
            out.message = "could not initialize fixed-domain mt64s stream"
            if (restore_rc) {
                out.status = "RNG_RESTORE_FAILED"
                out.message = "caller RNG state could not be restored"
            }
            return(out)
        }
        // Replay complete earlier probes.  This is deliberately independent
        // of solver batching; integration may instead carry the stream state
        // forward while preserving this exact logical order.
        for (probe=1; probe<=finish; probe++) {
            generated = vckss_rng__draw_probe_registered(canonical_trials)
            out.chunk_calls = out.chunk_calls+sum(generated[.,2])
            if (probe >= probe_start) {
                out.atoms[.,probe-probe_start+1] = generated[.,1]
            }
        }
    }
    else {
        for (probe=probe_start; probe<=finish; probe++) {
            stream = vckss_rng__probe_stream(domain,probe)
            rc = vckss_rng__set_stream_seed(stream,master_seed)
            if (rc) {
                restore_rc = vckss_rng__restore_streams(saved)
                out.status = "RNG_SETUP_FAILED"
                out.message = "could not initialize per-probe mt64s stream"
                if (restore_rc) {
                    out.status = "RNG_RESTORE_FAILED"
                    out.message = "caller RNG state could not be restored"
                }
                return(out)
            }
            generated = vckss_rng__draw_probe_registered(canonical_trials)
            out.chunk_calls = out.chunk_calls+sum(generated[.,2])
            output_column = probe-probe_start+1
            out.atoms[.,output_column] = generated[.,1]
        }
    }
    restore_rc = vckss_rng__restore_streams(saved)
    if (restore_rc) {
        out.status = "RNG_RESTORE_FAILED"
        out.message = "caller RNG state could not be restored"
        return(out)
    }
    out.status = "OK"
    out.message = "candidate probe atoms generated in canonical order"
    return(out)
}

struct vckss_rng__cursor scalar vckss_rng__open_cursor(
    real scalar master_seed,
    string scalar domain,
    real colvector semantic_rank,
    real colvector trials)
{
    struct vckss_rng__cursor scalar out
    struct vckss_rng__stream_snapshot scalar saved
    real scalar rc, restore_rc
    real colvector canonical_order

    out.status = "RNG_CURSOR_INVALID"
    out.message = "fixed-domain cursor input is invalid"
    out.contract_version = vckss_rng__production_contract()
    out.runtime = strofreal(st_numscalar("c(stata_version)"),"%9.0g")
    out.domain = domain
    out.call_shape = "vector-parameter-or-scalar-chunk-canonical-atoms-v2"
    out.master_seed = master_seed
    out.stream = vckss_rng__domain_stream(domain)
    out.next_probe = 1
    out.generator_state = ""
    out.semantic_rank = J(0,1,.)
    out.canonical_order = J(0,1,.)
    out.trials = J(0,1,.)
    if (!vckss_rng__runtime_registered()) {
        out.status = "RNG_RUNTIME_UNREGISTERED"
        out.message = "Stata runtime is not registered"
        return(out)
    }
    if (!vckss_rng__domain_ok(domain) | missing(master_seed) |
        master_seed < 0 | master_seed > 2147483647 |
        master_seed != floor(master_seed) |
        rows(semantic_rank) != rows(trials) |
        !vckss_rng__trials_ok(trials)) return(out)
    canonical_order = vckss_rng__canonical_order(semantic_rank)
    if (rows(canonical_order) != rows(semantic_rank)) {
        out.status = "RNG_SEMANTIC_KEY_INVALID"
        out.message = "semantic atom ranks must be exact positive and unique"
        return(out)
    }
    saved = vckss_rng__capture_streams(J(1,1,out.stream))
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream state could not be captured"
        return(out)
    }
    rc = vckss_rng__set_stream_seed(out.stream,master_seed)
    if (!rc) out.generator_state = rngstate()
    restore_rc = vckss_rng__restore_streams(saved)
    if (restore_rc) {
        out.status = "RNG_RESTORE_FAILED"
        out.message = "caller RNG state could not be restored"
        return(out)
    }
    if (rc) {
        out.status = "RNG_SETUP_FAILED"
        out.message = "could not initialize fixed-domain cursor"
        return(out)
    }
    out.semantic_rank = semantic_rank[canonical_order]
    out.canonical_order = canonical_order
    out.trials = trials[canonical_order]
    out.status = "OK"
    out.message = "fixed-domain cursor initialized"
    return(out)
}

struct vckss_rng__result scalar vckss_rng__cursor_next(
    pointer(struct vckss_rng__cursor scalar) scalar cursor,
    real scalar probe_count)
{
    struct vckss_rng__result scalar out
    struct vckss_rng__stream_snapshot scalar saved
    real scalar rc, restore_rc, probe, first_probe
    real matrix generated
    string scalar next_state

    if (cursor == NULL | missing(probe_count) | probe_count < 1 |
        probe_count != floor(probe_count)) {
        return(vckss_rng__failure(
            "RNG_CURSOR_INVALID","fixed-domain cursor request is invalid"))
    }
    if ((*cursor).status != "OK" |
        (*cursor).next_probe+probe_count-1 >
            vckss_rng__maximum_exact_integer()) {
        return(vckss_rng__failure(
            "RNG_PROBE_RANGE_INVALID","cursor probe range is invalid"))
    }
    out = vckss_rng__empty_result()
    first_probe = (*cursor).next_probe
    out.contract_version = (*cursor).contract_version
    out.candidate = "per_domain_stream"
    out.runtime = (*cursor).runtime
    out.domain = (*cursor).domain
    out.master_seed = (*cursor).master_seed
    out.probe_start = first_probe
    out.probe_count = probe_count
    out.stream_first = (*cursor).stream
    out.stream_last = (*cursor).stream
    out.semantic_rank = (*cursor).semantic_rank
    out.canonical_order = (*cursor).canonical_order
    out.atoms = J(rows((*cursor).trials),probe_count,.)

    saved = vckss_rng__capture_streams(J(1,1,(*cursor).stream))
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream state could not be captured"
        return(out)
    }
    rc = _stata("set rng mt64s",1,1)
    if (!rc) rc = _stata("set rngstream "+
        strofreal((*cursor).stream,"%9.0f"),1,1)
    if (!rc) rngstate((*cursor).generator_state)
    if (rc) {
        restore_rc = vckss_rng__restore_streams(saved)
        out.status = restore_rc ? "RNG_RESTORE_FAILED" : "RNG_SETUP_FAILED"
        out.message = "could not resume fixed-domain cursor"
        return(out)
    }
    for (probe=1; probe<=probe_count; probe++) {
        generated = vckss_rng__draw_probe_registered((*cursor).trials)
        out.atoms[.,probe] = generated[.,1]
        out.chunk_calls = out.chunk_calls+sum(generated[.,2])
    }
    next_state = rngstate()
    restore_rc = vckss_rng__restore_streams(saved)
    if (restore_rc) {
        out.status = "RNG_RESTORE_FAILED"
        out.message = "caller RNG state could not be restored"
        return(out)
    }
    (*cursor).generator_state = next_state
    (*cursor).next_probe = first_probe+probe_count
    out.status = "OK"
    out.message = "fixed-domain cursor batch generated"
    return(out)
}

struct vckss_rng__call_shape_result scalar vckss_rng__compare_call_shapes(
    real scalar master_seed,
    real scalar stream,
    real colvector trials)
{
    struct vckss_rng__call_shape_result scalar out
    struct vckss_rng__stream_snapshot scalar saved
    real scalar rc, restore_rc
    real matrix scalar_draw

    out.status = "BINOMIAL_CONTRACT_UNSUPPORTED"
    out.message = "call-shape comparison input is invalid"
    out.scalar_atoms = J(0,1,.)
    out.vector_atoms = J(0,1,.)
    out.scalar_state = ""
    out.vector_state = ""
    out.atoms_equal = .
    out.states_equal = .
    if (!vckss_rng__runtime_registered() |
        missing(master_seed) | master_seed < 0 |
        master_seed > 2147483647 | master_seed != floor(master_seed) |
        missing(stream) | stream < 1 | stream > 32768 |
        stream != floor(stream) | !vckss_rng__trials_ok(trials) |
        max(trials) > vckss_rng__max_binomial_trials()) return(out)

    saved = vckss_rng__capture_streams(J(1,1,stream))
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream state could not be captured"
        return(out)
    }
    rc = vckss_rng__set_stream_seed(stream,master_seed)
    if (rc) {
        restore_rc = vckss_rng__restore_streams(saved)
        out.status = restore_rc ? "RNG_RESTORE_FAILED" : "RNG_SETUP_FAILED"
        out.message = "could not initialize scalar comparison stream"
        return(out)
    }
    scalar_draw = vckss_rng__draw_probe_scalar(trials)
    out.scalar_atoms = scalar_draw[.,1]
    out.scalar_state = rngstate()

    rc = vckss_rng__set_stream_seed(stream,master_seed)
    if (rc) {
        restore_rc = vckss_rng__restore_streams(saved)
        out.status = restore_rc ? "RNG_RESTORE_FAILED" : "RNG_SETUP_FAILED"
        out.message = "could not initialize vector comparison stream"
        return(out)
    }
    out.vector_atoms = vckss_rng__draw_probe_vector(trials)
    out.vector_state = rngstate()
    out.atoms_equal = all(out.scalar_atoms :== out.vector_atoms)
    out.states_equal = out.scalar_state == out.vector_state
    restore_rc = vckss_rng__restore_streams(saved)
    if (restore_rc) {
        out.status = "RNG_RESTORE_FAILED"
        out.message = "caller RNG state could not be restored"
        return(out)
    }
    out.status = "OK"
    out.message = "scalar and vectorized call shapes compared"
    return(out)
}

struct vckss_rng__benchmark_result scalar vckss_rng__benchmark(
    real scalar master_seed,
    string scalar domain,
    real scalar probes,
    real colvector semantic_rank,
    real colvector trials,
    real scalar repetitions)
{
    struct vckss_rng__benchmark_result scalar out
    struct vckss_rng__result scalar generated
    real scalar repetition

    out.status = "RNG_BENCHMARK_INVALID"
    out.message = "candidate benchmark input is invalid"
    out.recommended_candidate = ""
    out.repetitions = repetitions
    out.per_probe_seconds = .
    out.per_domain_seconds = .
    out.per_probe_over_domain = .
    if (missing(repetitions) | repetitions < 1 |
        repetitions != floor(repetitions)) return(out)

    // Warm both candidates before the paired timing loop.  This evidence
    // helper owns timers 87 and 88; estimator execution never calls it.
    generated = vckss_rng__generate("per_probe_stream",master_seed,
        domain,1,probes,semantic_rank,trials)
    if (generated.status != "OK") {
        out.status = generated.status
        out.message = generated.message
        return(out)
    }
    generated = vckss_rng__generate("per_domain_stream",master_seed,
        domain,1,probes,semantic_rank,trials)
    if (generated.status != "OK") {
        out.status = generated.status
        out.message = generated.message
        return(out)
    }
    timer_clear(87)
    timer_clear(88)
    for (repetition=1; repetition<=repetitions; repetition++) {
        timer_on(87)
        generated = vckss_rng__generate("per_probe_stream",master_seed,
            domain,1,probes,semantic_rank,trials)
        timer_off(87)
        if (generated.status != "OK") {
            out.status = generated.status
            out.message = generated.message
            return(out)
        }
        timer_on(88)
        generated = vckss_rng__generate("per_domain_stream",master_seed,
            domain,1,probes,semantic_rank,trials)
        timer_off(88)
        if (generated.status != "OK") {
            out.status = generated.status
            out.message = generated.message
            return(out)
        }
    }
    out.per_probe_seconds = timer_value(87)[1]
    out.per_domain_seconds = timer_value(88)[1]
    if (out.per_domain_seconds > 0) {
        out.per_probe_over_domain =
            out.per_probe_seconds/out.per_domain_seconds
    }
    if (out.per_domain_seconds <= out.per_probe_seconds) {
        out.recommended_candidate = "per_domain_stream"
    }
    else out.recommended_candidate = "per_probe_stream"
    out.status = "OK"
    out.message = "paired candidate timing completed; recommendation is evidence-based"
    return(out)
}

VCKSS_RNG_CALLER_GUARD = vckss_rng__empty_guard()

end
