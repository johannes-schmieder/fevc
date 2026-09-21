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

## Requirements

- Stata 18 or 19.
- Precompiled native backends for macOS (Apple Silicon and Intel) and
  Linux x86-64; no compiler or Rust installation is needed.
- Windows binaries and testing are deferred.

## Installation

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/main/")
```

If you already use the Stata `github` command:

```stata
github install johannes-schmieder/fevc
```

Both routes install the command, help, and Mac/Linux native backends.
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
- [Installation](INSTALLATION.md)
- [Changelog](fevc/CHANGELOG.md)
- [Detailed documentation](fevc/docs/README.md)
- [Contributing](CONTRIBUTING.md)

## License

GPL-3.0-only for the package code. See [LICENSE](LICENSE),
[code licensing](CODE_LICENSE.md), and [third-party notices](THIRD_PARTY_NOTICES.md).
