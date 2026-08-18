# Code license

## Selected license

The owner selected the GNU General Public License, Version 3 only
(`GPL-3.0-only`) on 2026-08-18 for the CMG implementation and for any
distributed package that forms one program with that implementation. The full
license text is in [`LICENSES/GPL-3.0-only.txt`](LICENSES/GPL-3.0-only.txt).

## Covered code

Subject to the file-level third-party notices and exclusions below, the grant
covers:

- `shared/cmg/**` code, tests, build tools, and generated CMG artifacts;
- the CMG-containing KSS package code under `kss_bc/**` when distributed with
  or as part of the CMG implementation; and
- CMG-containing PPML package code under `ppml_talo/**` if and when that
  adapter is distributed with or as part of the CMG implementation.

Repository-authored files in those covered distributions are offered under
GPL-3.0-only. Adapted CMG files retain the copyright and GPL notices of their
original authors. A file-level provenance manifest must identify copied and
modified third-party source.

## Not covered by this grant

This is not a license for the whole repository. It does not grant rights to:

- `application/**` imported code, data, or provenance material;
- manuscripts, releases, source notes, proofs, reviews, or archived material;
- restricted, licensed, confidential, or excluded data; or
- third-party dependencies except under their own licenses.

Those materials may appear in the same repository as a collection or
aggregate without becoming GPL-covered. Their existing notices and use terms
remain controlling.

## Distribution condition

Any conveyed object code must satisfy GPLv3's corresponding-source and notice
requirements. `CMG-MATA-1` is implemented entirely in Mata and includes no
compiled CMG helper, MEX file, Stata plugin, subprocess, or binary interchange
layer. Public distribution still requires a human review of the exact package
boundary, corresponding-source bundle, upstream notices, and third-party/data
exclusions. This document records the owner's license selection; it is not
legal advice.
