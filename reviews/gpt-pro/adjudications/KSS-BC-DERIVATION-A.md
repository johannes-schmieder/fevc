# Adjudication: KSS-BC-DERIVATION-A

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-DERIVATION-A/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-DERIVATION-A.md`
- Packet SHA-256: `dd14661e342dce1cedd8ea713032b63b6d8fc55fc4912ce0a354bd205edeac6f`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| The mixed raw fourth moment has coefficient one, and the block inverse derivatives and `+B,-V` signs are correct through order `R^-1`. | accepted | Retained the formulas and added a production-source coefficient audit in `kss_bc/tests/python/test_package_layout.py`. |
| The expansion needs explicit pointwise order, plug-in-moment orders, and an interior spectral qualification. | accepted | `kss_bc/docs/JLA_FINITE_PROJECTION.md` now states the `R^-1` scope and `O_p(R^-3/2)` plug-in error. `kss_bc/docs/BLOCK_CONTROL_DERIVATION.md` states the spectral neighborhood and nonuniform boundary. |
| Withholding selects probe realizations and does not inherit the unconditional bias expansion. | accepted | The documentation now says one failed block aborts the entire command, no partial target is returned, and no conditional-unbiasedness claim is made. Exact two- and four-observation gate counterexamples are registered in `test_jla_formula.py`. |
| Literal physical-copy probes do not equal fresh stored-row Rademachers. | accepted | The repaired derivation gives the expansion isometry, the standardized-sum fourth moment `3-2/f`, the unnormalized binomial aggregate used by Mata, observation leverage scaling, match contractions, and target centering. `test_frequency_probes.py` exhaustively enumerates the physical law; `test_frequency.do` enters through the public command with frequencies and varying controls. |
| Production callers and coefficient coverage were absent from the packet. | accepted | The repaired review packet includes `kss_bc.ado`, `kss_bc.mata`, the public Stata integration tests, and the static coefficient audit. |
| Approximate inverse actions need separate numerical control. | accepted as a numerical qualification | Every PCG action recomputes a full-system residual; the maximum is returned. The documentation now distinguishes this numerical evidence from exact projection algebra and from a forward-error theorem. |
| A favorable finite-probe leverage draw can falsely accept a truly nonestimable joint-control deletion. | accepted after local reproduction | The submitted runtime falsely accepted 22 of 100 two-probe seeds on the registered block-only-control design. API level 6 adds a deterministic within-cell scatter certificate before any randomized joint-control result is accepted. The same 100 seeds now produce zero accepts, and `UNVERIFIED_DELETION_RANK` is tested as a typed failure. |
| The documented 600,000-replication result was not reproduced by the submitted 150,000-draw test. | accepted | The registered test now runs the documented 600,000 draws and checks both the fixed-seed estimate and MCSE. |

## Status decision

The response remains `valid_with_repairs` and is `ai_reviewed` evidence only.
The repairs change the reviewed candidate, so KB5 is not closed by this
adjudication. A fresh caller-inclusive Pro review is required before closure.
