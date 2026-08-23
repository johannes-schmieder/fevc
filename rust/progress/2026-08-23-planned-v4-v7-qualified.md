# Planned V4/V7 licensed plugin qualification

Date: 2026-08-23

## Exact tested source

- repository: `johannes-schmieder/varcomp_kss`
- branch: `codex/rust-backend-completion`
- source SHA: `0d47e7e1a464c068c0cd61d8a1fcb6d0738e42eb`
- workflow: `Licensed Stata CI`
- profile: `plugin-build`
- status: `success`
- Stata RC: `0`
- process RC: `0`
- run ID: `32620645440`

Rust and Stata were not executed in the ChatGPT sandbox. All evidence below
comes from the private self-hosted Mac runner and is bound to the source SHA
above.

## Qualified boundary

The private Stata boundary now dispatches solve V4 and reconciles the additive
V7 execution-plan receipt. The qualified path covers:

- capability request V3 and solve request V4;
- execution-plan V1 and detailed receipt V7;
- requested and selected algorithm, engine, and solver route;
- automatic route selection and setup-only fallback receipts;
- independent leverage and target batch plans;
- advisory wall-work planning;
- Counter-V1 planned and actual accounting;
- solve-scope and end-to-end memory lifetime distinctions;
- complete numerical residual and accounting checks; and
- release and idle-state cleanup.

The Stata reconciler preserves the frozen V6 prefix contracts while treating
V7 as authoritative for the actual planned generic route. In particular, the
legacy generic prefix remains diagonal/no-fallback, while V7 exports the true
requested and selected route. V7's command-memory field is the post-prepare
solve peak; the older public command peak remains the maximum of preparation
and solve peaks.

## Comprehensive evidence

The successful profile performed the repository's comprehensive qualifier,
including:

- Rust 1.81 formatting, strict Clippy, and workspace tests;
- authenticated pinned public Stata SPI inputs;
- C shim, interrupt, error-transport, and ABI-header fixtures;
- release arm64 and x86_64 slices;
- signed universal macOS plugin construction;
- export, dependency, deployment-floor, and signature audits;
- fresh native arm64 Stata processes in isolated temporary directories;
- Rosetta x86_64 execution;
- thin and universal candidate loading;
- exact, compressed, explicit generic, and planned V4/V7 private routes;
- isolated installation of the new V4/V7 helper files;
- lifecycle, Rust-versus-Mata/reference, fixed-oracle differential, and
  numerical residual checks.

The receipt reports native arm64 `PASS_NATIVE`, x86_64 `PASS_ROSETTA`, and
classification `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`. This is strong
same-machine development and compatibility evidence; it is not native Intel,
Windows, Linux, public-release, or cross-hardware performance evidence.

## Next recovery point

1. Preserve Mata selection for omitted `backend()` and `backend(auto)`.
2. Expose the qualified planned route for explicit `backend(rust)` requests.
3. Post the full V7 routing, batching, wall, Counter, and memory receipt in
   `e()` and add public routing/receipt/failure tests.
4. Run exact-SHA quick gates after every focused checkpoint and repeat the
   comprehensive plugin profile after public-boundary closure.
5. Then implement Rust exact parity for `stayers(both)` and continue large-N
   performance and memory work.
