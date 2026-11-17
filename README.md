# fevc

`fevc` is a Stata command for bias-corrected variance decompositions in
worker–firm fixed-effects models, using the leave-out approach of Kline,
Saggio, and Sølvsten (2020). It reports the variance of worker effects, the
variance of firm effects, their covariance, and the variance of their sum.

The command supports controls, frequency weights, and leaving out either
whole worker–firm matches or single observations. It uses exact calculation
for small problems and a randomized approximation for large ones. A native
Rust plugin provides speed on large datasets, and a Mata implementation runs
wherever Stata runs. By default the command also reports repeat-measurement
standard errors and one confidence interval for each of the four components:
the normal interval where the estimate is close to normal, and otherwise the
`q = 1` interval of Kline, Saggio, and Sølvsten (2020, Section 7.1), with
their covariance posted as `e(V)` when every component is normal.
`inference(highrank)` asks for normal intervals and `inference(q1)` for the
`q = 1` interval wherever it exists; `inference(none)` gives point estimates
only. Standard errors for projections of fixed effects on covariates are
available on request (`project()`). See the
[inference guide](fevc/docs/INFERENCE.md) and the
[standard-error document](fevc/docs/repeat_measurement/unified/se_unified_20261116.pdf),
which derives the method and compares it with the KSS standard errors.

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
that centering can introduce in point estimates. Projections accept `mean`
and `none`. The standard errors of the components accept all three; their
simulated noise is centred as the outcome is, so no fixed-mean approximation
is made. See the [centering guide](fevc/docs/CENTERING.md).

The standard errors of the components rest on assumptions that the point
estimates do not need. Deletion units that repeat a worker–firm cell (a
stayer's observations, for example) receive their own sample variance; the
other units receive a fitted variance model, so for them the intervals are
as good as that model and not robust to arbitrary heteroskedasticity. The
choice between the normal and the `q = 1` interval was fixed after
development simulations and then passed registered confirmations through
the command, with and without controls; that is simulation evidence for
designs with at most one bottleneck, not a coverage guarantee. Read the
assumptions in `help fevc` and the [inference guide](fevc/docs/INFERENCE.md)
before reporting intervals.

## Requirements

- Stata 15 or newer (through Stata 19).
- Precompiled native plugins for macOS (Apple silicon and Intel), Linux
  x86-64, and Windows x86-64 are included; no compiler or Rust installation
  is needed. The plugins pass native Stata qualification on macOS, Linux,
  and Windows; Intel macOS tests use Rosetta on Apple silicon. See the
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

To install the tagged
[0.6.0 release](https://github.com/johannes-schmieder/fevc/releases/tag/v0.6.0)
directly from GitHub using Stata's built-in installer:

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/v0.6.0/")
```

Alternatively, use the community-contributed
[`github` installer](https://github.com/haghish/github):

```stata
* Install the github command once, if needed
net install github, from("https://haghish.github.io/github/")
github install johannes-schmieder/fevc
```

All three methods install the command, help, and native plugins.
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
- [Inference guide](fevc/docs/INFERENCE.md) and the
  [standard-error document](fevc/docs/repeat_measurement/unified/se_unified_20261116.pdf)
  ([sources and method note](fevc/docs/repeat_measurement/README.md))
- [Outcome centering](fevc/docs/CENTERING.md)
- [Installation](INSTALLATION.md)
- [Changelog](fevc/CHANGELOG.md)
- [Detailed documentation](fevc/docs/README.md)
- [Contributing](CONTRIBUTING.md)

## License

MIT for the package code. See [LICENSE](LICENSE),
[code licensing](CODE_LICENSE.md), and [third-party notices](THIRD_PARTY_NOTICES.md).
