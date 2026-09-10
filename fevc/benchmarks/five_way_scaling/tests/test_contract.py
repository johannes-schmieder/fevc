from pathlib import Path

from fevc.benchmarks.five_way_scaling.build_manifest import build_rows, latin_order
from fevc.benchmarks.five_way_scaling.common import (
    ROLES, exact_tolerance, normalization_factor, normalized_targets,
)
from fevc.benchmarks.five_way_scaling.generate_input import generate
from fevc.benchmarks.five_way_scaling.exact_oracle import oracle
import csv


def test_confirmation_grid_and_latin_order() -> None:
    rows = build_rows("confirmation")
    assert len(rows) == 45
    assert len({row["cell_id"] for row in rows}) == 9
    assert len({(row["cell_id"], row["repeat"]) for row in rows}) == 45
    assert {tuple(row["role_order"].split(",")) for row in rows} == {
        latin_order(repeat) for repeat in range(1, 6)
    }
    assert all(set(row["role_order"].split(",")) == set(ROLES) for row in rows)


def test_registered_normalization_and_identity() -> None:
    assert normalization_factor("fevc", 10) == 1
    assert normalization_factor("pytwoway", 10) == 1
    assert normalization_factor("matlab", 10) == 0.9
    raw = {"worker": 2.0, "firm": 3.0, "covariance": -0.5, "total": 4.0}
    value = normalized_targets("r", 10, raw)
    assert value == {"worker": 1.8, "firm": 2.7, "covariance": -0.45, "total": 3.6}


def test_exact_tolerance() -> None:
    assert exact_tolerance(0.0) == 1e-5
    assert exact_tolerance(1_000.0) == 0.01


def test_generator_is_deterministic_and_unique(tmp_path: Path) -> None:
    first = generate(tmp_path / "a.csv", tmp_path / "a.json", 960)
    second = generate(tmp_path / "b.csv", tmp_path / "b.json", 960)
    assert first["sha256"] == second["sha256"]
    lines = (tmp_path / "a.csv").read_text(encoding="utf-8").splitlines()
    assert len(lines) == 961
    pairs = {(line.split(",")[1], line.split(",")[2]) for line in lines[1:]}
    assert len(pairs) == 960


def test_dense_oracle_matches_repository_independent_oracle(tmp_path: Path) -> None:
    from fevc.tests.python.oracle import exact_kss
    generate(tmp_path / "input.csv", tmp_path / "input.json", 960)
    with (tmp_path / "input.csv").open(encoding="utf-8", newline="") as handle:
        data = list(csv.DictReader(handle))
    result = exact_kss(
        [float(row["y"]) for row in data],
        [int(row["worker"]) for row in data],
        [int(row["firm"]) for row in data],
        deletion="match",
    )
    standalone = oracle(tmp_path / "input.csv")
    for index, target in enumerate(("worker", "firm", "covariance", "total")):
        assert abs(standalone["targets"][target] - result.corrected[index]) < 1e-10


def test_pinned_sanic_dependency_is_deployed_and_verified() -> None:
    root = Path(__file__).parents[1]
    expected = "998041c3303f63c3651070c49d9d991118223a8ba4d7e8199ea6c3ce3e38175e"
    deploy = (root / "deploy_scc.sh").read_text(encoding="utf-8")
    prepare = (root / "prepare.sge").read_text(encoding="utf-8")
    protocol = (root / "PROTOCOL.md").read_text(encoding="utf-8")
    assert "sanic_0.0.2.tar.gz" in deploy and "= 5" in deploy
    assert "sanic_0.0.2.tar.gz" in prepare
    assert 'packageVersion("sanic")' in prepare
    assert all(expected in text for text in (deploy, prepare, protocol))


def test_pytwoway_python_abi_and_solver_api_are_pinned() -> None:
    root = Path(__file__).parents[1]
    prepare = (root / "prepare.sge").read_text(encoding="utf-8")
    assert "numpy=1.26.4 pandas=2.0.3 scipy=1.13.1" in prepare
    for package, version in (("numpy", "1.26.4"), ("pandas", "2.0.3"),
                             ("scipy", "1.13.1")):
        assert f'version("{package}")=="{version}"' in prepare


def test_julia_preparation_imports_without_redundant_precompile() -> None:
    root = Path(__file__).parents[1]
    prepare = (root / "prepare.sge").read_text(encoding="utf-8")
    assert '@assert VERSION==v"1.9.1"' in prepare
    assert "Pkg.instantiate()" in prepare
    assert "Pkg.precompile()" not in prepare
    assert 'pkgversion(VarianceComponentsHDFE)==v"0.1.0"' in prepare
    assert "FEVC_FIVE_WAY_JULIA_PREP_PASS" in prepare


def test_smoke_adapter_repairs_are_explicit() -> None:
    root = Path(__file__).parents[1]
    matlab = (root / "matlab_run.m").read_text(encoding="utf-8")
    r_driver = (root / "r_run.R").read_text(encoding="utf-8")
    python = (root / "pytwoway_run.py").read_text(encoding="utf-8")
    assert "size(unique([worker firm],'rows'),1)==n" in matlab
    assert 'environmentName(imports),"imports:LeaveOutKSS"' in r_driver
    assert 'assign("detectCores",function(...) cores+1L,envir=imports)' in r_driver
    assert 'params["solver_tol"] != 1e-10' in python
    assert '"solver_tol":1e-10' not in python
    assert '"preconditioner":"jacobi"' in python
