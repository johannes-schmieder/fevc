from __future__ import annotations

import errno
import importlib.util
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "run_stata_ci", ROOT / "ci" / "run_stata_ci.py"
)
assert SPEC is not None and SPEC.loader is not None
RUN_STATA_CI = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUN_STATA_CI)


class RemoveTreeWithRetriesTests(unittest.TestCase):
    def test_retries_enotempty(self) -> None:
        target = Path("/private/tmp/vckss-stata-ci-test")
        rmtree = mock.Mock(
            side_effect=[
                OSError(errno.ENOTEMPTY, "Directory not empty", target),
                OSError(errno.ENOTEMPTY, "Directory not empty", target),
                None,
            ]
        )
        sleep = mock.Mock()

        with mock.patch.object(RUN_STATA_CI.shutil, "rmtree", rmtree), mock.patch.object(
            RUN_STATA_CI.time, "sleep", sleep
        ):
            RUN_STATA_CI.remove_tree_with_retries(
                target, attempts=4, delay_seconds=0.25
            )

        self.assertEqual(rmtree.call_args_list, [mock.call(target)] * 3)
        self.assertEqual(sleep.call_args_list, [mock.call(0.25), mock.call(0.5)])

    def test_propagates_last_enotempty(self) -> None:
        error = OSError(errno.ENOTEMPTY, "Directory not empty")
        rmtree = mock.Mock(side_effect=error)

        with mock.patch.object(RUN_STATA_CI.shutil, "rmtree", rmtree), mock.patch.object(
            RUN_STATA_CI.time, "sleep"
        ):
            with self.assertRaises(OSError) as caught:
                RUN_STATA_CI.remove_tree_with_retries(
                    Path("/private/tmp/test"), attempts=2
                )

        self.assertIs(caught.exception, error)
        self.assertEqual(rmtree.call_count, 2)

    def test_does_not_retry_other_errors(self) -> None:
        error = PermissionError(errno.EACCES, "Permission denied")
        rmtree = mock.Mock(side_effect=error)

        with mock.patch.object(RUN_STATA_CI.shutil, "rmtree", rmtree):
            with self.assertRaises(PermissionError):
                RUN_STATA_CI.remove_tree_with_retries(Path("/private/tmp/test"))

        rmtree.assert_called_once()

    def test_accepts_already_absent_path(self) -> None:
        rmtree = mock.Mock(side_effect=FileNotFoundError("already absent"))

        with mock.patch.object(RUN_STATA_CI.shutil, "rmtree", rmtree):
            RUN_STATA_CI.remove_tree_with_retries(
                Path("/private/tmp/already-absent")
            )

        rmtree.assert_called_once()

    def test_rejects_nonpositive_attempt_count(self) -> None:
        with self.assertRaisesRegex(ValueError, "attempts must be positive"):
            RUN_STATA_CI.remove_tree_with_retries(
                Path("/private/tmp/test"), attempts=0
            )


if __name__ == "__main__":
    unittest.main()
