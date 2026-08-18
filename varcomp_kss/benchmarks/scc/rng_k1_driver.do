version 18.0
clear all
set more off
set varabbrev off

args experiment_id bundle_sha source_commit job_id output_dir ///
    requested_slots_arg actual_slots_arg processors_arg

local requested_slots = real("`requested_slots_arg'")
local actual_slots = real("`actual_slots_arg'")
local requested_processors = real("`processors_arg'")

if !ustrregexm("`experiment_id'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`bundle_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !ustrregexm("`job_id'", "^[0-9]+$") | ///
    `requested_slots' != 14 | `actual_slots' != 14 | ///
    `requested_processors' != 4 {
    di as error "invalid RNG-K1 SCC driver arguments"
    exit 198
}

if c(stata_version) < 19 | c(stata_version) >= 20 | c(MP) != 1 {
    di as error "RNG-K1 compatibility evidence requires Stata/MP 19"
    exit 459
}
capture set processors `requested_processors'
if _rc | c(processors) != `requested_processors' {
    di as error "Stata/MP did not honor the four-processor request"
    exit 459
}

capture mkdir `"`output_dir'"'
confirm file "`c(pwd)'/varcomp_kss/varcomp_kss_rng.mata"
quietly do "`c(pwd)'/varcomp_kss/varcomp_kss_rng.mata"

/* Give the caller a non-mt64s active generator and a non-domain selected
   stream.  The receipt later certifies the active state, sort state, streams
   1/2, every per-probe stream, the call-shape streams, and stream 177. */
set rng mt64s
set rngstream 177
set seed 1357911
set rng kiss32
set seed 20260816

mata:
mata set matastrict on
mata set matalnum on

struct rngk1_evidence
{
    string scalar status
    string scalar per_probe_leverage_status
    string scalar per_probe_target_status
    string scalar per_domain_leverage_status
    string scalar per_domain_target_status
    string scalar per_probe_contract
    string scalar per_domain_contract
    string scalar leverage_timing_status
    string scalar target_timing_status
    string scalar leverage_recommendation
    string scalar target_recommendation
    string scalar per_probe_candidate_result
    string scalar per_domain_candidate_result
    real matrix per_probe_leverage
    real matrix per_probe_target
    real matrix per_domain_leverage
    real matrix per_domain_target
    real scalar module_contract_ok
    real scalar runtime_fail_closed
    real scalar golden_vectors_match
    real scalar canonical_order_invariant
    real scalar per_probe_partition_invariant
    real scalar per_domain_partition_invariant
    real scalar cursor_partition_invariant
    real scalar processor_per_probe_invariant
    real scalar processor_per_domain_invariant
    real scalar processor_switch_ok
    real scalar small_callshape_atoms_equal
    real scalar small_callshape_states_equal
    real scalar boundary_callshape_atoms_equal
    real scalar boundary_callshape_states_equal
    real scalar production_callshape_atoms_equal
    real scalar prod_callshape_states_equal
    real scalar exact_integer_gate_pass
    real scalar boundary_chunk_pass
    real scalar large_generation_pass
    real scalar boundary_atom
    real scalar chunked_atom
    real scalar chunked_calls
    real scalar production_scalar_seconds
    real scalar production_vector_seconds
    real scalar leverage_per_probe_seconds
    real scalar leverage_per_domain_seconds
    real scalar target_per_probe_seconds
    real scalar target_per_domain_seconds
    real scalar timing_selects_domain
    real scalar production_candidate_atoms
    real scalar production_candidate_probes
    real scalar production_candidate_repetitions
    real scalar prod_probe_median_seconds
    real scalar production_per_probe_min_seconds
    real scalar production_per_probe_max_seconds
    real scalar prod_domain_median_seconds
    real scalar prod_domain_min_seconds
    real scalar prod_domain_max_seconds
    real scalar production_domain_wins
    real scalar prod_timing_streams_restored
    real scalar probe_hidden_streams_restored
    real scalar per_probe_changed_stream_count
    real scalar per_probe_test_cleanup_restored
    real scalar ns_lev_streams_restored
    real scalar ns_tgt_streams_restored
    real scalar nonselected_caller_restored
    real scalar per_probe_candidate_qualified
    real scalar per_domain_candidate_qualified
    real scalar core_pass
    real scalar core_rc
    real scalar processor_restore_rc
    real scalar guard_restore_rc
    real scalar all_stream_restore_rc
    real scalar active_algorithm_restored
    real scalar active_stream_restored
    real scalar active_state_restored
    real scalar sort_state_restored
    real scalar domain_stream1_restored
    real scalar domain_stream2_restored
    real scalar selected_stream_state_restored
    real scalar domain_guard_full_restored
    real scalar every_touched_stream_restored
    real scalar overall_pass
}

struct rngk1_evidence scalar rngk1__empty()
{
    struct rngk1_evidence scalar out

    out.status = "NOT_RUN"
    out.per_probe_leverage_status = "NOT_RUN"
    out.per_probe_target_status = "NOT_RUN"
    out.per_domain_leverage_status = "NOT_RUN"
    out.per_domain_target_status = "NOT_RUN"
    out.per_probe_contract = ""
    out.per_domain_contract = ""
    out.leverage_timing_status = "NOT_RUN"
    out.target_timing_status = "NOT_RUN"
    out.leverage_recommendation = ""
    out.target_recommendation = ""
    out.per_probe_candidate_result = "NOT_RUN"
    out.per_domain_candidate_result = "NOT_RUN"
    out.per_probe_leverage = J(0,0,.)
    out.per_probe_target = J(0,0,.)
    out.per_domain_leverage = J(0,0,.)
    out.per_domain_target = J(0,0,.)
    out.module_contract_ok = 0
    out.runtime_fail_closed = 0
    out.golden_vectors_match = 0
    out.canonical_order_invariant = 0
    out.per_probe_partition_invariant = 0
    out.per_domain_partition_invariant = 0
    out.cursor_partition_invariant = 0
    out.processor_per_probe_invariant = 0
    out.processor_per_domain_invariant = 0
    out.processor_switch_ok = 0
    out.small_callshape_atoms_equal = 0
    out.small_callshape_states_equal = 0
    out.boundary_callshape_atoms_equal = 0
    out.boundary_callshape_states_equal = 0
    out.production_callshape_atoms_equal = 0
    out.prod_callshape_states_equal = 0
    out.exact_integer_gate_pass = 0
    out.boundary_chunk_pass = 0
    out.large_generation_pass = 0
    out.boundary_atom = .
    out.chunked_atom = .
    out.chunked_calls = .
    out.production_scalar_seconds = .
    out.production_vector_seconds = .
    out.leverage_per_probe_seconds = .
    out.leverage_per_domain_seconds = .
    out.target_per_probe_seconds = .
    out.target_per_domain_seconds = .
    out.timing_selects_domain = 0
    out.production_candidate_atoms = 50000
    out.production_candidate_probes = 40
    out.production_candidate_repetitions = 3
    out.prod_probe_median_seconds = .
    out.production_per_probe_min_seconds = .
    out.production_per_probe_max_seconds = .
    out.prod_domain_median_seconds = .
    out.prod_domain_min_seconds = .
    out.prod_domain_max_seconds = .
    out.production_domain_wins = .
    out.prod_timing_streams_restored = 0
    out.probe_hidden_streams_restored = 0
    out.per_probe_changed_stream_count = .
    out.per_probe_test_cleanup_restored = 0
    out.ns_lev_streams_restored = 0
    out.ns_tgt_streams_restored = 0
    out.nonselected_caller_restored = 0
    out.per_probe_candidate_qualified = 0
    out.per_domain_candidate_qualified = 0
    out.core_pass = 0
    out.core_rc = .
    out.processor_restore_rc = .
    out.guard_restore_rc = .
    out.all_stream_restore_rc = .
    out.active_algorithm_restored = 0
    out.active_stream_restored = 0
    out.active_state_restored = 0
    out.sort_state_restored = 0
    out.domain_stream1_restored = 0
    out.domain_stream2_restored = 0
    out.selected_stream_state_restored = 0
    out.domain_guard_full_restored = 0
    out.every_touched_stream_restored = 0
    out.overall_pass = 0
    return(out)
}

string colvector rngk1__capture_streams(real colvector streams)
{
    struct vckss_rng__snapshot scalar saved
    real scalar index, rc, restore_rc
    string colvector out

    out = J(rows(streams),1,"")
    saved = vckss_rng__capture()
    rc = _stata("set rng mt64s",1,1)
    for (index=1; index<=rows(streams) & !rc; index++) {
        rc = _stata("set rngstream "+
            strtrim(strofreal(streams[index],"%9.0f")),1,1)
        if (!rc) out[index] = rngstate()
    }
    restore_rc = vckss_rng__restore(saved)
    if (rc | restore_rc | any(out :== "")) return(J(0,1,""))
    return(out)
}

real scalar rngk1__restore_streams(
    real colvector streams,
    string colvector states)
{
    struct vckss_rng__snapshot scalar saved
    real scalar index, rc, restore_rc

    if (rows(streams) == 0 | rows(states) != rows(streams) |
        any(states :== "")) return(198)
    saved = vckss_rng__capture()
    rc = _stata("set rng mt64s",1,1)
    for (index=1; index<=rows(streams) & !rc; index++) {
        rc = _stata("set rngstream "+
            strtrim(strofreal(streams[index],"%9.0f")),1,1)
        if (!rc) rngstate(states[index])
    }
    restore_rc = vckss_rng__restore(saved)
    return(rc ? rc : restore_rc)
}

real scalar rngk1__full_equal(
    struct vckss_rng__full_snapshot scalar left,
    struct vckss_rng__full_snapshot scalar right)
{
    return(left.status == "OK" & right.status == "OK" &
        left.active.algorithm == right.active.algorithm &
        left.active.stream == right.active.stream &
        left.active.state == right.active.state &
        left.sort_state == right.sort_state &
        left.mt64s_stream1_state == right.mt64s_stream1_state &
        left.mt64s_stream2_state == right.mt64s_stream2_state &
        left.mt64s_selected_stream_state ==
            right.mt64s_selected_stream_state)
}

struct rngk1_evidence scalar rngk1__core()
{
    struct rngk1_evidence scalar out
    struct vckss_rng__result scalar plev, ptgt, dlev, dtgt
    struct vckss_rng__result scalar first, second, shuffled
    struct vckss_rng__result scalar pone_lev, pone_tgt, pfour_lev, pfour_tgt
    struct vckss_rng__result scalar done_lev, done_tgt, dfour_lev, dfour_tgt
    struct vckss_rng__result scalar large
    struct vckss_rng__result scalar prod_p_lev, prod_p_tgt
    struct vckss_rng__result scalar prod_d_lev, prod_d_tgt
    struct vckss_rng__cursor scalar cursor_all, cursor_split
    struct vckss_rng__result scalar cursor_whole, cursor_first, cursor_second
    struct vckss_rng__call_shape_result scalar shape, boundary_shape
    struct vckss_rng__benchmark_result scalar lev_timing, tgt_timing
    struct vckss_rng__snapshot scalar saved
    struct vckss_rng__snapshot scalar caller9_before, caller9_after
    struct vckss_rng__stream_snapshot scalar lev78_before, lev78_after
    struct vckss_rng__stream_snapshot scalar tgt78_before, tgt78_after
    struct vckss_rng__stream_snapshot scalar prod_before, prod_after
    string colvector pbefore, pafter, pclean
    string scalar chunk_state, manual_state, scalar_state, vector_state
    real colvector keys, prod_keys, trials, permutation, per_probe_streams
    real colvector production_trials
    real colvector prod_trials, prod_streams, probe_sorted, domain_sorted
    real matrix expected_plev, expected_dlev, expected_ptgt, expected_dtgt
    real matrix scalar_draw, production_candidate_timing
    real rowvector chunked
    real scalar manual_successes, manual_atom, rc1, rc4, cleanup_rc
    real scalar selected9_rc, atom_index, repetition, production_status_ok

    out = rngk1__empty()
    pone_lev = vckss_rng__empty_result()
    pone_tgt = vckss_rng__empty_result()
    pfour_lev = vckss_rng__empty_result()
    pfour_tgt = vckss_rng__empty_result()
    done_lev = vckss_rng__empty_result()
    done_tgt = vckss_rng__empty_result()
    dfour_lev = vckss_rng__empty_result()
    dfour_tgt = vckss_rng__empty_result()
    prod_p_lev = vckss_rng__empty_result()
    prod_p_tgt = vckss_rng__empty_result()
    prod_d_lev = vckss_rng__empty_result()
    prod_d_tgt = vckss_rng__empty_result()
    /* Numeric ranks 3,1,4,2 preserve the historical a,b,c,d canonical
       order without retaining one string per atom. */
    keys = (3 \ 1 \ 4 \ 2)
    trials = (7 \ 1 \ 19 \ 2)
    permutation = (3 \ 1 \ 4 \ 2)
    per_probe_streams = (1 \ 2 \ 3 \ 16384 \ 16385 \ 16386)
    expected_plev = (1,-1,1 \ -2,2,2 \ -3,3,3 \ -1,1,7)
    expected_dlev = (1,1,1 \ -2,0,0 \ -3,-3,-1 \ -1,3,-3)
    expected_ptgt = (-1,1,1 \ 0,0,0 \ 5,3,-3 \ 7,7,-3)
    expected_dtgt = (-1,1,-1 \ 2,0,0 \ 3,1,1 \ 1,3,-1)

    out.module_contract_ok =
        vckss_rng__api_level() == 2 &
        vckss_rng__build_id() ==
            "varcomp-kss-rng-k1-mt64s-complete-guard-v2" &
        vckss_rng__invariant_version() == "KSS-RNG-K1-INVARIANT-V1" &
        vckss_rng__k1_recommendation() ==
            "per_domain_stream_cursor" &
        vckss_rng__max_binomial_trials() == 100000000000 &
        vckss_rng__maximum_exact_integer() == 2^53-1
    out.runtime_fail_closed =
        st_numscalar("c(stata_version)") >= 19 &
        st_numscalar("c(stata_version)") < 20 &
        vckss_rng__production_contract() == ""

    /* Candidate one deliberately receives an all-touched-stream audit.  It
       is rejected unless streams 1--3 and 16384--16386 all restore. */
    pbefore = rngk1__capture_streams(per_probe_streams)
    plev = vckss_rng__generate(
        "per_probe_stream",8675309,"leverage",1,3,keys,trials)
    ptgt = vckss_rng__generate(
        "per_probe_stream",8675309,"target",1,3,keys,trials)
    out.per_probe_leverage_status = plev.status
    out.per_probe_target_status = ptgt.status
    out.per_probe_contract = plev.contract_version
    if (plev.status == "OK" & ptgt.status == "OK") {
        out.per_probe_leverage = plev.atoms
        out.per_probe_target = ptgt.atoms
        shuffled = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",1,3,
            keys[permutation],trials[permutation])
        out.canonical_order_invariant = shuffled.status == "OK"
        if (shuffled.status == "OK") {
            out.canonical_order_invariant =
                all(shuffled.semantic_rank :== plev.semantic_rank) &
                all(shuffled.atoms :== plev.atoms)
        }
        first = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",1,1,keys,trials)
        second = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",2,2,keys,trials)
        if (first.status == "OK" & second.status == "OK") {
            out.per_probe_partition_invariant =
                all((first.atoms,second.atoms) :== plev.atoms)
        }
    }
    rc1 = _stata("set processors 1",1,1)
    if (!rc1 & st_numscalar("c(processors)") == 1) {
        pone_lev = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",1,3,keys,trials)
        pone_tgt = vckss_rng__generate(
            "per_probe_stream",8675309,"target",1,3,keys,trials)
    }
    rc4 = _stata("set processors 4",1,1)
    if (!rc4 & st_numscalar("c(processors)") == 4) {
        pfour_lev = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",1,3,keys,trials)
        pfour_tgt = vckss_rng__generate(
            "per_probe_stream",8675309,"target",1,3,keys,trials)
    }
    out.processor_switch_ok = !rc1 & !rc4 &
        st_numscalar("c(processors)") == 4
    if (out.processor_switch_ok & pone_lev.status == "OK" &
        pone_tgt.status == "OK" & pfour_lev.status == "OK" &
        pfour_tgt.status == "OK") {
        out.processor_per_probe_invariant =
            all(pone_lev.atoms :== pfour_lev.atoms) &
            all(pone_tgt.atoms :== pfour_tgt.atoms)
    }
    lev_timing = vckss_rng__benchmark(
        8675309,"leverage",3,keys,trials,3)
    tgt_timing = vckss_rng__benchmark(
        8675309,"target",3,keys,trials,3)
    out.leverage_timing_status = lev_timing.status
    out.target_timing_status = tgt_timing.status
    out.leverage_recommendation = lev_timing.recommended_candidate
    out.target_recommendation = tgt_timing.recommended_candidate
    out.leverage_per_probe_seconds = lev_timing.per_probe_seconds
    out.leverage_per_domain_seconds = lev_timing.per_domain_seconds
    out.target_per_probe_seconds = tgt_timing.per_probe_seconds
    out.target_per_domain_seconds = tgt_timing.per_domain_seconds
    pafter = rngk1__capture_streams(per_probe_streams)
    if (rows(pbefore) == rows(per_probe_streams) &
        rows(pafter) == rows(per_probe_streams)) {
        out.probe_hidden_streams_restored = all(pbefore :== pafter)
        out.per_probe_changed_stream_count = sum(pbefore :!= pafter)
        cleanup_rc = rngk1__restore_streams(per_probe_streams,pbefore)
        pclean = rngk1__capture_streams(per_probe_streams)
        out.per_probe_test_cleanup_restored = cleanup_rc == 0 &
            rows(pclean) == rows(pbefore) & all(pclean :== pbefore)
    }

    /* Exact regression for the previously missed latent-state case: leave
       stream 9 selected under kiss32, touch nonselected leverage streams
       7/8 and target streams 16390/16391, and require all four plus the
       caller's active state to be unchanged. */
    saved = vckss_rng__capture()
    selected9_rc = _stata("set rng mt64s",1,1)
    if (!selected9_rc) selected9_rc = _stata("set rngstream 9",1,1)
    if (!selected9_rc) selected9_rc = _stata("set rng kiss32",1,1)
    if (!selected9_rc) rngstate(saved.state)
    if (!selected9_rc) {
        caller9_before = vckss_rng__capture()
        lev78_before = vckss_rng__capture_streams((7 \ 8))
        tgt78_before = vckss_rng__capture_streams((16390 \ 16391))
        first = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",7,2,keys,trials)
        second = vckss_rng__generate(
            "per_probe_stream",8675309,"target",7,2,keys,trials)
        lev78_after = vckss_rng__capture_streams((7 \ 8))
        tgt78_after = vckss_rng__capture_streams((16390 \ 16391))
        caller9_after = vckss_rng__capture()
        if (first.status == "OK" & lev78_before.status == "OK" &
            lev78_after.status == "OK") {
            out.ns_lev_streams_restored =
                all(lev78_before.state :== lev78_after.state)
        }
        if (second.status == "OK" & tgt78_before.status == "OK" &
            tgt78_after.status == "OK") {
            out.ns_tgt_streams_restored =
                all(tgt78_before.state :== tgt78_after.state)
        }
        out.nonselected_caller_restored =
            caller9_before.algorithm == caller9_after.algorithm &
            caller9_before.stream == 9 & caller9_after.stream == 9 &
            caller9_before.state == caller9_after.state
    }
    cleanup_rc = vckss_rng__restore(saved)
    out.nonselected_caller_restored =
        out.nonselected_caller_restored & cleanup_rc == 0

    /* Candidate two is the selected fixed-domain cursor.  The outer module
       guard, finalized by the Stata layer, must restore streams 1 and 2. */
    dlev = vckss_rng__generate(
        "per_domain_stream",8675309,"leverage",1,3,keys,trials)
    dtgt = vckss_rng__generate(
        "per_domain_stream",8675309,"target",1,3,keys,trials)
    out.per_domain_leverage_status = dlev.status
    out.per_domain_target_status = dtgt.status
    out.per_domain_contract = dlev.contract_version
    if (dlev.status == "OK" & dtgt.status == "OK") {
        out.per_domain_leverage = dlev.atoms
        out.per_domain_target = dtgt.atoms
        first = vckss_rng__generate(
            "per_domain_stream",8675309,"leverage",1,1,keys,trials)
        second = vckss_rng__generate(
            "per_domain_stream",8675309,"leverage",2,2,keys,trials)
        if (first.status == "OK" & second.status == "OK") {
            out.per_domain_partition_invariant =
                all((first.atoms,second.atoms) :== dlev.atoms)
        }
    }
    cursor_all = vckss_rng__open_cursor(8675309,"leverage",keys,trials)
    cursor_split = vckss_rng__open_cursor(8675309,"leverage",keys,trials)
    if (cursor_all.status == "OK" & cursor_split.status == "OK") {
        cursor_whole = vckss_rng__cursor_next(&cursor_all,3)
        cursor_first = vckss_rng__cursor_next(&cursor_split,1)
        cursor_second = vckss_rng__cursor_next(&cursor_split,2)
        if (cursor_whole.status == "OK" & cursor_first.status == "OK" &
            cursor_second.status == "OK") {
            out.cursor_partition_invariant =
                all((cursor_first.atoms,cursor_second.atoms) :==
                    cursor_whole.atoms) &
                all(cursor_whole.atoms :== dlev.atoms)
        }
    }
    rc1 = _stata("set processors 1",1,1)
    if (!rc1 & st_numscalar("c(processors)") == 1) {
        done_lev = vckss_rng__generate(
            "per_domain_stream",8675309,"leverage",1,3,keys,trials)
        done_tgt = vckss_rng__generate(
            "per_domain_stream",8675309,"target",1,3,keys,trials)
    }
    rc4 = _stata("set processors 4",1,1)
    if (!rc4 & st_numscalar("c(processors)") == 4) {
        dfour_lev = vckss_rng__generate(
            "per_domain_stream",8675309,"leverage",1,3,keys,trials)
        dfour_tgt = vckss_rng__generate(
            "per_domain_stream",8675309,"target",1,3,keys,trials)
    }
    out.processor_switch_ok = out.processor_switch_ok & !rc1 & !rc4 &
        st_numscalar("c(processors)") == 4
    if (out.processor_switch_ok & done_lev.status == "OK" &
        done_tgt.status == "OK" & dfour_lev.status == "OK" &
        dfour_tgt.status == "OK") {
        out.processor_per_domain_invariant =
            all(done_lev.atoms :== dfour_lev.atoms) &
            all(done_tgt.atoms :== dfour_tgt.atoms)
    }

    if (plev.status == "OK" & ptgt.status == "OK" &
        dlev.status == "OK" & dtgt.status == "OK") {
        out.golden_vectors_match =
            all(plev.atoms :== expected_plev) &
            all(dlev.atoms :== expected_dlev) &
            all(ptgt.atoms :== expected_ptgt) &
            all(dtgt.atoms :== expected_dtgt)
    }

    /* Production-shaped candidate timing: 50,000 semantic atoms, both
       domains, P40, and three paired repetitions.  generate() includes its
       complete touched-stream snapshot/restore overhead.  Tiny four-atom
       timings above are smoke diagnostics only and never select a route. */
    prod_keys = 1::out.production_candidate_atoms
    prod_trials = mod((1::out.production_candidate_atoms),97):+1
    prod_streams =
        (1::out.production_candidate_probes \
        (16384::(16383+out.production_candidate_probes)))
    prod_before = vckss_rng__capture_streams(prod_streams)
    production_status_ok = prod_before.status == "OK"
    prod_p_lev = vckss_rng__generate(
        "per_probe_stream",8675309,"leverage",1,
        out.production_candidate_probes,prod_keys,prod_trials)
    prod_p_tgt = vckss_rng__generate(
        "per_probe_stream",8675309,"target",1,
        out.production_candidate_probes,prod_keys,prod_trials)
    prod_d_lev = vckss_rng__generate(
        "per_domain_stream",8675309,"leverage",1,
        out.production_candidate_probes,prod_keys,prod_trials)
    prod_d_tgt = vckss_rng__generate(
        "per_domain_stream",8675309,"target",1,
        out.production_candidate_probes,prod_keys,prod_trials)
    production_status_ok = production_status_ok &
        prod_p_lev.status == "OK" & prod_p_tgt.status == "OK" &
        prod_d_lev.status == "OK" & prod_d_tgt.status == "OK"
    production_candidate_timing =
        J(out.production_candidate_repetitions,2,.)
    for (repetition=1;
        repetition<=out.production_candidate_repetitions; repetition++) {
        timer_clear(91)
        timer_on(91)
        prod_p_lev = vckss_rng__generate(
            "per_probe_stream",8675309,"leverage",1,
            out.production_candidate_probes,prod_keys,prod_trials)
        prod_p_tgt = vckss_rng__generate(
            "per_probe_stream",8675309,"target",1,
            out.production_candidate_probes,prod_keys,prod_trials)
        timer_off(91)
        production_candidate_timing[repetition,1] = timer_value(91)[1]
        timer_clear(92)
        timer_on(92)
        prod_d_lev = vckss_rng__generate(
            "per_domain_stream",8675309,"leverage",1,
            out.production_candidate_probes,prod_keys,prod_trials)
        prod_d_tgt = vckss_rng__generate(
            "per_domain_stream",8675309,"target",1,
            out.production_candidate_probes,prod_keys,prod_trials)
        timer_off(92)
        production_candidate_timing[repetition,2] = timer_value(92)[1]
        production_status_ok = production_status_ok &
            prod_p_lev.status == "OK" & prod_p_tgt.status == "OK" &
            prod_d_lev.status == "OK" & prod_d_tgt.status == "OK"
    }
    prod_after = vckss_rng__capture_streams(prod_streams)
    out.prod_timing_streams_restored =
        prod_after.status == "OK" & prod_before.status == "OK"
    if (out.prod_timing_streams_restored) {
        out.prod_timing_streams_restored =
            all(prod_before.state :== prod_after.state)
    }
    if (production_status_ok &
        !hasmissing(production_candidate_timing)) {
        probe_sorted = sort(production_candidate_timing[.,1],1)
        domain_sorted = sort(production_candidate_timing[.,2],1)
        out.production_per_probe_min_seconds = probe_sorted[1]
        out.prod_probe_median_seconds = probe_sorted[2]
        out.production_per_probe_max_seconds = probe_sorted[3]
        out.prod_domain_min_seconds = domain_sorted[1]
        out.prod_domain_median_seconds = domain_sorted[2]
        out.prod_domain_max_seconds = domain_sorted[3]
        out.production_domain_wins = sum(
            production_candidate_timing[.,2] :<
            production_candidate_timing[.,1])
        out.timing_selects_domain =
            out.production_domain_wins >= 2 &
            out.prod_domain_median_seconds <
                out.prod_probe_median_seconds
    }

    shape = vckss_rng__compare_call_shapes(
        8675309,30001,(1 \ 2 \ 7 \ 19 \ 100))
    if (shape.status == "OK") {
        out.small_callshape_atoms_equal = shape.atoms_equal
        out.small_callshape_states_equal = shape.states_equal
    }
    boundary_shape = vckss_rng__compare_call_shapes(
        8675309,30002,(vckss_rng__max_binomial_trials()))
    if (boundary_shape.status == "OK") {
        out.boundary_callshape_atoms_equal = boundary_shape.atoms_equal
        out.boundary_callshape_states_equal = boundary_shape.states_equal
        out.boundary_atom = boundary_shape.vector_atoms[1]
    }
    production_trials = mod((1::50000),97):+1
    saved = vckss_rng__capture()
    if (vckss_rng__set_stream_seed(30001,8675309) == 0) {
        timer_clear(81)
        timer_on(81)
        scalar_draw = vckss_rng__draw_probe_scalar(production_trials)
        timer_off(81)
        scalar_state = rngstate()
        out.production_scalar_seconds = timer_value(81)[1]
        if (vckss_rng__set_stream_seed(30001,8675309) == 0) {
            timer_clear(82)
            timer_on(82)
            production_trials = vckss_rng__draw_probe_vector(
                production_trials)
            timer_off(82)
            vector_state = rngstate()
            out.production_vector_seconds = timer_value(82)[1]
            out.production_callshape_atoms_equal =
                all(scalar_draw[.,1] :== production_trials)
            out.prod_callshape_states_equal =
                scalar_state == vector_state
        }
    }
    cleanup_rc = vckss_rng__restore(saved)
    out.production_callshape_atoms_equal =
        out.production_callshape_atoms_equal & cleanup_rc == 0
    out.prod_callshape_states_equal =
        out.prod_callshape_states_equal & cleanup_rc == 0

    out.exact_integer_gate_pass =
        vckss_rng__trials_ok((2^53-1)) == 1 &
        vckss_rng__trials_ok((2^53-1 \ 1)) == 0 &
        rows(vckss_rng__draw_probe_vector(
            (vckss_rng__max_binomial_trials()+1))) == 0
    saved = vckss_rng__capture()
    if (vckss_rng__set_stream_seed(30002,8675309) == 0) {
        chunked = vckss_rng__binomial_atom_scalar(
            vckss_rng__max_binomial_trials()+7)
        chunk_state = rngstate()
        if (vckss_rng__set_stream_seed(30002,8675309) == 0) {
            manual_successes =
                rbinomial(1,1,vckss_rng__max_binomial_trials(),0.5)+
                rbinomial(1,1,7,0.5)
            manual_atom = 2*manual_successes-
                (vckss_rng__max_binomial_trials()+7)
            manual_state = rngstate()
            out.chunked_atom = chunked[1]
            out.chunked_calls = chunked[2]
            out.boundary_chunk_pass = chunked[2] == 2 &
                chunked[1] == manual_atom & chunk_state == manual_state
        }
    }
    cleanup_rc = vckss_rng__restore(saved)
    out.boundary_chunk_pass = out.boundary_chunk_pass & cleanup_rc == 0
    large = vckss_rng__generate(
        "per_domain_stream",8675309,"target",1,1,(1),
        (vckss_rng__max_binomial_trials()+7))
    if (large.status == "OK") {
        out.large_generation_pass = large.chunk_calls == 2 &
            abs(large.atoms[1,1]) <= vckss_rng__max_binomial_trials()+7 &
            mod(large.atoms[1,1]+
                vckss_rng__max_binomial_trials()+7,2) == 0
    }

    out.per_probe_candidate_qualified =
        out.golden_vectors_match & out.canonical_order_invariant &
        out.per_probe_partition_invariant &
        out.processor_per_probe_invariant &
        out.probe_hidden_streams_restored &
        out.ns_lev_streams_restored &
        out.ns_tgt_streams_restored &
        out.nonselected_caller_restored
    if (out.per_probe_candidate_qualified) {
        out.per_probe_candidate_result = "QUALIFIED_STATA19_CANDIDATE"
    }
    else if (!out.probe_hidden_streams_restored |
        !out.ns_lev_streams_restored |
        !out.ns_tgt_streams_restored |
        !out.nonselected_caller_restored) {
        out.per_probe_candidate_result =
            "REJECT_HIDDEN_MT64S_STREAM_STATE_MUTATION"
    }
    else out.per_probe_candidate_result =
        "REJECT_COMPATIBILITY_OR_INVARIANCE_FAILURE"
    out.core_pass = out.module_contract_ok & out.runtime_fail_closed &
        out.golden_vectors_match & out.canonical_order_invariant &
        out.per_probe_partition_invariant &
        out.per_domain_partition_invariant &
        out.cursor_partition_invariant &
        out.processor_per_probe_invariant &
        out.processor_per_domain_invariant & out.processor_switch_ok &
        out.small_callshape_atoms_equal &
        out.small_callshape_states_equal &
        out.boundary_callshape_atoms_equal &
        out.boundary_callshape_states_equal &
        out.production_callshape_atoms_equal &
        out.prod_callshape_states_equal &
        out.exact_integer_gate_pass & out.boundary_chunk_pass &
        out.large_generation_pass &
        out.per_probe_test_cleanup_restored &
        out.ns_lev_streams_restored &
        out.ns_tgt_streams_restored &
        out.nonselected_caller_restored &
        out.prod_timing_streams_restored &
        lev_timing.status == "OK" & tgt_timing.status == "OK" &
        out.production_candidate_atoms == 50000 &
        out.production_candidate_probes == 40 &
        out.production_candidate_repetitions >= 3 &
        !missing(out.production_per_probe_min_seconds) &
        !missing(out.prod_probe_median_seconds) &
        !missing(out.production_per_probe_max_seconds) &
        !missing(out.prod_domain_min_seconds) &
        !missing(out.prod_domain_median_seconds) &
        !missing(out.prod_domain_max_seconds) &
        out.production_per_probe_min_seconds <=
            out.prod_probe_median_seconds &
        out.prod_probe_median_seconds <=
            out.production_per_probe_max_seconds &
        out.prod_domain_min_seconds <=
            out.prod_domain_median_seconds &
        out.prod_domain_median_seconds <=
            out.prod_domain_max_seconds &
        out.timing_selects_domain
    out.status = out.core_pass ? "CORE_COMPLETE" : "CORE_FAILED"
    return(out)
}

string scalar rngk1__number(real scalar value)
{
    if (missing(value)) return(".")
    return(strtrim(strofreal(value,"%21.17g")))
}

void rngk1__write_snapshot(
    string scalar path,
    struct vckss_rng__full_snapshot scalar snapshot,
    real colvector streams,
    string colvector states)
{
    real scalar file, index
    string scalar tab

    file = fopen(path,"w")
    tab = char(9)
    fput(file,"status"+tab+snapshot.status)
    fput(file,"active_algorithm"+tab+snapshot.active.algorithm)
    fput(file,"active_stream"+tab+rngk1__number(snapshot.active.stream))
    fput(file,"active_state"+tab+snapshot.active.state)
    fput(file,"sort_state"+tab+snapshot.sort_state)
    fput(file,"domain_stream1_state"+tab+snapshot.mt64s_stream1_state)
    fput(file,"domain_stream2_state"+tab+snapshot.mt64s_stream2_state)
    fput(file,"selected_stream_state"+tab+
        snapshot.mt64s_selected_stream_state)
    for (index=1; index<=rows(streams); index++) {
        fput(file,"mt64s_stream_"+rngk1__number(streams[index])+tab+
            states[index])
    }
    fclose(file)
}

void rngk1__write_golden(
    string scalar path,
    struct rngk1_evidence scalar evidence)
{
    real scalar candidate, domain, file, row
    string rowvector candidates, domains
    string colvector keys
    real matrix values

    file = fopen(path,"w")
    fput(file,"candidate,domain,semantic_key,probe_1,probe_2,probe_3")
    candidates = ("per_probe_stream","per_domain_stream")
    domains = ("leverage","target")
    keys = ("a" \ "b" \ "c" \ "d")
    for (candidate=1; candidate<=2; candidate++) {
        for (domain=1; domain<=2; domain++) {
            if (candidate == 1 & domain == 1) {
                values = evidence.per_probe_leverage
            }
            else if (candidate == 1 & domain == 2) {
                values = evidence.per_probe_target
            }
            else if (candidate == 2 & domain == 1) {
                values = evidence.per_domain_leverage
            }
            else values = evidence.per_domain_target
            if (rows(values) == 4 & cols(values) == 3) {
                for (row=1; row<=4; row++) {
                    fput(file,candidates[candidate]+","+domains[domain]+","+
                        keys[row]+","+rngk1__number(values[row,1])+","+
                        rngk1__number(values[row,2])+","+
                        rngk1__number(values[row,3]))
                }
            }
        }
    }
    fclose(file)
}

void rngk1__put(real scalar file, string scalar key, string scalar value)
{
    fput(file,key+char(9)+value)
}

void rngk1__write_receipt(
    string scalar path,
    string scalar experiment_id,
    string scalar source_commit,
    string scalar bundle_sha,
    string scalar job_id,
    real scalar requested_slots,
    real scalar actual_slots,
    real scalar requested_processors,
    struct rngk1_evidence scalar evidence)
{
    real scalar file

    file = fopen(path,"w")
    fput(file,"key"+char(9)+"value")
    rngk1__put(file,"receipt_version","KSS-RNG-K1-STATA19-V1")
    rngk1__put(file,"experiment_id",experiment_id)
    rngk1__put(file,"source_commit",source_commit)
    rngk1__put(file,"bundle_sha256",bundle_sha)
    rngk1__put(file,"job_id",job_id)
    rngk1__put(file,"stata_version",
        strtrim(strofreal(st_numscalar("c(stata_version)"),"%9.0g")))
    rngk1__put(file,"stata_flavor",st_global("c(flavor)"))
    rngk1__put(file,"stata_mp",rngk1__number(st_numscalar("c(MP)")))
    rngk1__put(file,"requested_slots",rngk1__number(requested_slots))
    rngk1__put(file,"actual_slots",rngk1__number(actual_slots))
    rngk1__put(file,"requested_stata_processors",
        rngk1__number(requested_processors))
    rngk1__put(file,"actual_stata_processors",
        rngk1__number(st_numscalar("c(processors)")))
    rngk1__put(file,"rng_api_level",
        rngk1__number(vckss_rng__api_level()))
    rngk1__put(file,"rng_build_id",vckss_rng__build_id())
    rngk1__put(file,"rng_invariant_version",
        vckss_rng__invariant_version())
    rngk1__put(file,"stata18_reference_contract",
        "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18")
    rngk1__put(file,"production_contract_at_runtime",
        vckss_rng__production_contract())
    rngk1__put(file,"production_runtime_fail_closed",
        rngk1__number(evidence.runtime_fail_closed))
    rngk1__put(file,"per_probe_leverage_status",
        evidence.per_probe_leverage_status)
    rngk1__put(file,"per_probe_target_status",
        evidence.per_probe_target_status)
    rngk1__put(file,"per_domain_leverage_status",
        evidence.per_domain_leverage_status)
    rngk1__put(file,"per_domain_target_status",
        evidence.per_domain_target_status)
    rngk1__put(file,"per_probe_contract",evidence.per_probe_contract)
    rngk1__put(file,"per_domain_contract",evidence.per_domain_contract)
    rngk1__put(file,"golden_vectors_match",
        rngk1__number(evidence.golden_vectors_match))
    rngk1__put(file,"canonical_order_invariant",
        rngk1__number(evidence.canonical_order_invariant))
    rngk1__put(file,"per_probe_partition_invariant",
        rngk1__number(evidence.per_probe_partition_invariant))
    rngk1__put(file,"per_domain_partition_invariant",
        rngk1__number(evidence.per_domain_partition_invariant))
    rngk1__put(file,"cursor_partition_invariant",
        rngk1__number(evidence.cursor_partition_invariant))
    rngk1__put(file,"processor_per_probe_invariant",
        rngk1__number(evidence.processor_per_probe_invariant))
    rngk1__put(file,"processor_per_domain_invariant",
        rngk1__number(evidence.processor_per_domain_invariant))
    rngk1__put(file,"processor_switch_ok",
        rngk1__number(evidence.processor_switch_ok))
    rngk1__put(file,"small_callshape_atoms_equal",
        rngk1__number(evidence.small_callshape_atoms_equal))
    rngk1__put(file,"small_callshape_states_equal",
        rngk1__number(evidence.small_callshape_states_equal))
    rngk1__put(file,"boundary_callshape_atoms_equal",
        rngk1__number(evidence.boundary_callshape_atoms_equal))
    rngk1__put(file,"boundary_callshape_states_equal",
        rngk1__number(evidence.boundary_callshape_states_equal))
    rngk1__put(file,"production_callshape_atoms_equal",
        rngk1__number(evidence.production_callshape_atoms_equal))
    rngk1__put(file,"production_callshape_states_equal",
        rngk1__number(evidence.prod_callshape_states_equal))
    rngk1__put(file,"exact_integer_gate_pass",
        rngk1__number(evidence.exact_integer_gate_pass))
    rngk1__put(file,"boundary_chunk_pass",
        rngk1__number(evidence.boundary_chunk_pass))
    rngk1__put(file,"large_generation_pass",
        rngk1__number(evidence.large_generation_pass))
    rngk1__put(file,"boundary_atom",rngk1__number(evidence.boundary_atom))
    rngk1__put(file,"chunked_atom",rngk1__number(evidence.chunked_atom))
    rngk1__put(file,"chunked_calls",rngk1__number(evidence.chunked_calls))
    rngk1__put(file,"production_scalar_seconds",
        rngk1__number(evidence.production_scalar_seconds))
    rngk1__put(file,"production_vector_seconds",
        rngk1__number(evidence.production_vector_seconds))
    rngk1__put(file,"leverage_timing_status",
        evidence.leverage_timing_status)
    rngk1__put(file,"target_timing_status",evidence.target_timing_status)
    rngk1__put(file,"leverage_recommendation",
        evidence.leverage_recommendation)
    rngk1__put(file,"target_recommendation",
        evidence.target_recommendation)
    rngk1__put(file,"leverage_per_probe_seconds",
        rngk1__number(evidence.leverage_per_probe_seconds))
    rngk1__put(file,"leverage_per_domain_seconds",
        rngk1__number(evidence.leverage_per_domain_seconds))
    rngk1__put(file,"target_per_probe_seconds",
        rngk1__number(evidence.target_per_probe_seconds))
    rngk1__put(file,"target_per_domain_seconds",
        rngk1__number(evidence.target_per_domain_seconds))
    rngk1__put(file,"tiny_timing_role","SMOKE_ONLY_NOT_SELECTION")
    rngk1__put(file,"selection_timing_contract",
        "50000_ATOMS_P40_BOTH_DOMAINS_3_PAIRED_REPS_V1")
    rngk1__put(file,"production_candidate_atoms",
        rngk1__number(evidence.production_candidate_atoms))
    rngk1__put(file,"production_candidate_probes",
        rngk1__number(evidence.production_candidate_probes))
    rngk1__put(file,"production_candidate_repetitions",
        rngk1__number(evidence.production_candidate_repetitions))
    rngk1__put(file,"production_per_probe_min_seconds",
        rngk1__number(evidence.production_per_probe_min_seconds))
    rngk1__put(file,"production_per_probe_median_seconds",
        rngk1__number(evidence.prod_probe_median_seconds))
    rngk1__put(file,"production_per_probe_max_seconds",
        rngk1__number(evidence.production_per_probe_max_seconds))
    rngk1__put(file,"production_per_domain_min_seconds",
        rngk1__number(evidence.prod_domain_min_seconds))
    rngk1__put(file,"production_per_domain_median_seconds",
        rngk1__number(evidence.prod_domain_median_seconds))
    rngk1__put(file,"production_per_domain_max_seconds",
        rngk1__number(evidence.prod_domain_max_seconds))
    rngk1__put(file,"production_domain_wins",
        rngk1__number(evidence.production_domain_wins))
    rngk1__put(file,"production_timing_streams_restored",
        rngk1__number(evidence.prod_timing_streams_restored))
    rngk1__put(file,"timing_includes_stream_snapshot_restore","1")
    rngk1__put(file,"timing_selects_domain",
        rngk1__number(evidence.timing_selects_domain))
    rngk1__put(file,"per_probe_streams_tested",
        "1,2,3,16384,16385,16386")
    rngk1__put(file,"per_probe_hidden_streams_restored",
        rngk1__number(evidence.probe_hidden_streams_restored))
    rngk1__put(file,"per_probe_changed_stream_count",
        rngk1__number(evidence.per_probe_changed_stream_count))
    rngk1__put(file,"per_probe_test_cleanup_restored",
        rngk1__number(evidence.per_probe_test_cleanup_restored))
    rngk1__put(file,"nonselected_probe_range","7:8")
    rngk1__put(file,"nonselected_selected_stream","9")
    rngk1__put(file,"nonselected_leverage_streams","7,8")
    rngk1__put(file,"nonselected_target_streams","16390,16391")
    rngk1__put(file,"nonselected_leverage_streams_restored",
        rngk1__number(evidence.ns_lev_streams_restored))
    rngk1__put(file,"nonselected_target_streams_restored",
        rngk1__number(evidence.ns_tgt_streams_restored))
    rngk1__put(file,"nonselected_caller_restored",
        rngk1__number(evidence.nonselected_caller_restored))
    rngk1__put(file,"per_probe_candidate_qualified",
        rngk1__number(evidence.per_probe_candidate_qualified))
    rngk1__put(file,"per_probe_candidate_result",
        evidence.per_probe_candidate_result)
    rngk1__put(file,"per_domain_streams_tested","1,2")
    rngk1__put(file,"per_domain_candidate_qualified",
        rngk1__number(evidence.per_domain_candidate_qualified))
    rngk1__put(file,"per_domain_candidate_result",
        evidence.per_domain_candidate_result)
    rngk1__put(file,"selected_candidate","per_domain_stream_cursor")
    rngk1__put(file,"active_algorithm_restored",
        rngk1__number(evidence.active_algorithm_restored))
    rngk1__put(file,"active_stream_restored",
        rngk1__number(evidence.active_stream_restored))
    rngk1__put(file,"active_state_restored",
        rngk1__number(evidence.active_state_restored))
    rngk1__put(file,"sort_state_restored",
        rngk1__number(evidence.sort_state_restored))
    rngk1__put(file,"domain_stream1_restored",
        rngk1__number(evidence.domain_stream1_restored))
    rngk1__put(file,"domain_stream2_restored",
        rngk1__number(evidence.domain_stream2_restored))
    rngk1__put(file,"selected_stream_state_restored",
        rngk1__number(evidence.selected_stream_state_restored))
    rngk1__put(file,"domain_guard_full_restored",
        rngk1__number(evidence.domain_guard_full_restored))
    rngk1__put(file,"every_touched_stream_restored",
        rngk1__number(evidence.every_touched_stream_restored))
    rngk1__put(file,"core_rc",rngk1__number(evidence.core_rc))
    rngk1__put(file,"processor_restore_rc",
        rngk1__number(evidence.processor_restore_rc))
    rngk1__put(file,"guard_restore_rc",
        rngk1__number(evidence.guard_restore_rc))
    rngk1__put(file,"all_stream_restore_rc",
        rngk1__number(evidence.all_stream_restore_rc))
    rngk1__put(file,"overall_status",evidence.status)
    fclose(file)
}

RNGK1_EVIDENCE = rngk1__empty()
RNGK1_ALL_STREAMS = uniqrows(sort(
    ((1::40) \ 177 \ (16384::16423) \ 30001 \ 30002),1))
RNGK1_BEFORE = vckss_rng__capture_full()
RNGK1_STREAMS_BEFORE = rngk1__capture_streams(RNGK1_ALL_STREAMS)
st_local("rngk1_guard_begin_rc",strofreal(vckss_rng__guard_begin()))
end

local rngk1_core_rc = 498
if real("`rngk1_guard_begin_rc'") == 0 {
    capture noisily mata: RNGK1_EVIDENCE = rngk1__core()
    local rngk1_core_rc = _rc
}

capture quietly set processors 4
local rngk1_processor_restore_rc = _rc
mata: st_local("rngk1_guard_restore_rc", strofreal(vckss_rng__guard_restore()))

mata:
RNGK1_AFTER_GUARD = vckss_rng__capture_full()
st_local("rngk1_all_stream_restore_rc",strofreal(rngk1__restore_streams(
    RNGK1_ALL_STREAMS,RNGK1_STREAMS_BEFORE)))
RNGK1_AFTER = vckss_rng__capture_full()
RNGK1_STREAMS_AFTER = rngk1__capture_streams(RNGK1_ALL_STREAMS)

RNGK1_EVIDENCE.core_rc = strtoreal(st_local("rngk1_core_rc"))
RNGK1_EVIDENCE.processor_restore_rc =
    strtoreal(st_local("rngk1_processor_restore_rc"))
RNGK1_EVIDENCE.guard_restore_rc =
    strtoreal(st_local("rngk1_guard_restore_rc"))
RNGK1_EVIDENCE.all_stream_restore_rc =
    strtoreal(st_local("rngk1_all_stream_restore_rc"))
RNGK1_EVIDENCE.active_algorithm_restored =
    RNGK1_BEFORE.active.algorithm == RNGK1_AFTER.active.algorithm
RNGK1_EVIDENCE.active_stream_restored =
    RNGK1_BEFORE.active.stream == RNGK1_AFTER.active.stream
RNGK1_EVIDENCE.active_state_restored =
    RNGK1_BEFORE.active.state == RNGK1_AFTER.active.state
RNGK1_EVIDENCE.sort_state_restored =
    RNGK1_BEFORE.sort_state == RNGK1_AFTER.sort_state
RNGK1_EVIDENCE.domain_stream1_restored =
    RNGK1_BEFORE.mt64s_stream1_state == RNGK1_AFTER.mt64s_stream1_state
RNGK1_EVIDENCE.domain_stream2_restored =
    RNGK1_BEFORE.mt64s_stream2_state == RNGK1_AFTER.mt64s_stream2_state
RNGK1_EVIDENCE.selected_stream_state_restored =
    RNGK1_BEFORE.mt64s_selected_stream_state ==
        RNGK1_AFTER.mt64s_selected_stream_state
RNGK1_EVIDENCE.domain_guard_full_restored =
    rngk1__full_equal(RNGK1_BEFORE,RNGK1_AFTER_GUARD)
RNGK1_EVIDENCE.every_touched_stream_restored =
    rows(RNGK1_STREAMS_BEFORE) == rows(RNGK1_ALL_STREAMS) &
    rows(RNGK1_STREAMS_AFTER) == rows(RNGK1_ALL_STREAMS) &
    all(RNGK1_STREAMS_BEFORE :== RNGK1_STREAMS_AFTER)
RNGK1_EVIDENCE.per_domain_candidate_qualified =
    RNGK1_EVIDENCE.core_pass &
    RNGK1_EVIDENCE.domain_guard_full_restored &
    RNGK1_EVIDENCE.guard_restore_rc == 0
RNGK1_EVIDENCE.per_domain_candidate_result =
    RNGK1_EVIDENCE.per_domain_candidate_qualified ?
    "QUALIFIED_STATA19_CANDIDATE_NOT_REGISTERED" :
    "REJECT_DOMAIN_GUARD_OR_COMPATIBILITY_FAILURE"
RNGK1_EVIDENCE.overall_pass =
    RNGK1_EVIDENCE.per_domain_candidate_qualified &
    RNGK1_EVIDENCE.core_rc == 0 &
    RNGK1_EVIDENCE.processor_restore_rc == 0 &
    RNGK1_EVIDENCE.all_stream_restore_rc == 0 &
    rngk1__full_equal(RNGK1_BEFORE,RNGK1_AFTER) &
    RNGK1_EVIDENCE.every_touched_stream_restored &
    st_numscalar("c(processors)") == 4
RNGK1_EVIDENCE.status = RNGK1_EVIDENCE.overall_pass ?
    "KSS_RNG_K1_STATA19_COMPATIBLE_FAIL_CLOSED" :
    "KSS_RNG_K1_STATA19_REJECTED"

rngk1__write_snapshot(st_local("output_dir")+"/caller_rng_before.tsv",
    RNGK1_BEFORE,RNGK1_ALL_STREAMS,RNGK1_STREAMS_BEFORE)
rngk1__write_snapshot(st_local("output_dir")+"/caller_rng_after.tsv",
    RNGK1_AFTER,RNGK1_ALL_STREAMS,RNGK1_STREAMS_AFTER)
rngk1__write_golden(st_local("output_dir")+"/golden_vectors.csv",
    RNGK1_EVIDENCE)
rngk1__write_receipt(st_local("output_dir")+"/stata_receipt.tsv",
    st_local("experiment_id"),st_local("source_commit"),
    st_local("bundle_sha"),st_local("job_id"),
    strtoreal(st_local("requested_slots_arg")),
    strtoreal(st_local("actual_slots_arg")),
    strtoreal(st_local("processors_arg")),RNGK1_EVIDENCE)
st_local("rngk1_final_status",RNGK1_EVIDENCE.status)
end

if "`rngk1_final_status'" != "KSS_RNG_K1_STATA19_COMPATIBLE_FAIL_CLOSED" {
    tempname failure
    file open `failure' using `"`output_dir'/stata.fail"', write text replace
    file write `failure' ///
        "KSS_RNG_K1_STATA_FAILURE `experiment_id' `bundle_sha' " ///
        "`source_commit' `job_id' `rngk1_final_status'" _n
    file close `failure'
    di as error "RNG-K1 Stata 19 compatibility gate failed"
    exit 459
}

tempname marker
file open `marker' using `"`output_dir'/stata.pass"', write text replace
file write `marker' ///
    "KSS_RNG_K1_STATA_PASS `experiment_id' `bundle_sha' " ///
    "`source_commit' `job_id'" _n
file close `marker'

di as result "KSS-RNG-K1 STATA19 COMPATIBILITY PASS: `experiment_id'"
exit 0
