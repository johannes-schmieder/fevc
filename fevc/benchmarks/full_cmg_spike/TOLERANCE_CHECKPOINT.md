# Full-CMG tolerance checkpoint

Source `491edfff20cb15f1e1229639390bcb76d58b742b` implements the private fit/probe tolerance split against the exact archived standalone CMG commit `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`. The fixed 8,192-firm, 327,680-worker, 200-probe macOS case passed at every probe tolerance from `1e-6` through `1e-10`.

| Effective probe tolerance | Maximum probe iterations | Command seconds | Complete residual | Maximum corrected-target difference from `1e-10` | Difference / MCSE |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `1e-6` | 12 | 142.479 | `9.86e-8` | `6.61e-13` | `1.23e-6` |
| `1e-7` | 13 | 153.986 | `5.72e-9` | `4.97e-14` | `1.18e-7` |
| `1e-8` | 15 | 168.862 | `9.94e-10` | `1.07e-14` | `2.31e-8` |
| `1e-9` | 16 | 175.438 | `7.76e-11` | `1.11e-15` | `5.03e-9` |
| `1e-10` | 18 | 188.046 | `4.85e-12` | `0` | `0` |

The registered maintained-MATLAB command time is 171.733 seconds. The `1e-6` checkpoint is 17.0% faster than that comparator, but it does not meet the private 2× promotion gate. Tightening beyond `1e-6` produces no statistically meaningful movement in the four corrected targets and only increases solve work. The next bottleneck is therefore repeated-solve architecture, not tolerance policy.

This table is a source-bound, single-run development ladder. It is not the final alternating warm-run qualification. Exact identities, command template, residual gates, input hash, plugin hash, process timing, and host details are recorded in [tolerance_ladder_2026-08-25.json](tolerance_ladder_2026-08-25.json).
