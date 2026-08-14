# Adjudication: KSS-BC-DERIVATION-B

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-DERIVATION-B/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-DERIVATION-B.md`
- Packet SHA-256: `54dad46585ac84a775a207f2565d339f3731de02c5c3f1c73fc8ca949be9d0df`
- Adjudication date: 2026-08-14

## Disposition

The independent B review confirmed the A review's algebraic verdict and
separately identified the same conditioning, physical-probe, spectral-margin,
and caller-coverage obligations. Every B finding is accepted and repaired in
the same files and tests listed in `KSS-BC-DERIVATION-A.md`.

The B review's requested `f=(2,1)` fourth-moment and per-copy leverage example
is now an explicit exhaustive test. Its zero-denominator examples are also
registered. Its request to quarantine coefficient two is enforced by a
production-source static audit; the maintained MATLAB coefficient-two formula
remains audit-only provenance and is not a runtime option.

The review remains `valid_with_repairs` and `ai_reviewed` evidence only. KB5
awaits the fresh repaired caller-inclusive challenge.
