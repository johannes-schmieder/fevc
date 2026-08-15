# Adjudication: ST3 worker-block Schur solver

## Review metadata

- Frozen packets:
  `theory/reviews/gpt-pro/requests/ST3-SCHUR-SOLVER-A/PACKET.zip` and
  `theory/reviews/gpt-pro/requests/ST3-SCHUR-SOLVER-B/PACKET.zip`.
- Packet SHA-256 values:
  `fd17f13764d8a8a133bc757c51f26a6a507b219e8bdb0541e4def031fcfa0fd7`
  and
  `e0a48ebabdd49c39fa5032e2adaad86d1e61dc7709c311b8a6f902e817a057ed`.
- Responses:
  `theory/reviews/gpt-pro/responses/ST3-SCHUR-SOLVER-A.md` and
  `theory/reviews/gpt-pro/responses/ST3-SCHUR-SOLVER-B.md`.
- Fresh-chat references:
  `https://chatgpt.com/c/6a7be52d-c520-83ea-af7b-2ae2d155708d` and
  `https://chatgpt.com/c/6a7be535-644c-83ea-b15f-dfa35df71219`.
- Visible model setting in both chats: Pro.
- Adjudication date: 2026-08-11.

The browser file chooser timed out before transmission. No upload or prompt
was submitted in those attempts. Each final fresh chat instead received the
same bounded, line-numbered source excerpts copied verbatim from its frozen
packet plus its distinct frozen packet hash and adversarial prompt. Neither
chat contained the other response or a desired verdict. Both final responses
are stored verbatim and pass `tools/reviews/validate_review_record.py`.

## Disposition table

| Review finding | Disposition | Repair or evidence | Remaining scope |
|---|---|---|---|
| The Schur action, transformed right-hand side, reconstruction, signs, transposes, dimensions, and exact reduced/full residual identity are correct. | accepted | Both reviews independently derived the block system. Dense tests compare the action on a full reduced basis and solves to `invsym(H)`. | Finite algebra only. |
| `S` is SPD only when the full retained quotient is full rank; `diag(R)` is a valid but potentially weak preconditioner. | accepted and documented | The companion now states both conditions and names the actual preconditioner. | Conditioning and performance remain ST3/ST8 obligations. |
| Naive squared norms can underflow for nonzero `1e-200*b`, producing false zero-RHS convergence. | accepted and repaired | Scalar and batched paths now use exact componentwise-zero detection, per-column max-absolute normalization, and scaled-sum-of-squares norms. Regression tests cover `1e-200` and `1e200` scaling. | PCG inner products still need residual-replacement/attainable-precision stress work before ST3 closes. |
| The initial reconstructed iterate was not accepted when its reduced residual already met the internal threshold. | accepted and repaired | Scalar and batched Schur paths test the initial reduced residual against the original normalized RHS. A nonzero RHS with exactly zero Schur RHS converges in zero iterations. | None for this branch. |
| The final full residual acted only as a veto and could not accept a `MAXITER` result already within the public tolerance. | accepted and repaired | A finite recomputed full residual at or below tolerance now determines `CONVERGED`; an exceeded residual downgrades provisional convergence. | Per-column warnings for a preceding breakdown or max-iteration event are not yet stored. |
| A same-precision recomputation plus a factor-of-two margin is not a rigorous certificate of the exact-real residual or forward error. | accepted; claim narrowed | Help, README, plan, changelog, and companion now call this an operational recomputed double-precision residual check. | A rigorous rounding-error upper bound or higher-precision residual is not implemented. |
| Worker--firm connectivity does not certify a full-rank quotient after adding a third FE block. | accepted and gated | Public `absorb()` now stops with a multiway-rank message. Internal generalized operators remain development code only. | A scalable multiway quotient-rank certificate is required before reopening `absorb()`. |
| Batched recurrences are columnwise in exact arithmetic but one column's numerical breakdown stops the entire batch. | accepted; open | Mixed nonproportional, zero, transformed-zero, tiny, and large columns now agree with separate dense/scalar solutions in passing cases. | Per-column status, iteration, and failure isolation remain ST3 work. |
| Empty batches, missing RHS values, and nonfinite preconditioner state were not rejected explicitly. | accepted and repaired | Entry checks now reject zero-column batches, missing RHS values, missing diagonals, and nonfinite/nonintegral iteration limits. | Broader overflow tests remain open. |
| Existing successful tests used a one-dimensional Schur system and proportional batch columns. | accepted and repaired | Tests now include a successful two-dimensional, two-iteration solve, full-basis Schur action, nonproportional mixed batches, scale extremes, invalid inputs, and exact transformed-zero RHS. | Near-singular networks, periodic explicit residuals, and per-column failure isolation remain open. |

## Local verification after repairs

- `ppml_talo/tests/run_all.do quick`: pass on Stata 18 after the repair.
- The new unit fixtures directly falsify the former `1e-200*b` zero-norm bug
  and compare `1e-200`/`1e200` solutions after rescaling.
- Both review records pass `tools/reviews/validate_review_record.py`.

## Status decision

The exact worker-block Schur algebra is `formula_verified` and has two new
`ai_reviewed` records. The decisive false-convergence path identified by both
reviews is repaired and has a regression test.

ST3 remains `IN PROGRESS`. It cannot close until the operational solver has
per-column batch failure isolation and iteration statuses, broader
ill-conditioning tests, and a documented attainable-precision/residual-
replacement policy. Multiway fixed effects are disabled until a full-rank
quotient certificate exists. No review here establishes statistical TALO
theory, runtime scalability, rigorous exact-real residual certification, or
human `independently_checked` status.
