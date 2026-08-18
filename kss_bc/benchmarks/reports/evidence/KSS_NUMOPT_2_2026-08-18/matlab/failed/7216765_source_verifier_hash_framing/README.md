# MATLAB attempt 7216765

This source-bound attempt stopped before MATLAB launched. The maintained
runtime tree was unchanged and reproduced its registered SHA-256 under the
existing `sha256sum files | sha256sum` framing. The new verifier mistakenly
used a different relative-path/digest framing and therefore rejected the same
184 files. Commit `ccd849a3be8c578958d5ea9c6b82dc3c51cb4d46` repairs the
verifier and adds a regression test. This attempt is retained as failed
harness evidence and contributes no runtime or scientific measurement.
