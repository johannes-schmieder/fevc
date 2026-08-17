*! kss_bc KSS-SCALE-1 RNG candidate module
*! version 0.2.0-dev 16aug2026

version 18.0

mata:
mata set matastrict on
mata set matalnum on

/*
The K1 checkpoint compares two execution-path-independent contracts.  Local
Stata 18 evidence selects the simpler qualifying fixed-domain cursor for the
installed experimental estimator.  Both candidates use the same canonical
semantic atom order and the registered vector-parameter call when every
trial count is within Stata's per-call limit.  Exact scalar chunking in that
same order is reserved for larger counts.
*/

struct kssbc_rng__snapshot
{
    string scalar algorithm
    real scalar stream
    string scalar state
}

struct kssbc_rng__full_snapshot
{
    string scalar status
    struct kssbc_rng__snapshot scalar active
    string scalar sort_state
    string scalar mt64s_stream1_state
    string scalar mt64s_stream2_state
    string scalar mt64s_selected_stream_state
}

struct kssbc_rng__stream_snapshot
{
    string scalar status
    struct kssbc_rng__snapshot scalar active
    real colvector stream
    string colvector state
}

struct kssbc_rng__result
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
    string colvector semantic_key
    real colvector canonical_order
    real matrix atoms
}

struct kssbc_rng__call_shape_result
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

struct kssbc_rng__benchmark_result
{
    string scalar status
    string scalar message
    string scalar recommended_candidate
    real scalar repetitions
    real scalar per_probe_seconds
    real scalar per_domain_seconds
    real scalar per_probe_over_domain
}

struct kssbc_rng__cursor
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
    string colvector semantic_key
    real colvector canonical_order
    real colvector trials
}

struct kssbc_rng__caller_guard
{
    real scalar active
    struct kssbc_rng__full_snapshot scalar saved
}

real scalar kssbc_rng__api_level()
{
    return(2)
}

string scalar kssbc_rng__build_id()
{
    return("kss-bc-rng-k1-mt64s-complete-guard-v2")
}

string scalar kssbc_rng__invariant_version()
{
    return("KSS-RNG-K1-INVARIANT-V1")
}

string scalar kssbc_rng__k1_recommendation()
{
    // Local Stata 18 evidence favors the stateful fixed-domain candidate.
    // This is not the installed contract until Stata 19 vectors qualify.
    return("per_domain_stream_cursor")
}

string scalar kssbc_rng__production_contract()
{
    real scalar runtime

    runtime = st_numscalar("c(stata_version)")
    if (!missing(runtime) & runtime >= 18 & runtime < 19) {
        return("KSS-MT64S-DOMAIN-CURSOR-V2-STATA18")
    }
    // Candidate generation remains available on Stata 19 so that its
    // golden vectors can be qualified.  Installed estimation fails closed
    // there until that runtime has its own registered contract (or is shown
    // to share the Stata 18 vectors exactly).
    return("")
}

real scalar kssbc_rng__max_binomial_trials()
{
    // Stata 18 documents rbinomial() on 1 <= n <= 1e+11.
    return(100000000000)
}

real scalar kssbc_rng__maximum_exact_integer()
{
    // Largest consecutive nonnegative integer exactly represented in double.
    return(2^53-1)
}

real scalar kssbc_rng__maximum_probes()
{
    // Leaves streams 32767 and 32768 for the fixed-domain candidate.
    return(16383)
}

string scalar kssbc_rng__candidate_version(string scalar candidate)
{
    if (candidate == "per_probe_stream") {
        return("KSS-MT64S-PER-PROBE-CANDIDATE-V2")
    }
    if (candidate == "per_domain_stream") {
        return("KSS-MT64S-PER-DOMAIN-CANDIDATE-V2")
    }
    return("")
}

real scalar kssbc_rng__candidate_ok(string scalar candidate)
{
    return(candidate == "per_probe_stream" |
        candidate == "per_domain_stream")
}

real scalar kssbc_rng__domain_ok(string scalar domain)
{
    return(domain == "leverage" | domain == "target")
}

real scalar kssbc_rng__runtime_registered()
{
    real scalar runtime

    runtime = st_numscalar("c(stata_version)")
    return(!missing(runtime) & runtime >= 18 & runtime < 20)
}

struct kssbc_rng__snapshot scalar kssbc_rng__capture()
{
    struct kssbc_rng__snapshot scalar out

    out.algorithm = st_global("c(rng)")
    out.stream = st_numscalar("c(rngstream)")
    out.state = rngstate()
    return(out)
}

real scalar kssbc_rng__restore(
    struct kssbc_rng__snapshot scalar saved)
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

struct kssbc_rng__stream_snapshot scalar kssbc_rng__capture_streams(
    real colvector streams)
{
    struct kssbc_rng__stream_snapshot scalar out
    real scalar index, rc, restore_rc

    out.status = "INVALID"
    out.active = kssbc_rng__capture()
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
    restore_rc = kssbc_rng__restore(out.active)
    if (rc | restore_rc) return(out)
    out.status = "OK"
    return(out)
}

real scalar kssbc_rng__restore_streams(
    struct kssbc_rng__stream_snapshot scalar saved)
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
    active_rc = kssbc_rng__restore(saved.active)
    if (!first_rc) first_rc = active_rc
    return(first_rc)
}

struct kssbc_rng__full_snapshot scalar kssbc_rng__capture_full()
{
    struct kssbc_rng__full_snapshot scalar out
    real scalar rc, restore_rc

    out.status = "INVALID"
    out.active = kssbc_rng__capture()
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
    restore_rc = kssbc_rng__restore(out.active)
    if (rc | restore_rc | out.mt64s_stream1_state == "" |
        out.mt64s_stream2_state == "" |
        out.mt64s_selected_stream_state == "") return(out)
    out.status = "OK"
    return(out)
}

real scalar kssbc_rng__restore_full(
    struct kssbc_rng__full_snapshot scalar saved)
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
    if (!rc) rc = kssbc_rng__restore(saved.active)
    if (!rc) rc = _stata("set sortrngstate "+saved.sort_state,1,1)
    return(rc)
}

struct kssbc_rng__caller_guard scalar kssbc_rng__empty_guard()
{
    struct kssbc_rng__caller_guard scalar out

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

real scalar kssbc_rng__guard_begin()
{
    external struct kssbc_rng__caller_guard scalar KSSBC_RNG_CALLER_GUARD

    if (KSSBC_RNG_CALLER_GUARD.active) return(498)
    KSSBC_RNG_CALLER_GUARD.saved = kssbc_rng__capture_full()
    if (KSSBC_RNG_CALLER_GUARD.saved.status != "OK") return(498)
    KSSBC_RNG_CALLER_GUARD.active = 1
    return(0)
}

real scalar kssbc_rng__guard_restore()
{
    external struct kssbc_rng__caller_guard scalar KSSBC_RNG_CALLER_GUARD
    real scalar rc

    if (!KSSBC_RNG_CALLER_GUARD.active) return(0)
    rc = kssbc_rng__restore_full(KSSBC_RNG_CALLER_GUARD.saved)
    if (!rc) KSSBC_RNG_CALLER_GUARD = kssbc_rng__empty_guard()
    return(rc)
}

real scalar kssbc_rng__set_stream_seed(
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

real scalar kssbc_rng__probe_stream(
    string scalar domain,
    real scalar probe)
{
    if (!kssbc_rng__domain_ok(domain) | missing(probe) | probe < 1 |
        probe > kssbc_rng__maximum_probes() | probe != floor(probe)) {
        return(.)
    }
    if (domain == "leverage") return(probe)
    return(kssbc_rng__maximum_probes()+probe)
}

real scalar kssbc_rng__domain_stream(string scalar domain)
{
    // Candidate namespaces are versioned separately.  Low stream numbers
    // avoid the measurable setup cost of jumping to the end of the registry.
    if (domain == "leverage") return(1)
    if (domain == "target") return(2)
    return(.)
}

real colvector kssbc_rng__canonical_order(string colvector semantic_key)
{
    real colvector sorted
    string colvector canonical

    if (cols(semantic_key) != 1 | rows(semantic_key) < 1 |
        any(semantic_key :== "")) return(J(0,1,.))
    sorted = order(semantic_key,1)
    canonical = semantic_key[sorted]
    if (rows(canonical) > 1) {
        if (any(canonical[|2\rows(canonical)|] :==
            canonical[|1\rows(canonical)-1|])) return(J(0,1,.))
    }
    return(sorted)
}

real scalar kssbc_rng__trials_ok(real colvector trials)
{
    real scalar atom, total

    if (cols(trials) != 1 | rows(trials) < 1 | hasmissing(trials) |
        any(trials :< 0) | any(trials :!= floor(trials)) |
        any(trials :> kssbc_rng__maximum_exact_integer())) return(0)
    total = 0
    for (atom=1; atom<=rows(trials); atom++) {
        // Subtract before adding so a rounded over-limit sum cannot pass.
        if (trials[atom] > kssbc_rng__maximum_exact_integer()-total) {
            return(0)
        }
        total = total+trials[atom]
    }
    return(1)
}

real rowvector kssbc_rng__binomial_atom_limit(
    real scalar trials,
    real scalar maximum_trials)
{
    real scalar successes, remaining, chunk, calls

    if (missing(trials) | trials < 0 | trials != floor(trials) |
        trials > kssbc_rng__maximum_exact_integer() |
        missing(maximum_trials) | maximum_trials < 1 |
        maximum_trials > kssbc_rng__max_binomial_trials() |
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

real rowvector kssbc_rng__binomial_atom_scalar(real scalar trials)
{
    return(kssbc_rng__binomial_atom_limit(
        trials,kssbc_rng__max_binomial_trials()))
}

real matrix kssbc_rng__draw_probe_scalar(real colvector trials)
{
    real scalar atom
    real rowvector draw
    real matrix out

    out = J(rows(trials),2,0)
    for (atom=1; atom<=rows(trials); atom++) {
        draw = kssbc_rng__binomial_atom_scalar(trials[atom])
        out[atom,.] = draw
    }
    return(out)
}

real colvector kssbc_rng__draw_probe_vector(real colvector trials)
{
    real colvector positive, out

    if (!kssbc_rng__trials_ok(trials) |
        max(trials) > kssbc_rng__max_binomial_trials()) {
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

real matrix kssbc_rng__draw_probe_registered(real colvector trials)
{
    real colvector atoms
    real matrix out

    if (!kssbc_rng__trials_ok(trials)) return(J(0,0,.))
    if (max(trials) > kssbc_rng__max_binomial_trials()) {
        return(kssbc_rng__draw_probe_scalar(trials))
    }
    atoms = kssbc_rng__draw_probe_vector(trials)
    if (rows(atoms) != rows(trials) | hasmissing(atoms)) {
        return(J(0,0,.))
    }
    out = atoms,J(rows(trials),1,0)
    if (any(trials :> 0)) out[1,2] = 1
    return(out)
}

struct kssbc_rng__result scalar kssbc_rng__empty_result()
{
    struct kssbc_rng__result scalar out

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
    out.maximum_trials = kssbc_rng__max_binomial_trials()
    out.chunk_calls = 0
    out.semantic_key = J(0,1,"")
    out.canonical_order = J(0,1,.)
    out.atoms = J(0,0,.)
    return(out)
}

struct kssbc_rng__result scalar kssbc_rng__failure(
    string scalar status,
    string scalar message)
{
    struct kssbc_rng__result scalar out

    out = kssbc_rng__empty_result()
    out.status = status
    out.message = message
    return(out)
}

struct kssbc_rng__result scalar kssbc_rng__generate(
    string scalar candidate,
    real scalar master_seed,
    string scalar domain,
    real scalar probe_start,
    real scalar probe_count,
    string colvector semantic_key,
    real colvector trials)
{
    struct kssbc_rng__result scalar out
    struct kssbc_rng__stream_snapshot scalar saved
    real scalar probe, finish, output_column, stream, rc, restore_rc
    real matrix generated
    real colvector canonical_order, canonical_trials, touched_streams

    out = kssbc_rng__empty_result()
    if (!kssbc_rng__candidate_ok(candidate)) {
        return(kssbc_rng__failure(
            "RNG_CANDIDATE_INVALID","unknown K1 RNG candidate"))
    }
    if (!kssbc_rng__runtime_registered()) {
        return(kssbc_rng__failure(
            "RNG_RUNTIME_UNREGISTERED","Stata runtime is not registered"))
    }
    if (!kssbc_rng__domain_ok(domain)) {
        return(kssbc_rng__failure(
            "RNG_DOMAIN_INVALID","RNG domain must be leverage or target"))
    }
    if (missing(master_seed) | master_seed < 0 |
        master_seed > 2147483647 | master_seed != floor(master_seed)) {
        return(kssbc_rng__failure(
            "RNG_SEED_INVALID","master seed is outside Stata's domain"))
    }
    if (missing(probe_start) | missing(probe_count) | probe_start < 1 |
        probe_count < 1 | probe_start != floor(probe_start) |
        probe_count != floor(probe_count)) {
        return(kssbc_rng__failure(
            "RNG_PROBE_RANGE_INVALID","probe range is invalid"))
    }
    finish = probe_start+probe_count-1
    if (finish > kssbc_rng__maximum_probes()) {
        return(kssbc_rng__failure(
            "RNG_PROBE_RANGE_INVALID","probe range exceeds K1 registry"))
    }
    if (rows(semantic_key) != rows(trials) |
        !kssbc_rng__trials_ok(trials)) {
        return(kssbc_rng__failure(
            "BINOMIAL_CONTRACT_UNSUPPORTED",
            "trial counts violate the exact chunk contract"))
    }
    canonical_order = kssbc_rng__canonical_order(semantic_key)
    if (rows(canonical_order) != rows(semantic_key)) {
        return(kssbc_rng__failure(
            "RNG_SEMANTIC_KEY_INVALID",
            "semantic atom keys must be nonempty and unique"))
    }
    canonical_trials = trials[canonical_order]
    out.contract_version = kssbc_rng__candidate_version(candidate)
    out.candidate = candidate
    out.runtime = strofreal(st_numscalar("c(stata_version)"),"%9.0g")
    out.domain = domain
    out.master_seed = master_seed
    out.probe_start = probe_start
    out.probe_count = probe_count
    out.semantic_key = semantic_key[canonical_order]
    out.canonical_order = canonical_order
    out.atoms = J(rows(trials),probe_count,.)

    if (candidate == "per_domain_stream") {
        stream = kssbc_rng__domain_stream(domain)
        touched_streams = J(1,1,stream)
        out.stream_first = stream
        out.stream_last = stream
    }
    else {
        touched_streams = J(probe_count,1,.)
        for (probe=probe_start; probe<=finish; probe++) {
            touched_streams[probe-probe_start+1] =
                kssbc_rng__probe_stream(domain,probe)
        }
        out.stream_first = touched_streams[1]
        out.stream_last = touched_streams[rows(touched_streams)]
    }
    saved = kssbc_rng__capture_streams(touched_streams)
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream states could not be captured"
        return(out)
    }

    if (candidate == "per_domain_stream") {
        rc = kssbc_rng__set_stream_seed(stream,master_seed)
        if (rc) {
            restore_rc = kssbc_rng__restore_streams(saved)
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
            generated = kssbc_rng__draw_probe_registered(canonical_trials)
            out.chunk_calls = out.chunk_calls+sum(generated[.,2])
            if (probe >= probe_start) {
                out.atoms[.,probe-probe_start+1] = generated[.,1]
            }
        }
    }
    else {
        for (probe=probe_start; probe<=finish; probe++) {
            stream = kssbc_rng__probe_stream(domain,probe)
            rc = kssbc_rng__set_stream_seed(stream,master_seed)
            if (rc) {
                restore_rc = kssbc_rng__restore_streams(saved)
                out.status = "RNG_SETUP_FAILED"
                out.message = "could not initialize per-probe mt64s stream"
                if (restore_rc) {
                    out.status = "RNG_RESTORE_FAILED"
                    out.message = "caller RNG state could not be restored"
                }
                return(out)
            }
            generated = kssbc_rng__draw_probe_registered(canonical_trials)
            out.chunk_calls = out.chunk_calls+sum(generated[.,2])
            output_column = probe-probe_start+1
            out.atoms[.,output_column] = generated[.,1]
        }
    }
    restore_rc = kssbc_rng__restore_streams(saved)
    if (restore_rc) {
        out.status = "RNG_RESTORE_FAILED"
        out.message = "caller RNG state could not be restored"
        return(out)
    }
    out.status = "OK"
    out.message = "candidate probe atoms generated in canonical order"
    return(out)
}

struct kssbc_rng__cursor scalar kssbc_rng__open_cursor(
    real scalar master_seed,
    string scalar domain,
    string colvector semantic_key,
    real colvector trials)
{
    struct kssbc_rng__cursor scalar out
    struct kssbc_rng__stream_snapshot scalar saved
    real scalar rc, restore_rc
    real colvector canonical_order

    out.status = "RNG_CURSOR_INVALID"
    out.message = "fixed-domain cursor input is invalid"
    out.contract_version =
        kssbc_rng__candidate_version("per_domain_stream")
    out.runtime = strofreal(st_numscalar("c(stata_version)"),"%9.0g")
    out.domain = domain
    out.call_shape = "vector-parameter-or-scalar-chunk-canonical-atoms-v2"
    out.master_seed = master_seed
    out.stream = kssbc_rng__domain_stream(domain)
    out.next_probe = 1
    out.generator_state = ""
    out.semantic_key = J(0,1,"")
    out.canonical_order = J(0,1,.)
    out.trials = J(0,1,.)
    if (!kssbc_rng__runtime_registered()) {
        out.status = "RNG_RUNTIME_UNREGISTERED"
        out.message = "Stata runtime is not registered"
        return(out)
    }
    if (!kssbc_rng__domain_ok(domain) | missing(master_seed) |
        master_seed < 0 | master_seed > 2147483647 |
        master_seed != floor(master_seed) |
        rows(semantic_key) != rows(trials) |
        !kssbc_rng__trials_ok(trials)) return(out)
    canonical_order = kssbc_rng__canonical_order(semantic_key)
    if (rows(canonical_order) != rows(semantic_key)) {
        out.status = "RNG_SEMANTIC_KEY_INVALID"
        out.message = "semantic atom keys must be nonempty and unique"
        return(out)
    }
    saved = kssbc_rng__capture_streams(J(1,1,out.stream))
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream state could not be captured"
        return(out)
    }
    rc = kssbc_rng__set_stream_seed(out.stream,master_seed)
    if (!rc) out.generator_state = rngstate()
    restore_rc = kssbc_rng__restore_streams(saved)
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
    out.semantic_key = semantic_key[canonical_order]
    out.canonical_order = canonical_order
    out.trials = trials[canonical_order]
    out.status = "OK"
    out.message = "fixed-domain cursor initialized"
    return(out)
}

struct kssbc_rng__result scalar kssbc_rng__cursor_next(
    pointer(struct kssbc_rng__cursor scalar) scalar cursor,
    real scalar probe_count)
{
    struct kssbc_rng__result scalar out
    struct kssbc_rng__stream_snapshot scalar saved
    real scalar rc, restore_rc, probe, first_probe
    real matrix generated
    string scalar next_state

    if (cursor == NULL | missing(probe_count) | probe_count < 1 |
        probe_count != floor(probe_count)) {
        return(kssbc_rng__failure(
            "RNG_CURSOR_INVALID","fixed-domain cursor request is invalid"))
    }
    if ((*cursor).status != "OK" |
        (*cursor).next_probe+probe_count-1 > kssbc_rng__maximum_probes()) {
        return(kssbc_rng__failure(
            "RNG_PROBE_RANGE_INVALID","cursor probe range is invalid"))
    }
    out = kssbc_rng__empty_result()
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
    out.semantic_key = (*cursor).semantic_key
    out.canonical_order = (*cursor).canonical_order
    out.atoms = J(rows((*cursor).trials),probe_count,.)

    saved = kssbc_rng__capture_streams(J(1,1,(*cursor).stream))
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
        restore_rc = kssbc_rng__restore_streams(saved)
        out.status = restore_rc ? "RNG_RESTORE_FAILED" : "RNG_SETUP_FAILED"
        out.message = "could not resume fixed-domain cursor"
        return(out)
    }
    for (probe=1; probe<=probe_count; probe++) {
        generated = kssbc_rng__draw_probe_registered((*cursor).trials)
        out.atoms[.,probe] = generated[.,1]
        out.chunk_calls = out.chunk_calls+sum(generated[.,2])
    }
    next_state = rngstate()
    restore_rc = kssbc_rng__restore_streams(saved)
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

struct kssbc_rng__call_shape_result scalar kssbc_rng__compare_call_shapes(
    real scalar master_seed,
    real scalar stream,
    real colvector trials)
{
    struct kssbc_rng__call_shape_result scalar out
    struct kssbc_rng__stream_snapshot scalar saved
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
    if (!kssbc_rng__runtime_registered() |
        missing(master_seed) | master_seed < 0 |
        master_seed > 2147483647 | master_seed != floor(master_seed) |
        missing(stream) | stream < 1 | stream > 32768 |
        stream != floor(stream) | !kssbc_rng__trials_ok(trials) |
        max(trials) > kssbc_rng__max_binomial_trials()) return(out)

    saved = kssbc_rng__capture_streams(J(1,1,stream))
    if (saved.status != "OK") {
        out.status = "RNG_STATE_CAPTURE_FAILED"
        out.message = "caller mt64s stream state could not be captured"
        return(out)
    }
    rc = kssbc_rng__set_stream_seed(stream,master_seed)
    if (rc) {
        restore_rc = kssbc_rng__restore_streams(saved)
        out.status = restore_rc ? "RNG_RESTORE_FAILED" : "RNG_SETUP_FAILED"
        out.message = "could not initialize scalar comparison stream"
        return(out)
    }
    scalar_draw = kssbc_rng__draw_probe_scalar(trials)
    out.scalar_atoms = scalar_draw[.,1]
    out.scalar_state = rngstate()

    rc = kssbc_rng__set_stream_seed(stream,master_seed)
    if (rc) {
        restore_rc = kssbc_rng__restore_streams(saved)
        out.status = restore_rc ? "RNG_RESTORE_FAILED" : "RNG_SETUP_FAILED"
        out.message = "could not initialize vector comparison stream"
        return(out)
    }
    out.vector_atoms = kssbc_rng__draw_probe_vector(trials)
    out.vector_state = rngstate()
    out.atoms_equal = all(out.scalar_atoms :== out.vector_atoms)
    out.states_equal = out.scalar_state == out.vector_state
    restore_rc = kssbc_rng__restore_streams(saved)
    if (restore_rc) {
        out.status = "RNG_RESTORE_FAILED"
        out.message = "caller RNG state could not be restored"
        return(out)
    }
    out.status = "OK"
    out.message = "scalar and vectorized call shapes compared"
    return(out)
}

struct kssbc_rng__benchmark_result scalar kssbc_rng__benchmark(
    real scalar master_seed,
    string scalar domain,
    real scalar probes,
    string colvector semantic_key,
    real colvector trials,
    real scalar repetitions)
{
    struct kssbc_rng__benchmark_result scalar out
    struct kssbc_rng__result scalar generated
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
    generated = kssbc_rng__generate("per_probe_stream",master_seed,
        domain,1,probes,semantic_key,trials)
    if (generated.status != "OK") {
        out.status = generated.status
        out.message = generated.message
        return(out)
    }
    generated = kssbc_rng__generate("per_domain_stream",master_seed,
        domain,1,probes,semantic_key,trials)
    if (generated.status != "OK") {
        out.status = generated.status
        out.message = generated.message
        return(out)
    }
    timer_clear(87)
    timer_clear(88)
    for (repetition=1; repetition<=repetitions; repetition++) {
        timer_on(87)
        generated = kssbc_rng__generate("per_probe_stream",master_seed,
            domain,1,probes,semantic_key,trials)
        timer_off(87)
        if (generated.status != "OK") {
            out.status = generated.status
            out.message = generated.message
            return(out)
        }
        timer_on(88)
        generated = kssbc_rng__generate("per_domain_stream",master_seed,
            domain,1,probes,semantic_key,trials)
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

KSSBC_RNG_CALLER_GUARD = kssbc_rng__empty_guard()

end
