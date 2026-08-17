version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "kss_bc/kss_bc_rng.mata"
if _rc {
    capture confirm file "../../kss_bc_rng.mata"
    if _rc exit 601
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/kss_bc"'

quietly do `"`pkgroot'/kss_bc_rng.mata"'

mata:
assert(kssbc_rng__api_level() == 2)
assert(kssbc_rng__build_id() == "kss-bc-rng-k1-mt64s-complete-guard-v2")
assert(kssbc_rng__invariant_version() == "KSS-RNG-K1-INVARIANT-V1")
assert(kssbc_rng__max_binomial_trials() == 100000000000)
assert(kssbc_rng__maximum_exact_integer() == 2^53-1)
assert(kssbc_rng__maximum_probes() == 16383)
assert(kssbc_rng__probe_stream("leverage",1) == 1)
assert(kssbc_rng__probe_stream("leverage",16383) == 16383)
assert(kssbc_rng__probe_stream("target",1) == 16384)
assert(kssbc_rng__probe_stream("target",16383) == 32766)
assert(kssbc_rng__domain_stream("leverage") == 1)
assert(kssbc_rng__domain_stream("target") == 2)
assert(kssbc_rng__k1_recommendation() == "per_domain_stream_cursor")
assert(kssbc_rng__production_contract() ==
    "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18")
assert(kssbc_rng__trials_ok((2^53-1)) == 1)
assert(kssbc_rng__trials_ok((2^53-1 \ 1)) == 0)
end

// Both candidates restore the caller's algorithm, stream selection, and
// complete current-generator state.
set rng kiss32
set seed 20260816
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
mata:
candidate_streams = (1 \ 2 \ 3 \ 16384 \ 16385 \ 16386)
candidate_streams_before = kssbc_rng__capture_streams(candidate_streams)
assert(candidate_streams_before.status == "OK")
keys = ("c" \ "a" \ "d" \ "b")
trials = (7 \ 1 \ 19 \ 2)
probe = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",1,3,keys,trials)
assert(probe.status == "OK")
assert(probe.chunk_calls == 3)
assert(probe.semantic_key ==
    ("a" \ "b" \ "c" \ "d"))
assert(probe.canonical_order == (2 \ 4 \ 1 \ 3))
assert(probe.stream_first == 1 & probe.stream_last == 3)
assert(all(abs(probe.atoms) :<= trials[probe.canonical_order]))
assert(all(mod(probe.atoms:+trials[probe.canonical_order],2) :== 0))
st_matrix("per_probe_atoms",probe.atoms)
end
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

mata:
domain = kssbc_rng__generate(
    "per_domain_stream",8675309,"leverage",1,3,keys,trials)
assert(domain.status == "OK")
assert(domain.chunk_calls == 3)
assert(domain.stream_first == 1 & domain.stream_last == 1)
st_matrix("per_domain_atoms",domain.atoms)
end
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Exercise the same exact chunk loop with a deliberately small evidence
// limit.  Calling the documented 1e11 boundary belongs in the timed K1
// benchmark rather than this quick correctness gate.
mata:
saved = kssbc_rng__capture_streams(J(1,1,30002))
assert(saved.status == "OK")
assert(kssbc_rng__set_stream_seed(30002,8675309) == 0)
chunked = kssbc_rng__binomial_atom_limit(17,10)
assert(chunked[2] == 2)
assert(abs(chunked[1]) <= 17)
assert(mod(chunked[1]+17,2) == 0)
assert(kssbc_rng__restore_streams(saved) == 0)
end
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Exercise Stata's documented maximum binomial trial count and the first
// exact large-count chunk above it.  The chunked atom and terminal state must
// match two explicit calls in canonical order; the boundary vector call must
// match the scalar route exactly.
mata:
boundary_shape = kssbc_rng__compare_call_shapes(
    8675309,30002,(kssbc_rng__max_binomial_trials()))
assert(boundary_shape.status == "OK")
assert(boundary_shape.atoms_equal == 1)
assert(boundary_shape.states_equal == 1)

boundary_saved = kssbc_rng__capture_streams(J(1,1,30002))
assert(boundary_saved.status == "OK")
assert(kssbc_rng__set_stream_seed(30002,8675309) == 0)
boundary_chunked = kssbc_rng__binomial_atom_scalar(
    kssbc_rng__max_binomial_trials()+7)
boundary_chunked_state = rngstate()
assert(boundary_chunked[2] == 2)

assert(kssbc_rng__set_stream_seed(30002,8675309) == 0)
boundary_successes =
    rbinomial(1,1,kssbc_rng__max_binomial_trials(),0.5)+
    rbinomial(1,1,7,0.5)
boundary_manual = 2*boundary_successes-
    (kssbc_rng__max_binomial_trials()+7)
boundary_manual_state = rngstate()
assert(boundary_chunked[1] == boundary_manual)
assert(boundary_chunked_state == boundary_manual_state)
assert(kssbc_rng__restore_streams(boundary_saved) == 0)
st_matrix("rng_large_count_evidence",
    (boundary_shape.scalar_atoms[1],boundary_chunked[1],
     boundary_chunked[2]))
end
matrix colnames rng_large_count_evidence = boundary_atom chunked_atom calls
matrix list rng_large_count_evidence
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Canonical semantic keys make raw row order and irrelevant dense encodings
// immaterial.  Generating a probe range in pieces produces the same atoms.
mata:
permutation = (3 \ 1 \ 4 \ 2)
shuffled = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",1,3,
    keys[permutation],trials[permutation])
assert(shuffled.status == "OK")
assert(shuffled.semantic_key == probe.semantic_key)
assert(shuffled.atoms == probe.atoms)

first = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",1,1,keys,trials)
second = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",2,2,keys,trials)
assert((first.atoms,second.atoms) == probe.atoms)

first = kssbc_rng__generate(
    "per_domain_stream",8675309,"leverage",1,1,keys,trials)
second = kssbc_rng__generate(
    "per_domain_stream",8675309,"leverage",2,2,keys,trials)
assert((first.atoms,second.atoms) == domain.atoms)

target_probe = kssbc_rng__generate(
    "per_probe_stream",8675309,"target",1,3,keys,trials)
target_domain = kssbc_rng__generate(
    "per_domain_stream",8675309,"target",1,3,keys,trials)
assert(target_probe.status == "OK" & target_domain.status == "OK")
assert(any(target_probe.atoms :!= probe.atoms))
assert(any(target_domain.atoms :!= domain.atoms))
candidate_streams_after = kssbc_rng__capture_streams(candidate_streams)
assert(candidate_streams_after.status == "OK")
assert(candidate_streams_after.state == candidate_streams_before.state)

// Random atoms are invariant to the licensed processor count.  This is an
// exact RNG contract; downstream floating-point estimator reductions retain
// their separate numerical-tolerance contract.
end
local caller_processors = c(processors)
local comparison_processors = min(4,`caller_processors')
if `comparison_processors' > 1 {
    quietly set processors 1
    mata: processor_one = kssbc_rng__generate("per_domain_stream",8675309,"target",1,3,keys,trials)
    mata: assert(processor_one.status == "OK")
    quietly set processors `comparison_processors'
    mata: processor_many = kssbc_rng__generate("per_domain_stream",8675309,"target",1,3,keys,trials)
    mata: assert(processor_many.status == "OK")
    mata: assert(processor_many.atoms == processor_one.atoms)
    mata: assert(processor_many.semantic_key == processor_one.semantic_key)
    quietly set processors `caller_processors'
}
mata:

expected_probe = (1,-1,1 \ -2,2,2 \ -3,3,3 \ -1,1,7)
expected_domain = (1,1,1 \ -2,0,0 \ -3,-3,-1 \ -1,3,-3)
expected_target_probe = (-1,1,1 \ 0,0,0 \ 5,3,-3 \ 7,7,-3)
expected_target_domain = (-1,1,-1 \ 2,0,0 \ 3,1,1 \ 1,3,-1)
assert(probe.atoms == expected_probe)
assert(domain.atoms == expected_domain)
assert(target_probe.atoms == expected_target_probe)
assert(target_domain.atoms == expected_target_domain)

// The fixed-domain candidate has a stateful batch API; it never replays
// earlier probes when the solver requests a new batch.
cursor_stream_before = kssbc_rng__capture_streams((1 \ 2))
assert(cursor_stream_before.status == "OK")
cursor_all = kssbc_rng__open_cursor(8675309,"leverage",keys,trials)
cursor_split = kssbc_rng__open_cursor(8675309,"leverage",keys,trials)
assert(cursor_all.status == "OK" & cursor_split.status == "OK")
cursor_all_batch = kssbc_rng__cursor_next(&cursor_all,3)
cursor_first = kssbc_rng__cursor_next(&cursor_split,1)
cursor_second = kssbc_rng__cursor_next(&cursor_split,2)
assert(cursor_all_batch.status == "OK")
assert((cursor_first.atoms,cursor_second.atoms) == cursor_all_batch.atoms)
assert(cursor_all_batch.atoms == domain.atoms)
assert(cursor_split.next_probe == 4)
cursor_stream_after = kssbc_rng__capture_streams((1 \ 2))
assert(cursor_stream_after.status == "OK")
assert(cursor_stream_after.state == cursor_stream_before.state)
end
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// The module registers whether scalar and vector parameter calls consume the
// same mt64s sequence.  Version 2 uses the vector-parameter call whenever
// every trial count is within Stata's documented per-call limit.
mata:
shape_stream_before = kssbc_rng__capture_streams(J(1,1,30001))
assert(shape_stream_before.status == "OK")
shape = kssbc_rng__compare_call_shapes(
    8675309,30001,(1 \ 2 \ 7 \ 19 \ 100))
assert(shape.status == "OK")
assert(shape.atoms_equal == 1)
assert(shape.states_equal == 1)
shape_stream_after = kssbc_rng__capture_streams(J(1,1,30001))
assert(shape_stream_after.status == "OK")
assert(shape_stream_after.state == shape_stream_before.state)
end
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Production-shaped K1 evidence: the vector and scalar call shapes produce
// identical atoms and terminal mt64s states for 50,000 semantic atoms.  The
// paired timings must favor the vector route in at least two of three runs
// and in the median; this guards against selecting it from a tiny fixture.
mata:
callshape_trials = mod((1::50000),97):+1
callshape_timing = J(3,2,.)
callshape_saved = kssbc_rng__capture_streams(J(1,1,30001))
assert(callshape_saved.status == "OK")
for (callshape_rep=1; callshape_rep<=3; callshape_rep++) {
    assert(kssbc_rng__set_stream_seed(30001,8675309) == 0)
    timer_clear(83)
    timer_on(83)
    callshape_scalar = kssbc_rng__draw_probe_scalar(callshape_trials)
    timer_off(83)
    callshape_scalar_state = rngstate()
    callshape_timing[callshape_rep,1] = timer_value(83)[1]

    assert(kssbc_rng__set_stream_seed(30001,8675309) == 0)
    timer_clear(84)
    timer_on(84)
    callshape_vector = kssbc_rng__draw_probe_vector(callshape_trials)
    timer_off(84)
    callshape_vector_state = rngstate()
    callshape_timing[callshape_rep,2] = timer_value(84)[1]
    assert(callshape_scalar[.,1] == callshape_vector)
    assert(callshape_scalar_state == callshape_vector_state)
}
assert(kssbc_rng__restore_streams(callshape_saved) == 0)
assert(sum(callshape_timing[.,2] :< callshape_timing[.,1]) >= 2)
callshape_scalar_sorted = sort(callshape_timing[.,1],1)
callshape_vector_sorted = sort(callshape_timing[.,2],1)
assert(callshape_vector_sorted[2] < callshape_scalar_sorted[2])
st_matrix("rng_callshape_timing",callshape_timing)
end
matrix colnames rng_callshape_timing = scalar vector
matrix list rng_callshape_timing
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Paired, post-warmup timings produce a recommendation without hardwiring
// either candidate into the estimator.
mata:
timing = kssbc_rng__benchmark(8675309,"leverage",3,keys,trials,3)
target_timing = kssbc_rng__benchmark(8675309,"target",3,keys,trials,3)
assert(timing.status == "OK" & target_timing.status == "OK")
assert(timing.per_probe_seconds >= 0 & timing.per_domain_seconds >= 0)
assert(target_timing.per_probe_seconds >= 0 &
    target_timing.per_domain_seconds >= 0)
combined_probe = timing.per_probe_seconds+target_timing.per_probe_seconds
combined_domain = timing.per_domain_seconds+target_timing.per_domain_seconds
assert(combined_probe+combined_domain > 0)
assert(combined_domain < combined_probe)
st_matrix("rng_candidate_timing",
    (timing.per_probe_seconds,timing.per_domain_seconds,
     target_timing.per_probe_seconds,target_timing.per_domain_seconds,
     combined_probe/combined_domain))
st_local("rng_recommendation","per_domain_stream_cursor")
end
matrix colnames rng_candidate_timing = lev_probe lev_domain tgt_probe tgt_domain ratio
matrix list rng_candidate_timing
display as text "K1 local timing recommendation: `rng_recommendation'"
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Invalid contracts fail before touching the caller RNG.
mata:
bad = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",1,2,
    ("duplicate" \ "duplicate"),(1 \ 1))
assert(bad.status == "RNG_SEMANTIC_KEY_INVALID")
too_large = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",1,2,
    ("a" \ "b"),(2^53 \ 1))
assert(too_large.status == "BINOMIAL_CONTRACT_UNSUPPORTED")
bad_range = kssbc_rng__generate(
    "per_probe_stream",8675309,"leverage",16383,2,
    ("a" \ "b"),(1 \ 1))
assert(bad_range.status == "RNG_PROBE_RANGE_INVALID")
end
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

quietly cd `"`oldpwd'"'
di as result "PASS test_scale_rng.do"
exit 0
