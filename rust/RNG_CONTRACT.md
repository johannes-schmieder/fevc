# VCKSS-COUNTER-V1 random-number contract

`VCKSS-COUNTER-V1` is the Rust-native deterministic probe contract. It is distinct from the registered Stata `mt64s` compatibility contracts and must never be described as fixed-seed equality with those contracts.

## Generator

The core generator is Philox4x64-10 with the standard multipliers and Weyl increments:

```text
M0 = d2b74407b1ce6e93
M1 = ca5a826395121157
W0 = 9e3779b97f4a7c15
W1 = bb67ae8584caa73b
```

The two 64-bit Philox key words are obtained by applying the in-tree SplitMix64 permutation to `seed` and to `seed xor d1b54a32d192ed03`. Ten Philox rounds are applied. The Weyl key increment is applied between rounds and not after the last round.

## Logical address

One block is addressed by:

```text
counter[0] = canonical entity key
counter[1] = floor(logical probe / 4)
counter[2] = physical 64-bit word index
counter[3] = fixed domain tag
lane       = logical probe modulo 4
```

The domain tags are frozen in `rust/crates/vckss-core/src/rng.rs`. Leverage and target calculations use different tags. Diagnostic, retry, and self-test domains are also separate so adding draws in one domain cannot shift another domain.

The canonical entity key is an estimator-semantic rank, not a storage offset or hash-table iteration index. A caller must therefore finish canonicalization before requesting atoms.

## Rademacher sums

A semantic atom with `n` physical trials is generated from exactly `ceil(n/64)` Philox words. Every selected bit is one independent pseudorandom Rademacher sign under the counter contract. The returned sum is

```text
2 * popcount(selected bits) - n
```

This exactly preserves the finite Bernoulli-bit distribution of the registered counter generator and avoids an approximation to a binomial draw. Trial counts must be positive exact binary64 integers not exceeding `2^53`. A defensive per-atom word cap prevents a malformed frequency from triggering an unbounded loop; normal panel frequencies are far below it.

## Invariance

For a fixed contract version, seed, domain, logical probe, canonical entity, and physical trial count, the atom is independent of:

- thread count and scheduling;
- probe batch width and batch boundaries;
- solver routing and iteration history;
- input row order after canonicalization;
- operating system and CPU architecture.

Permanent hexadecimal block, word, compressed-sum, and batch-partition vectors are compiled into the Rust tests. Any intentional change requires a new contract name.

## Compatibility mode

A future Stata-compatibility mode may reproduce a registered `mt64s`/`rbinomial()` contract. It will have a separate name and test vectors. It must not silently alias `VCKSS-COUNTER-V1`.
