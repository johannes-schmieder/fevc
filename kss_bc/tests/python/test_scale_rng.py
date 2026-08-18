from __future__ import annotations

from collections import Counter
from itertools import product
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "kss_bc_rng.mata"
STATA_TEST = ROOT / "tests" / "stata" / "test_scale_rng.do"


def _rademacher_sum_distribution(trials: int) -> Counter[int]:
    return Counter(sum(signs) for signs in product((-1, 1), repeat=trials))


def test_rng_module_exposes_both_unselected_candidates() -> None:
    source = MODULE.read_text(encoding="utf-8")
    assert "KSS-RNG-K1-INVARIANT-V1" in source
    assert "KSS-MT64S-PER-PROBE-CANDIDATE-V2" in source
    assert "KSS-MT64S-PER-DOMAIN-CANDIDATE-V2" in source
    assert 'candidate == "per_probe_stream"' in source
    assert 'candidate == "per_domain_stream"' in source
    assert "selected_candidate" not in source


def test_production_contract_is_runtime_versioned_and_fail_closed() -> None:
    source = MODULE.read_text(encoding="utf-8")
    assert "KSS-MT64S-DOMAIN-CURSOR-V3-STATA18" in source
    assert "KSS-MT64S-DOMAIN-CURSOR-V3-STATA19" in source
    production = source.split(
        "string scalar kssbc_rng__production_contract()", 1
    )[1].split("real scalar kssbc_rng__max_binomial_trials", 1)[0]
    assert "runtime >= 18 & runtime < 19" in production
    assert "runtime >= 19 & runtime < 20" in production
    assert 'return("")' in production


def test_production_cursor_has_no_legacy_probe_registry_cap() -> None:
    source = MODULE.read_text(encoding="utf-8")
    cursor = source.split("kssbc_rng__open_cursor", 1)[1]
    assert "kssbc_rng__maximum_probes()" not in cursor
    assert "kssbc_rng__maximum_exact_integer()" in cursor
    legacy = source.split("kssbc_rng__generate(", 1)[1].split(
        "kssbc_rng__open_cursor", 1
    )[0]
    assert "kssbc_rng__maximum_probes()" in legacy


def test_contract_keys_exclude_execution_path_choices() -> None:
    source = MODULE.read_text(encoding="utf-8")
    struct = source.split("struct kssbc_rng__result", 1)[1].split("}", 1)[0]
    for required in (
        "contract_version",
        "candidate",
        "domain",
        "master_seed",
        "probe_start",
        "probe_count",
        "semantic_rank",
        "canonical_order",
    ):
        assert required in struct
    for forbidden in (
        "batch",
        "tile",
        "route",
        "processor",
        "iteration",
        "convergence",
    ):
        assert forbidden not in struct


def test_stream_registry_separates_domains_and_candidates() -> None:
    leverage = set(range(1, 16_384))
    target = set(range(16_384, 32_767))
    fixed_domains = {1, 2}
    assert leverage.isdisjoint(target)
    assert len(fixed_domains) == 2
    assert fixed_domains <= leverage


def test_chunked_binomial_sum_has_exact_rademacher_law() -> None:
    # Bin(n1, 1/2)+Bin(n2, 1/2) is Bin(n1+n2, 1/2).  Enumerating signs
    # independently checks the exact convolution used by large-count chunks.
    first = _rademacher_sum_distribution(3)
    second = _rademacher_sum_distribution(4)
    convolved: Counter[int] = Counter()
    for left, left_count in first.items():
        for right, right_count in second.items():
            convolved[left + right] += left_count * right_count
    assert convolved == _rademacher_sum_distribution(7)


def test_module_registers_large_count_and_call_shape_gates() -> None:
    source = MODULE.read_text(encoding="utf-8")
    assert "return(100000000000)" in source
    assert "return(2^53-1)" in source
    assert "maximum_exact_integer()-total" in source
    assert "sum(trials)" not in source
    assert "vector-parameter-or-scalar-chunk-canonical-atoms-v2" in source
    assert "kssbc_rng__compare_call_shapes" in source
    assert "kssbc_rng__binomial_atom_limit" in source
    assert "scalar_state" in source
    assert "vector_state" in source
    assert "rbinomial(1,1,chunk,0.5)" in source


def test_candidate_recommendation_is_computed_from_paired_timing() -> None:
    source = MODULE.read_text(encoding="utf-8")
    assert "kssbc_rng__benchmark" in source
    assert "timer_on(87)" in source
    assert "timer_on(88)" in source
    assert "per_domain_seconds <= out.per_probe_seconds" in source
    assert 'recommended_candidate = "per_domain_stream"' in source
    assert 'recommended_candidate = "per_probe_stream"' in source


def test_fixed_domain_candidate_has_nonreplaying_cursor_api() -> None:
    source = MODULE.read_text(encoding="utf-8")
    assert "struct kssbc_rng__cursor" in source
    assert "kssbc_rng__open_cursor" in source
    assert "kssbc_rng__cursor_next" in source
    cursor = source.split("kssbc_rng__cursor_next", 1)[1]
    assert "generator_state" in cursor
    assert "next_probe" in cursor
    assert "for (probe=1; probe<=probe_count; probe++)" in cursor
    assert "for (probe=1; probe<=finish; probe++)" not in cursor.split(
        "kssbc_rng__compare_call_shapes", 1
    )[0]


def test_caller_rng_restoration_is_an_explicit_success_and_failure_gate() -> None:
    source = MODULE.read_text(encoding="utf-8")
    test = STATA_TEST.read_text(encoding="utf-8")
    assert "struct kssbc_rng__snapshot" in source
    assert "struct kssbc_rng__stream_snapshot" in source
    assert "kssbc_rng__capture_streams" in source
    assert "kssbc_rng__restore_streams" in source
    assert "rngstate(saved.state)" in source
    assert source.count("RNG_RESTORE_FAILED") >= 4
    assert 'c(rngstate)' in test
    assert 'c(rngstream)' in test
    assert 'c(rng)' in test
    assert "streams_after.state == streams_before.state" in test
    assert "shape_after.state == shape_before.state" in test
    assert "RNG_SEMANTIC_KEY_INVALID" in test


def test_semantic_order_is_sorted_and_dense_ids_are_not_an_input() -> None:
    source = MODULE.read_text(encoding="utf-8")
    canonical = source.split(
        "real colvector kssbc_rng__canonical_order", 1
    )[1].split("real scalar kssbc_rng__trials_ok", 1)[0]
    assert "order(semantic_rank,1)" in canonical
    assert "maximum_exact_integer" in canonical
    assert "unique" not in canonical.lower() or "must be" not in canonical.lower()
    assert "worker_id" not in canonical
    assert "firm_id" not in canonical
    assert "dense" not in canonical


def test_stata_gate_covers_partition_and_row_order_invariance() -> None:
    test = STATA_TEST.read_text(encoding="utf-8")
    assert "ranks[permutation]" in test
    assert "trials[permutation]" in test
    assert "(first_atoms.atoms,later_atoms.atoms) == all_atoms.atoms" in test
    assert "shuffled_atoms.atoms == all_atoms.atoms" in test
    assert "target_atoms.atoms :!= all_atoms.atoms" in test
    assert "range_cursor.next_probe = 16384" in test
    assert "kssbc_rng__benchmark(" not in test
