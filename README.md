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
extra increment fixed. Projection supports Mean (the default) and None;
component inference requires explicit `centering(none)`. Corrected is not
available with projection. See
[the centering guide](fevc/docs/CENTERING.md).

Mata supports these source options. Native point centering needs centering
API 1; Mean projection additionally needs projection-centering API 1.
The local Mac arm64, Rosetta x86-64 and universal candidates at source
`24754269` pass clean build/runtime qualification and 24 installed-capability,
point-centering and Mean-projection checks. Linux x86-64 also passes full
qualification, installed point centering and
Mean projection at the same source (SCC job `7962808`). Windows runtime
qualification remains pending after another
private smoke failure. The existing Windows payload retains its owner-approved
manual-test status. This local candidate has not been published or tagged;
see [native provenance](native/README.md) for exact artifacts and scope.

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
