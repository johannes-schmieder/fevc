# Independent saved-draw audit

`audit.py` reconstructs the completed comparison from raw task outputs without
importing the campaign validator or estimator. It checks immutable bindings,
the complete inventory, numerical interval identities, original-saved-draw and
paired point agreement, summaries, diagnostics and the prospective readiness
decision. It does not create new outcome draws or establish an asymptotic claim.

From the repository root:

```bash
./.venv/bin/python rust/experiments/unified_residual_moment_audit/audit.py RUN NEW_RECEIPT
FEVC_UNIFIED_AUDIT_RUN=RUN ./.venv/bin/python -m pytest -q rust/experiments/unified_residual_moment_audit/test_audit.py
```

The output receipt must not already exist. The optional completed-run tests
use temporary result copies and read-only symlinks; they never alter evidence.
Without the environment variable, the five corruption tests are explicitly
skipped and the twelve arithmetic tests still run.
