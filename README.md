# fevc

`fevc` estimates bias-corrected variance components in worker–firm fixed-effects
models in Stata, using the leave-out approach of Kline, Saggio, and Sølvsten.
It reports worker-effect variance, firm-effect variance, their covariance,
and the variance of their combined contribution.

The command supports controls, frequency weights, and match or observation
deletion. It provides exact calculation for small problems and a randomized
approximation for larger datasets, with a native Rust backend for supported
requests. Point estimation is the default; inference is available for
[supported models](fevc/docs/INFERENCE.md).

The default `mcse(all)` reports approximate numerical MCSE for the four main
point estimates. Use `mcse(off)` to skip its additional calculation and display.
MCSE can accompany supported projection and sampling-inference requests; it
does not describe those additional outputs. See [usage](fevc/README.md).

Outcome centering is available in the current repository source through
`centering(none|mean|corrected)`, with Mean as the default for both exact and
JLA. Mean adds only a frequency-weighted mean and subtraction. Corrected adjusts for estimating
that mean, using a shared exact system or the existing JLA full/two-half
leverage pools. MCSE holds the mean fixed and, for Corrected, also holds its
extra increment fixed. Projection and component `inference(highrank|q1)`
support Mean (the default) and None on their existing supported tuples. Mean
component inference treats the observed mean as fixed and omits its estimation
uncertainty; it is a working approximation, not conditional inference given
the observed mean. Corrected is unavailable with either attachment. See
[the centering guide](fevc/docs/CENTERING.md).

Mata supports these source options. Native point centering needs centering
API 1; Mean projection additionally needs projection-centering API 1, and
Mean component inference needs component-centering API 1. Existing combined
exact-Mata requests support Mean; native combined component/projection
requests remain unsupported.
New Mac candidates at `b9f80ce9` pass full qualification and installed Mean
component checks for thin arm64, thin Rosetta x86-64 and universal on both
architectures. Linux passes full and installed Mean checks at `63757839`.
These candidates are preserved but not adopted: Windows remains blocked after
two private source-build smoke failures. The prepared hosted-build alternative
awaits approval for source publication before all five payloads qualify and a
bounded retry. Shipped binaries retain their preceding capabilities. See
[native provenance](native/README.md) for exact artifacts and status.

The [completed bounded assessment](fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
retains 22 exact-Mata availability-screen failures. All eight native primary
cells pass the broad descriptive screens with 100% interval availability;
material numerical sensitivity and earlier calibration limits remain. There
is no general coverage claim.

## Requirements

- Stata 18 or 19.
- Precompiled native backends for macOS (Apple Silicon and Intel), Linux
  x86-64, and Windows x86-64; no compiler or Rust installation is needed.
- Qualification is specific to the platform and exact artifact; see
  [platform evidence and limitations](native/README.md).

## Installation

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/main/")
```

If you already use the Stata `github` command:

```stata
github install johannes-schmieder/fevc
```

Both routes install the published command, help, and native backends for all
three platforms. They do not yet deliver the October 8 local candidate.
Restart Stata after updating. See [installation notes](INSTALLATION.md) if
you have an older development installation.

## Example

With worker–firm panel data loaded:

```stata
* Bias-corrected worker–firm variance decomposition
fevc log_wage, worker(worker_id) firm(firm_id)

* Include year effects
fevc log_wage i.year, worker(worker_id) firm(firm_id)

* Show uncorrected estimates, estimated bias, and corrected components
estat decomposition, full

* Numerical MCSE (all main point probes by default)
matrix list e(mcse)
estat diagnostics
```

Match deletion is the default. The displayed decomposition separates worker,
firm, and sorting contributions; sorting is twice the worker–firm covariance.

To run a self-contained example with simulated data:

```stata
fevc_run exact_controls using fevc.sthlp
```

This example restores your data when it finishes. See `help fevc` for syntax,
options, and more examples, or read the [usage guide](fevc/README.md).

## Documentation

- [Usage guide](fevc/README.md)
- [Outcome centering](fevc/docs/CENTERING.md)
- [Installation](INSTALLATION.md)
- [Changelog](fevc/CHANGELOG.md)
- [Detailed documentation](fevc/docs/README.md)
- [Contributing](CONTRIBUTING.md)

## License

GPL-3.0-only for the package code. See [LICENSE](LICENSE),
[code licensing](CODE_LICENSE.md), and [third-party notices](THIRD_PARTY_NOTICES.md).
