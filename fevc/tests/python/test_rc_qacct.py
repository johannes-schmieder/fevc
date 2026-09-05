from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location(
    "rc_qacct", Path(__file__).resolve().parents[2] / "tools/check_rc_qacct.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def record(task="undefined"):
    return ("================================\njobnumber 123\n"
            f"taskid {task}\nowner johannes\nproject welfgr\nslots 1\n"
            "failed 0\nexit_status 0\nru_wallclock 12.3\ncpu 11.2\n"
            "maxvmem 123.4M\nhostname scc-host\nqname shared\n")


def test_complete_scalar_and_array():
    assert len(MODULE.validate(record(), 123, {"undefined"})) == 1
    assert len(MODULE.validate(record("2") + record("1"), 123, {"1", "2"})) == 2


@pytest.mark.parametrize("payload", [
    "", "invalid", record("1"), record("1") * 2,
    record("1") + record("3"), record("1") + record("2").replace("failed 0", "failed 100"),
    record("1") + record("2").replace("exit_status 0", "exit_status 1"),
    record("1") + record("2").replace("jobnumber 123", "jobnumber 124"),
    record("1") + record("2").replace("slots 1", "slots 2"),
    record("1") + record("2").replace("cpu 11.2", "cpu nan"),
    record("1") + record("2").replace("maxvmem 123.4M\n", ""),
])
def test_rejects_incomplete_failed_or_mixed_accounting(payload):
    with pytest.raises(ValueError):
        MODULE.validate(payload, 123, {"1", "2"})
