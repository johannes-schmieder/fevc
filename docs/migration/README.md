# Migration provenance

This repository was extracted from the private
`johannes-schmieder/ppml-variance` repository on 2026-08-18 without rewriting
the source repository's published history.

`ppml-variance-commit-map.tsv` maps each retained original commit to its
filtered `varcomp_kss` commit. Historical benchmark receipts and paper
manifests deliberately retain the original PPML-repository commit identifiers
under which the computations were performed. Use this map to locate their
corresponding filtered commits.

The extraction retained `kss_bc/`, `shared/`, KSS/CMG root plans and licenses,
and the KSS/CMG review records. It excluded the PPML proof, manuscript,
application, software, and unrelated review histories. The KSS paper was moved
to a separate fresh-history repository named `varcomp_kss_paper`.

Terminology note: active documentation now uses **KSS Matlab package/code**.
Unless explicitly identified as **KSS Matlab Econometrica replication code**,
it refers to GitHub `LeaveOutTwoWay`, commit `8b957ffe` (October 17, 2024;
latest version verified September 10, 2026). Historical records below retain
their original wording, source identities and results.
