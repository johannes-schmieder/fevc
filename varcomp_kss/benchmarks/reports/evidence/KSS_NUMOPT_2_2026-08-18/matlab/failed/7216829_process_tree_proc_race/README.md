# Failed MATLAB comparison job 7216829

Job 7216829 ran `strong_f16_d2` under comparator commit
`ccd849a3be8c578958d5ea9c6b82dc3c51cb4d46` and bundle
`b92bfe307eafe51897cb52f4bc255828142498bdc524967a2a3dab94a4f76f77`.
MATLAB produced its application marker and aggregate after 1,578.721 command
seconds, but the independent process-tree monitor stopped on a transient
Linux procfs `ESRCH` (`ProcessLookupError`) while another process disappeared
during a scan. The wrapper therefore exited 2 and qacct records
`failed=0`, `exit_status=2`.

The process-tree artifact is `FAIL`; its partial 16,754,480-KiB peak and the
application outputs are not accepted comparison evidence. The original job,
task, scheduler request, stdout, qacct, application, resource, and failure
receipts are preserved here. A focused repair skips an individual procfs
entry on any per-process `OSError` while retaining the positive-sample and
five-named-PID acceptance gates. The task must run again under a new immutable
source bundle; this receipt is never revalidated retroactively.
