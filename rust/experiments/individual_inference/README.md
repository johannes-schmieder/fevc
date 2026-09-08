# Public individual-inference development harness

This adapter calls the actual native prepare/V2-augmentation/capability/solve/
V5-export/release lifecycle. It appends small entrypoints to byte-verified,
unchanged historical DGP examples; it does not patch an estimator or inject
oracle variances into the measured calls. The high-precision match oracle is
a separate design preflight, not the method whose coverage is measured.

The registered development run is **FAIL**. See
`fevc/docs/INDIVIDUAL_INFERENCE_DEVELOPMENT_2026-09-06.md`. Do not repeat it,
change gates or start confirmation without the next owner decision.

Files:

- `build.py`: pinned Rust build, source/executable hashes and verified DGPs.
- `public_api.rs`: actual public native call and optional synthetic CSV export
  for Stata parity. Public numerical defaults remain explicit and frozen.
- `observation.rs`, `match.rs`: independent outcome generation and semantic
  replication keys from the unchanged DGP sources.
- `run.py`: immutable source/task manifest, isolated task outputs, strict
  validation, original scientific gates, all-point accounting, tiny preflight
  and split/reversed-task checks. Development requires a matching tiny result
  and Stata parity receipt. There is deliberately no confirmation command.
- `test_run.py`: malformed/partial/duplicate/status/seed tests and deliberate
  scientific failure. Run separately with repository-local pytest.
- `fevc/tools/check_individual_native_parity.py`: six public Stata comparisons
  on exactly the native-generated data, with omitted probe defaults.
- `fevc/tools/audit_individual_development.py`: independent raw-output count
  and summary reconciliation; it never changes the scientific verdict.

Use `./.venv/bin/python`; every build/campaign output directory must be new.
Builds are local and the Stata parity driver requires licensed local Stata.
The confirmation plan has 120,000 calls, but it remains blocked by development.
The completed 19,200-call results, source bundle, raw target output, logs and
audit are retained under the ignored directory
`.local/diagnostics/individual-inference-upgrade-20260906/`.

The post-run pre-V5 q0-export guard is a later transport-only change. It does
not change the failed run's recorded binaries or create new scientific evidence.

The separate `../individual_inference_followup/` now checks the owner-approved
512-iteration match-q0 budget. This historical adapter deliberately retains
its frozen 128-iteration match setting: it represents the original failed
development defaults, not the later q0 budget. Do not reuse it as a new
default-setting confirmation harness.
