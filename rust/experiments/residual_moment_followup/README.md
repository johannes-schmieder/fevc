# Internal residual-moment follow-up

This is a research harness, not a Stata option or a production route. The
prospective protocol and pre-confirmation capture adjustment are in
`fevc/docs/observation_residual_moments_followup_v1*.json`.

`build.py OUTPUT` mechanically appends `adapter.rs` to a hash-checked copy of
the original independent observation oracle. It leaves that oracle untouched.
`--tests` builds its tests plus independent physical-block/collapsed-match
parity checks. Rust 1.85.1 is selected from rustup; an explicit toolchain bin
directory can be supplied using `FEVC_RUST_TOOLCHAIN_BIN`.

Use the repository Python environment. `run.py` provides `development`,
`paired`, `freeze`, and `confirmation` stages, each taking a new output directory
and `--executable PATH`. It refuses existing output files. Freeze creates an
immutable source bundle and a manifest binding all tasks, geometry inputs,
executable, and scientific thresholds before confirmation outcomes exist.
Confirmation refuses source/input drift. Test the full pipeline first with
`pytest test_harness.py`; `FEVC_FOLLOWUP_TEST_EXECUTABLE` selects the executable.

Actual JLA diagonals come from an outcome-free deterministic response prepass.
The attachment uses 1024 fixed spectrum iterations (the initial 128-iteration
development capture failed on near-tied diffuse modes); its spectral tolerance
is unchanged. Its deterministic capture response is also scaled by 10000 to
keep its tiny artificial covariance above an absolute roundoff envelope. A
paired check preserves bitwise h/B-diagonal identity under that positive
rescaling; actual simulation outcomes are not rescaled. The two initial capture
failures and a superseded outcome-free preflight are retained. No attachment
interval is a simulated interval here. The new
candidate is deliberately tested off its exact-leverage contract in the JLA
arms, which does not extend its supported API. The experiment holds point
kernels, covariance traces, and q1 reference quadrature exact.

Scaling uses the real full-quotient diagonal model solver for every projection
probe. Reported candidate memory is an additional admission envelope, not
whole-process RSS; the research fixture's dense coefficient matrices and
oracle setup are outside its production path. Match comparisons use the same
physical observation population and independent physical errors, not a new
qualification under arbitrary within-match dependence.
