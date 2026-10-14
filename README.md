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
All five included native plugins are built from `002205f2` and include pooled
mover–stayer component inference and bounded diagonal residual refinement.
Mac and Linux runtime qualification passes; Mac x86-64 checks use Rosetta.
Windows build and unit checks pass, with Windows Stata runtime testing pending.
See the [adoption manifest](native/pooled-component-20261010/manifest.json)
for exact artifacts and tested scopes.

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

Version [0.5.0](https://github.com/johannes-schmieder/fevc/releases/tag/v0.5.0)
is the first public release. Install it in Stata with:

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/v0.5.0/")
```

Alternatively, use the community-contributed
[`github` installer](https://github.com/haghish/github):

```stata
* Install the github command once, if needed
net install github, from("https://haghish.github.io/github/")
github install johannes-schmieder/fevc
```

Both routes install the command, help, license notices, and native plugins.
The `net install` URL above pins version 0.5.0; `github install` follows `main`,
which currently contains 0.5.0 and may receive later development updates.
For an offline installation, download `fevc-0.5.0.zip` from the release,
extract it, and run `net install fevc, replace from("/path/to/extracted/fevc")`.
Restart Stata after updating to load the new programs and native plugin.
See [installation notes](INSTALLATION.md) if you have an older development
installation.

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

- [Technical supplement](https://johannes-schmieder.com/papers/fevc_technical_supplement.pdf)
- [Usage guide](fevc/README.md)
- [Outcome centering](fevc/docs/CENTERING.md)
- [Installation](INSTALLATION.md)
- [Changelog](fevc/CHANGELOG.md)
- [Detailed documentation](fevc/docs/README.md)
- [Contributing](CONTRIBUTING.md)

## License

GPL-3.0-only for the package code. See [LICENSE](LICENSE),
[code licensing](CODE_LICENSE.md), and [third-party notices](THIRD_PARTY_NOTICES.md).
