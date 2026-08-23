from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
text = path.read_text()

old_fixture = """generate long frequency = 1+mod(5*row0+2,3)
generate double target_weight = .75+(row0+1)/192
generate byte touse = 1

quietly count
"""
new_fixture = """generate long frequency = 1+mod(5*row0+2,3)
generate double target_weight = .75+(row0+1)/192

quietly count
"""
if text.count(old_fixture) != 1:
    raise SystemExit("compressed fixture sample-marker anchor changed")
text = text.replace(old_fixture, new_fixture)

old_signature = """quietly _datasignature
local caller_signature `\"`r(datasignature)'\"'

capture noisily _vckss_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight touse `nscope' `ncomplete' `nstayers'      ///
"""
new_signature = """quietly _datasignature
local caller_signature `\"`r(datasignature)'\"'

// The direct runner owns its marked-sample variable.  Create that disposable
// marker only after freezing the caller-data signature; ereturn post consumes
// it as the active e(sample) without making it part of caller data.
tempvar internal_touse
generate byte `internal_touse' = 1

capture noisily _vckss_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `internal_touse' `nscope' `ncomplete' `nstayers' ///
"""
if text.count(old_signature) != 1:
    raise SystemExit("compressed direct-runner signature anchor changed")
text = text.replace(old_signature, new_signature)

old_restore = """local sortedby_after : sortedby
assert `\"`sortedby_after'\"' == `\"`caller_sortedby'\"'
quietly count if touse != 1
assert r(N) == 0
// This standalone internal call posts e(sample) against the caller-owned
// touse variable.  Its result and sample have been fully checked above; clear
// that internal estimation result before comparing the raw caller dataset.
// The actual public commands below retain their active e(sample) while their
// complete data-restoration signatures are checked.
quietly ereturn clear
quietly _datasignature
assert `\"`r(datasignature)'\"' == `\"`caller_signature'\"'
"""
new_restore = """local sortedby_after : sortedby
assert `\"`sortedby_after'\"' == `\"`caller_sortedby'\"'
// Keep the validated internal result and e(sample) active.  Because its
// disposable marker was created after caller_signature, this comparison
// covers every caller data variable and detects any raw-data mutation.
quietly _datasignature
assert `\"`r(datasignature)'\"' == `\"`caller_signature'\"'
"""
if text.count(old_restore) != 1:
    raise SystemExit("compressed direct-runner restoration anchor changed")
text = text.replace(old_restore, new_restore)

path.write_text(text)
