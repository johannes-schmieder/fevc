from pathlib import Path

profile = Path(".ci/stata/requested-profile.txt")
current = profile.read_text()
if current != "quick\n":
    raise SystemExit(f"expected quick profile before diagnostic qualifier, found {current!r}")
profile.write_text("plugin-build\n")
