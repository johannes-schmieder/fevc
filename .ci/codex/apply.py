from __future__ import annotations

import subprocess


payload = subprocess.check_output(
    ["git", "show", "HEAD^:.ci/codex/apply.py"],
    text=True,
)
old = 'end_marker = "program define _vckss_rust_exact, eclass"'
new = 'end_marker = "program define _vckss_impl, eclass sortpreserve"'
count = payload.count(old)
if count != 1:
    raise RuntimeError(f"planned-route anchor repair expected one match, found {count}")
payload = payload.replace(old, new, 1)
print("repaired planned-route program boundary")
exec(compile(payload, ".ci/codex/apply.py[parent]", "exec"))
