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
assert(kssbc_rng__api_level() == 3)
assert(kssbc_rng__build_id() ==
    "kss-bc-rng-runtime-scoped-domain-cursor-v3")
assert(kssbc_rng__invariant_version() == "KSS-RNG-K1-INVARIANT-V1")
assert(kssbc_rng__k1_recommendation() == "per_domain_stream_cursor")
assert(kssbc_rng__max_binomial_trials() == 100000000000)
assert(kssbc_rng__maximum_exact_integer() == 2^53-1)
assert(kssbc_rng__domain_stream("leverage") == 1)
assert(kssbc_rng__domain_stream("target") == 2)
runtime = st_numscalar("c(stata_version)")
if (runtime >= 18 & runtime < 19) {
    assert(kssbc_rng__production_contract() ==
        "KSS-MT64S-DOMAIN-CURSOR-V3-STATA18")
}
else if (runtime >= 19 & runtime < 20) {
    assert(kssbc_rng__production_contract() ==
        "KSS-MT64S-DOMAIN-CURSOR-V3-STATA19")
}
else assert(kssbc_rng__production_contract() == "")
assert(kssbc_rng__trials_ok((2^53-1)) == 1)
assert(kssbc_rng__trials_ok((2^53-1 \ 1)) == 0)
end

// The one-time K1 timing comparison is frozen evidence. This ordinary
// regression exercises only the selected stateful domain cursor.
set rng kiss32
set seed 20260816
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'

mata:
keys = ("c" \ "a" \ "d" \ "b")
trials = (7 \ 1 \ 19 \ 2)
permutation = (3 \ 1 \ 4 \ 2)
streams_before = kssbc_rng__capture_streams((1 \ 2))
assert(streams_before.status == "OK")

cursor_all = kssbc_rng__open_cursor(8675309,"leverage",keys,trials)
cursor_split = kssbc_rng__open_cursor(8675309,"leverage",keys,trials)
cursor_shuffled = kssbc_rng__open_cursor(
    8675309,"leverage",keys[permutation],trials[permutation])
assert(cursor_all.status == "OK" & cursor_split.status == "OK")
assert(cursor_shuffled.status == "OK")
all_atoms = kssbc_rng__cursor_next(&cursor_all,3)
first_atoms = kssbc_rng__cursor_next(&cursor_split,1)
later_atoms = kssbc_rng__cursor_next(&cursor_split,2)
shuffled_atoms = kssbc_rng__cursor_next(&cursor_shuffled,3)
assert(all_atoms.status == "OK")
assert((first_atoms.atoms,later_atoms.atoms) == all_atoms.atoms)
assert(shuffled_atoms.atoms == all_atoms.atoms)
assert(all_atoms.semantic_key == ("a" \ "b" \ "c" \ "d"))
assert(all_atoms.canonical_order == (2 \ 4 \ 1 \ 3))
assert(all_atoms.stream_first == 1 & all_atoms.stream_last == 1)
assert(cursor_split.next_probe == 4)

target_cursor = kssbc_rng__open_cursor(8675309,"target",keys,trials)
target_atoms = kssbc_rng__cursor_next(&target_cursor,3)
assert(target_atoms.status == "OK")
assert(target_atoms.stream_first == 2 & target_atoms.stream_last == 2)
assert(any(target_atoms.atoms :!= all_atoms.atoms))

expected_leverage = (1,1,1 \ -2,0,0 \ -3,-3,-1 \ -1,3,-3)
expected_target = (-1,1,-1 \ 2,0,0 \ 3,1,1 \ 1,3,-1)
assert(all_atoms.atoms == expected_leverage)
assert(target_atoms.atoms == expected_target)

// Crossing the old per-probe registry boundary is valid for the production
// cursor. Set the logical index directly so this range regression costs one
// draw rather than replaying 16,383 historical probes.
range_cursor = kssbc_rng__open_cursor(8675309,"leverage",keys,trials)
range_cursor.next_probe = 16384
range_atom = kssbc_rng__cursor_next(&range_cursor,1)
assert(range_atom.status == "OK")
assert(range_atom.probe_start == 16384)

streams_after = kssbc_rng__capture_streams((1 \ 2))
assert(streams_after.status == "OK")
assert(streams_after.state == streams_before.state)

shape_before = kssbc_rng__capture_streams(J(1,1,30001))
shape = kssbc_rng__compare_call_shapes(
    8675309,30001,(1 \ 2 \ 7 \ 19 \ 100))
shape_after = kssbc_rng__capture_streams(J(1,1,30001))
assert(shape.status == "OK")
assert(shape.atoms_equal == 1 & shape.states_equal == 1)
assert(shape_after.state == shape_before.state)

chunk_before = kssbc_rng__capture_streams(J(1,1,30002))
assert(kssbc_rng__set_stream_seed(30002,8675309) == 0)
chunked = kssbc_rng__binomial_atom_limit(17,10)
assert(chunked[2] == 2)
assert(abs(chunked[1]) <= 17 & mod(chunked[1]+17,2) == 0)
assert(kssbc_rng__restore_streams(chunk_before) == 0)

bad = kssbc_rng__open_cursor(
    8675309,"leverage",("duplicate" \ "duplicate"),(1 \ 1))
assert(bad.status == "RNG_SEMANTIC_KEY_INVALID")
too_large = kssbc_rng__open_cursor(
    8675309,"leverage",("a" \ "b"),(2^53 \ 1))
assert(too_large.status == "RNG_CURSOR_INVALID")
end

assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

quietly cd `"`oldpwd'"'
di as result "PASS test_scale_rng.do"
exit 0
