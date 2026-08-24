from __future__ import annotations

import subprocess

SOURCE_COMMIT = "ef448e8c26c25c8d18b94fdcc2fd99011c630906"
script = subprocess.check_output(
    ["git", "show", f"{SOURCE_COMMIT}:.ci/codex/apply.py"],
    text=True,
)
old = '''expected = {
    "varcomp_kss/_vckss_rust_reconcile_exact_v7.ado": 1,
    "varcomp_kss/_vckss_rust_post_exact_v7.ado": 1,
    "varcomp_kss/tests/stata/test_rust_planned_compressed_post.do": 1,
}
'''
new = '''expected = {
    "varcomp_kss/_vckss_rust_reconcile_exact_v7.ado": 1,
    "varcomp_kss/_vckss_rust_post_exact_v7.ado": 1,
    "varcomp_kss/tests/stata/test_rust_planned_compressed_post.do": 1,
    "varcomp_kss/tests/stata/test_rust_public_install.do": 1,
}
'''
if script.count(old) != 1:
    raise SystemExit(
        f"staged exact-limit transformation audit anchor changed: found {script.count(old)}"
    )
script = script.replace(old, new, 1)
exec(compile(script, ".ci/codex/replayed-exact-limit.py", "exec"), {"__name__": "__main__"})
