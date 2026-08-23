# Licensed Stata CI runner

This repository uses one private, repository-level GitHub Actions runner on the
Mac Studio to execute licensed Stata/MP. The runner is privileged: repository
workflow code runs as the logged-in `johannes` macOS account. Do not enable this
workflow for public repositories, forks, or untrusted pull requests.

## Machine and Stata

- Host: Apple Silicon Mac Studio (`arm64`), macOS 26.5.2.
- Stata: Stata/MP 18, bundle version 18.0.130.
- Executable: `/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp`.
- Batch form: `/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do FILE.do [arguments]`.
- Stata 18 on this Mac returns shell code 0 even for an uncaught Stata error.
  The `stata_rc` written by `ci/stata_ci.do` is therefore authoritative.
- The current user profile is
  `/Users/johannes/Library/Application Support/Stata/profile.do`; it was empty
  when the runner was installed.

The machine's AC sleep setting is `sleep 0`. The user must remain logged in when
the runner is managed as a per-user LaunchAgent. No inbound port or SSH service
is required; the runner makes outbound connections to GitHub.

## Repository interface

Run a profile locally with:

```sh
./ci/run_stata_ci.sh version
./ci/run_stata_ci.sh smoke
./ci/run_stata_ci.sh quick
./ci/run_stata_ci.sh full
```

Profiles are data-driven in `ci/stata_profiles.json`. Each run copies only
tracked and non-ignored repository files into a fresh temporary directory,
isolates Stata PLUS/PERSONAL directories, launches a fresh Stata process group,
and kills only that group on timeout. Local evidence is written under
`.ci/stata/run/`, which is ignored by Git.

The existing quick/full fixture suite uses `egen ... = nvals()`. Those profiles
seed only `_/_gnvals.ado` from the current user's installed `egenmore` package
into the temporary PLUS tree. The default source is
`~/Library/Application Support/Stata/ado/plus`; set `STATA_PLUS_SOURCE` to an
equivalent PLUS root when reconstructing elsewhere. No other home-directory
content is scanned or copied.

The workflow is `.github/workflows/stata-ci.yml`. Pushes to `main` and
`codex/**` run the Rust 1.81 formatting/lint/test checks and the Stata/Mata
`quick` suite sequentially on this Mac. Manual dispatch supports `version`,
`smoke`, `quick`, and `full` Stata profiles without automatically repeating the
Rust quick lane. There is deliberately no `pull_request` trigger.

The GitHub-hosted Linux/Windows/macOS Rust matrices in `rust-backend.yml` and
`rust-stata-backend.yml` are manual qualification workflows. They do not run on
ordinary pushes or pull requests and therefore do not duplicate the Mac lane.

Artifacts retain full logs and the raw receipt for 30 days. A separate
least-privilege publisher job on the same repository runner commits:

- `.ci/stata/results/<tested-sha>.json` for every completed run;
- `.ci/stata/latest.json` only when no newer non-receipt commit exists.

Receipt-only commits contain `[skip ci]`, and both receipt paths are excluded
from workflow triggers. A remote client must match the full `tested_sha`, not
merely trust `latest.json`.

## Receipt schema

Schema version 1 includes `tested_sha`, `run_id`, `run_attempt`, `profile`,
`status`, `failure_kind`, `process_rc`, `stata_rc`, Stata version/edition,
platform, runner name, timestamps, duration, and simple pass/fail counts.
Failures distinguish launch errors, timeouts, crashes or missing status,
ordinary Stata errors, and missing required outputs.

## Runner installation and operation

- Installation: `/Users/johannes/actions-runners/varcomp-kss-stata`.
- Runner version at installation: 2.336.0 (automatic runner updates enabled).
- Runner name: `macstudio-stata-mp18-varcomp-kss`.
- Scope: repository-level, private repository
  `johannes-schmieder/varcomp_kss` only.
- Labels: `self-hosted`, `macOS`, `ARM64`, `stata`, `stata-mp`, `stata18`.
- Work directory: `_work` below the installation directory.
- Service: standard GitHub per-user LaunchAgent at
  `/Users/johannes/Library/LaunchAgents/actions.runner.johannes-schmieder-varcomp_kss.macstudio-stata-mp18-varcomp-kss.plist`.

Manage the runner from its installation directory:

```sh
./svc.sh status
./svc.sh stop
./svc.sh start
```

The service was empirically tested with the same real Stata workflow used in
CI, not only with an online/idle check. Because this is a per-user LaunchAgent,
`johannes` must remain logged into the macOS GUI session. The configured AC
sleep value is zero; display sleep does not stop the runner.

For an offline or stuck runner, first inspect `./svc.sh status`, the repository's
**Settings → Actions → Runners** page, and these bounded service logs:

```sh
tail -n 200 "/Users/johannes/Library/Logs/actions.runner.johannes-schmieder-varcomp_kss.macstudio-stata-mp18-varcomp-kss/stdout.log"
tail -n 200 "/Users/johannes/Library/Logs/actions.runner.johannes-schmieder-varcomp_kss.macstudio-stata-mp18-varcomp-kss/stderr.log"
```

To unregister, stop and uninstall the LaunchAgent with `./svc.sh stop` and
`./svc.sh uninstall`, then use the repository runner settings page to remove the
runner (or supply a newly generated removal token to `./config.sh remove`). To
re-register, obtain a new short-lived registration token from that page and run
`./config.sh` again with the recorded name, labels, and `_work` directory, then
reinstall/start the service. Registration or removal tokens must never be
committed, logged, or copied into this file.

## Security assumptions

- The GitHub repository remains private and only trusted collaborators have
  write access.
- Actions from public forks and untrusted pull requests are not enabled.
- Artifact paths are explicit; workflows never upload the home directory.
- Tests use synthetic fixtures only unless research data is explicitly approved.
- Development packages are tested from temporary directories, not installed in
  the permanent Stata PLUS tree.
