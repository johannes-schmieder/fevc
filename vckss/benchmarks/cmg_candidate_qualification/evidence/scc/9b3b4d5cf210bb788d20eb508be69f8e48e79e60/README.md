# SCC Stata processor-capability rejection

This immutable packet records the expected fail-closed preparation result for
VCkss source `9b3b4d5cf210bb788d20eb508be69f8e48e79e60`. It is infrastructure and
capability evidence only. It contains no estimator execution, candidate timing,
scientific result, promotion result, pilot result, or production result.

Run `20260827T221701Z-9b3b4d5-cmq-capgate` submitted only preparation job
`7340493`. The harness requested four unrestricted preparation slots and SCC
scheduled it on `scc-gr4`. Stata/MP 19 reported four licensed processors against
the registered requirement of 16. The wrapper therefore stopped at
`stata_processor_capability` before building either candidate/comparison plugin
or submitting the 72-task array. `qacct` records `failed=0`, `exit_status=1`,
one second of wall time, and 277.406 MiB maximum virtual memory.

`rejection.json` is the compact interpretation. The other files are exact
copies of the remote run identity, effective scheduler receipt, submission
receipt, Stata capability receipt, wrapper failure, and raw accounting record.
`SHA256SUMS` binds every file in this directory. This rejection does not weaken
the registered 1/8/16-core qualification or 1/2/4/8/16-core production design;
the exact design remains blocked until Stata licenses at least 16 processors.
