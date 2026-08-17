from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PROGRAM = ROOT / "kss_bc/benchmarks/scc/process_tree_rss.awk"


def process_tree_rss(root_pid: int, snapshot: str) -> int:
    completed = subprocess.run(
        ["awk", "-v", f"root={root_pid}", "-f", str(PROGRAM)],
        input=snapshot,
        text=True,
        capture_output=True,
        check=True,
    )
    return int(completed.stdout.strip())


def test_process_tree_rss_excludes_unrelated_processes() -> None:
    snapshot = """\
200 1 4000
201 200 5000
100 1 10
102 101 30
101 100 20
300 201 6000
"""
    assert process_tree_rss(100, snapshot) == 1024 * (10 + 20 + 30)


def test_process_tree_rss_handles_root_without_descendants() -> None:
    snapshot = """\
100 1 17
200 1 9000
"""
    assert process_tree_rss(100, snapshot) == 1024 * 17


def test_process_tree_rss_returns_zero_for_absent_root() -> None:
    snapshot = """\
200 1 4000
201 200 5000
"""
    assert process_tree_rss(100, snapshot) == 0
