# fevc

`fevc` is a Stata command for bias-corrected variance decompositions in
worker–firm fixed-effects models, using the leave-out approach of Kline,
Saggio, and Sølvsten (2020). It reports the variance of worker effects, the
variance of firm effects, their covariance, and the variance of their sum.

The command supports controls, frequency weights, and leaving out either
whole worker–firm matches or single observations. It uses exact calculation
for small problems and a randomized approximation for large ones. A native
Rust plugin provides speed on large datasets, and a Mata implementation runs
wherever Stata runs. Point estimation is the default. Standard errors for
projections of fixed effects on covariates, and experimental confidence
intervals for the variance components, are available on request for
supported models; see the
[inference guide](fevc/docs/INFERENCE.md).

Because the randomized approximation depends on random draws, `fevc` reports
by default a numerical Monte Carlo standard error (MCSE) for each of the four
estimates. It measures numerical precision only, not sampling uncertainty,
and does not cover projections or confidence intervals. Use `mcse(off)` to
skip it. See the [usage guide](fevc/README.md).

By default, the outcome mean is subtracted before the bias correction
(`centering(mean)`), which can make the correction considerably more
precise, particularly when the outcome mean is far from zero.
`centering(none)` gives the uncentered estimator of the Kline, Saggio, and
Sølvsten article, and `centering(corrected)` adds an adjustment for the bias
that centering can introduce in point estimates. Projections and confidence intervals accept
`mean` and `none`; with `mean`, confidence intervals treat the observed mean
as fixed and ignore the uncertainty from estimating it. See the
[centering guide](fevc/docs/CENTERING.md).

Confidence intervals for the variance components are experimental and rely
on additional assumptions. In the package's simulation assessment, the exact Mata method
produced intervals too rarely in three of its four primary designs, and the
native calculation, which produced intervals in all eight primary designs,
was materially sensitive to numerical settings.
There is no general coverage guarantee; read the
[assessment](fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
before reporting intervals.

## Requirements

- Stata 18 or 19.
- Precompiled native plugins for macOS (Apple silicon and Intel), Linux
  x86-64, and Windows x86-64 are included; no compiler or Rust installation
  is needed. The Windows plugin passes build and unit tests but has not yet
  been tested inside Stata on Windows. See the
  [native plugin records](native/README.md) for platform testing details.

## Installation

Install `fevc` from SSC in Stata:

```stata
ssc install fevc
```

To update an existing installation:

```stata
ssc install fevc, replace
```

Alternatively, use the community-contributed
[`github` installer](https://github.com/haghish/github):

```stata
* Install the github command once, if needed
net install github, from("https://haghish.github.io/github/")
github install johannes-schmieder/fevc
```

Both methods install the command, help, and native plugins.
SSC provides the version distributed through SSC; `github install` installs
the current `main` branch, which may include newer changes.
Restart Stata after updating to load the new programs and native plugin.
For a specific release, offline installation, or upgrading an earlier
development version, see the
[installation notes](INSTALLATION.md).

## Example

With worker–firm panel data loaded:

```stata
* Bias-corrected worker–firm variance decomposition
fevc log_wage, worker(worker_id) firm(firm_id)

* Include year effects
fevc log_wage i.year, worker(worker_id) firm(firm_id)

* Show uncorrected estimates, estimated bias, and corrected components
estat decomposition, full

* Numerical precision of the randomized calculation
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
