"""Every matrix leg must select its compiler despite a directory-local pin."""
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[3]


def test_matrix_jobs_select_and_report_their_toolchain():
    for filename in ("rust-backend.yml", "rust-stata-backend.yml"):
        workflow = (ROOT / ".github/workflows" / filename).read_text()
        jobs = [job for job in re.split(r"(?m)^  [\w-]+:\n", workflow)
                if "matrix.toolchain" in job]
        assert jobs
        for job in jobs:
            assert "    env:\n      RUSTUP_TOOLCHAIN: ${{ matrix.toolchain }}\n" in job
            assert "rustc --version --verbose" in job
            assert "cargo --version" in job
            assert "cargo clippy" in job and "-D warnings" in job
