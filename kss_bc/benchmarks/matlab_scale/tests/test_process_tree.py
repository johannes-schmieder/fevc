import pytest
from common import BenchmarkError, validate_process_identity
from monitor_process_tree import (
    certified_identity_sample,
    descendant_pids,
    process_snapshot,
)


def write_process(proc_root, pid, ppid, rss_kib):
    process = proc_root / str(pid)
    process.mkdir()
    process.joinpath("stat").write_text(
        f"{pid} (test) S {ppid} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
        encoding="utf-8",
    )
    process.joinpath("status").write_text(
        f"Name:\ttest\nVmRSS:\t{rss_kib} kB\n",
        encoding="utf-8",
    )


def test_process_tree_includes_client_workers_and_nested_children(tmp_path):
    write_process(tmp_path, 100, 1, 1000)
    write_process(tmp_path, 101, 100, 2000)
    write_process(tmp_path, 102, 100, 3000)
    write_process(tmp_path, 103, 101, 4000)
    write_process(tmp_path, 200, 1, 9999)
    snapshot = process_snapshot(tmp_path)
    selected = descendant_pids(snapshot, 100)
    assert selected == {100, 101, 102, 103}
    assert sum(snapshot[pid][1] for pid in selected) == 10_000


def identity_record():
    return {
        "schema": "kss_matlab_scale_process_identity_v1",
        "status": "PASS",
        "pid_api": "matlabProcessID_R2025a",
        "mode": "cold",
        "label": "scale4-well",
        "case_sha256": "a" * 64,
        "expected_pool_workers": 4,
        "client_pid": 101,
        "worker_indices": [1, 2, 3, 4],
        "worker_pids": [102, 103, 104, 105],
    }


def test_named_matlab_pids_are_required_in_one_summed_rss_sample():
    identity = validate_process_identity(identity_record())
    snapshot = {
        100: (1, 1000),
        101: (100, 2000),
        102: (101, 3000),
        103: (101, 4000),
        104: (101, 5000),
        105: (101, 6000),
        106: (101, 7000),
    }
    certified = certified_identity_sample(snapshot, set(snapshot), identity)
    assert certified == {"rss_kib": 28_000, "process_count": 7}


def test_anonymous_five_descendants_do_not_substitute_for_named_worker_pid():
    identity = validate_process_identity(identity_record())
    snapshot = {
        100: (1, 1000),
        101: (100, 2000),
        102: (101, 3000),
        103: (101, 4000),
        104: (101, 5000),
        999: (101, 6000),
    }
    assert certified_identity_sample(snapshot, set(snapshot), identity) is None


def test_process_identity_rejects_duplicate_worker_pids():
    record = identity_record()
    record["worker_pids"][-1] = record["worker_pids"][0]
    with pytest.raises(BenchmarkError, match="not distinct"):
        validate_process_identity(record)
