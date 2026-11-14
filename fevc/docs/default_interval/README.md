# Monte Carlo evidence for the default interval

Compact records of the package Monte Carlo runs behind the default interval of `fevc` (one interval
per component: the normal interval, or the q = 1 interval when the leading direction's share of the
variance exceeds the rule threshold). The runs were made with the harness now kept in
`fevc/tools/default_interval/` (see its README); each run passes the plain Stata command, with the plugin
built from the named commit, through two-block synthetic panels of about 48,000 person-years.

Each directory holds the run's `manifest.json` and `manifest.sha256` (frozen before any task ran),
`cells.csv` and `rules.csv` from `aggregate.py` where the run was aggregated, and `confirmation.json` from
`confirm.py` for the two confirmation attempts. The files are copied unchanged except for one
substitution in each `manifest.json`: the machine-local directory in front of the staged package copy
(`pkgroot`) is written `<scratch>/pkgmc/`. `manifest.sha256` is the sha256 of the manifest as it was
frozen, with the original path; restoring that prefix reproduces it.

## Runs

**dev1** (development; package commit 250a760c; manifest sha256 `0e5aa61e...d5a9`). 16 designs, from no
bottleneck to a strong one, under match and observation deletion, 1,000 replications each (32,000 fits),
with the explicit options `inference(auto) inferencemodel(repeat_measurement)` and the package's first
threshold (share > 0.05). No fit failed, the inventory was clean and the recorded choice was reproduced
in every replication. Candidate rules were compared on these draws; the threshold 0.10 was chosen from
them and then confirmed on new designs. No verdict: a development run is not a confirmation.

**conf1** (first confirmation; package commit fb49e16d; manifest sha256 `6aee3af8...a200`). 20 new
designs (design seeds 31-50), both deletion modes, 1,000 replications each (40,000 fits), the plain default
command with rule threshold 0.10 and tolerances frozen in the manifest. Verdict **FAIL** on the inventory
criterion: one fit (design `c_J400_b300_g3_profile`, observation deletion, replication 761) returned point
estimates only, the documented fallback when the standard-error pass stops (here
`JLA_CONSTRAINT_FAILED`), and the harness of that time treated a fit without a model name as a
misconfigured output and rejected its whole task, so `confirmation.json` counts 39,950 replications. The
failure was the harness's: it had no rule for a fallback. `information_after_failure.json` records, for
information only and not as a verdict, the criteria computed with that fit counted as a replication
without an interval; every tolerance is met there. The verdict stands as registered.

**conf2** (second confirmation; package commit fb49e16d, same plugin; manifest sha256
`08427f94...a01f`). The designs, command, replications and tolerances of conf1, new outcome seeds, and the
harness corrected to count a fallback as an attempted replication without an interval (against the success
rate and as not covered). Verdict **PASS**: 40,000 fits, no failed fit, no fallback, inventory clean,
recorded choice reproduced in every replication; over the 160 cases (design, deletion mode, component) the
lowest coverage of the reported interval is 0.930 (the variance of the sum, which always takes the normal
interval) and the mean 0.9593.

**dev2_controls** (development check, not a confirmation; package commit fb49e16d; manifest sha256
`c8338dad...15c9`). Four designs with one control variable in the model (`"control": true`), both deletion
modes, 500 replications each (4,000 fits), plain default command. No fit failed and none fell back. The
coverage of the sum is 0.904-0.920 in all eight cases, below nominal; the cause recorded in the
development log is that the model of that commit treats the estimated control coefficient as known
(`e(inference_nuisance_omitted)` = 1). The first-order term of commit 02d7955f removes it (dev3_controls).

**dev3_controls** (development check, not a confirmation; package commit 02d7955f, the first with the
first-order term for estimated control coefficients; manifest sha256 `bf2d84c4...7209`). Thirteen designs,
both deletion modes, 500 replications each (13,000 fits), plain default command. The first four designs
and the run seed are those of dev2_controls, so the outcome draws are the same and the comparison is
paired: the coverage of the sum is 0.938-0.956 (before 0.904-0.920) and its standard error 0.97-1.04 of
the spread of the estimates (before 0.84-0.91). The other nine designs vary the control through the
harness's control options (unrelated to the effects; two controls; a coefficient of one, which makes the
control index as large as the effects; error variance moving with the control inside a cell, mild and
strong; much and little variation within matches); each manifest entry records what the fixture realises
(`control_realised`). No fit failed and none was without an interval; inventory clean; the recorded
choice reproduced in every replication. Lowest coverage 0.918 (the covariance, large control index, no
bottleneck, match deletion), mean 0.962.

**dev4_fixedoffset** (development check; package commit 02d7955f; manifest sha256 `f1aa4845...a514`). The
two large-index designs of dev3_controls with the option `nuisance(fixedoffset)`, 500 replications per
design and deletion mode (2,000 fits). Under match deletion the coverage is 0.948-0.986 and the standard
error of the covariance in the design without a bottleneck is 1.07 of the spread (0.88 under the default
joint nuisance in dev3_controls): the shortfall there belongs to the joint estimate, whose variance has a
term the standard error does not cover. Under observation deletion the option is not covered by the
standard errors; all 1,000 such fits returned point estimates with a reason, as documented, and count as
replications without an interval.

**dev4_worker** (development check; package commit 02d7955f; manifest sha256 `b3aca194...39de`). Two
designs with a control loaded on the worker effect (0.8, 0.2, noise 0.4), both deletion modes, 500
replications each (2,000 fits), plain default command. No fit failed; coverage 0.932-0.984.

## What is not here

- Raw task outputs (`out/`), Stata logs (`logs/`), the exported design CSVs (`designs/`) and the large
  `summary.json` files (dev1 1.2 MB, conf2 1.5 MB, dev2_controls 0.3 MB, dev3_controls 1.0 MB and the
  later runs) are kept outside the repository.
  The design CSVs can be regenerated with `fevc/tools/default_interval/designs.py`; their sha256 are in
  each manifest (`designs[name].sha256`) and the generator reproduces all of them.
- conf1 was not aggregated after its verdict, so it has no `cells.csv` or `rules.csv`.

## Paths and hashes in the manifests

`pkgroot` names the staged package copy with its plugin on the machine on which the runs were made, with
the directory in front of it replaced as described above. The `hashes` block binds each run to its plugin, the four hashed ado/Mata files, the task do-file, driver,
aggregator, verdict script and the design code as they were at the time. For dev1, conf1, conf2 and
dev2_controls these are the prototype files `lp_q1sim.py`, `lp6.py` and `akm_design.py`, which the imported
`designs.py` replaces; the imported harness files differ from those hashed files, so `pkgmc.py run` would
refuse to continue these runs. They are records, not runs to resume. The runs from dev3_controls on were
made with the harness in this repository and hash its files.
