# Archived VCKSS working tree

The predecessor `vckss/` working tree was removed from `main` on 2026-08-31
after the FEVC hard rename. It contained frozen benchmark output,
qualification evidence, predecessor documentation, and obsolete development
harnesses; no active FEVC runtime or build consumed it.

The complete directory remains available and byte-bound in Git:

- archive commit: `fccf47a915e6a9d6ddd1af89bac770364b7fe561`;
- `vckss/` tree: `aea812f447b29d4e2940b81608f03102990f523c`;
- [browse the archived tree](https://github.com/johannes-schmieder/fevc/tree/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss).

The release gates verify the pinned tree object and reject a recreated live
top-level `vckss/` directory. The relocation inventory under
`docs/migration/` remains an immutable description of the earlier rename; its
historical destination paths are intentionally not rewritten.

## Scalable projection and inference

- [focused scaling protocol and result](https://github.com/johannes-schmieder/fevc/blob/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/qualification/inference_matlab/SCALABLE_PROJECTION.md)
- [complete inference/MATLAB qualification directory](https://github.com/johannes-schmieder/fevc/tree/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/qualification/inference_matlab)
- [source-bound local forced-CMG receipts](https://github.com/johannes-schmieder/fevc/tree/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/qualification/inference_matlab/evidence/cmg_projection_local/33ede864111c319185949ede4ef6d2bcc44b1383)

## Performance records

- [full-CMG production evidence](https://github.com/johannes-schmieder/fevc/tree/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/benchmarks/full_cmg_production)
- [comparative-scaling harness and evidence](https://github.com/johannes-schmieder/fevc/tree/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/benchmarks/comparative_scaling)
- [projection/AKM scaling evidence](https://github.com/johannes-schmieder/fevc/tree/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/benchmarks/projection_akm_scaling)

## Development result reports

- [MATLAB package comparison](https://github.com/johannes-schmieder/fevc/blob/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/docs/MATLAB_KSS_VERSION_COMPARISON.md)
- [`PREP_RHS_1` result](https://github.com/johannes-schmieder/fevc/blob/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/docs/PREP_RHS_1_RESULTS_2026-08-19.md)
- [`FE_BUF_1` result](https://github.com/johannes-schmieder/fevc/blob/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/docs/FE_BUF_1_RESULTS_2026-08-19.md)
- [`PREP_BND_1` result](https://github.com/johannes-schmieder/fevc/blob/fccf47a915e6a9d6ddd1af89bac770364b7fe561/vckss/docs/PREP_BND_1_RESULTS_2026-08-19.md)

The predecessor CMG source-review manifest was also retained in the active
vendor provenance directory as
`rust/vendor/cmg/LEGACY_UPSTREAM_SOURCE_MANIFEST.yaml`.
