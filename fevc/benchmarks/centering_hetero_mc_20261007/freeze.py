"""Freeze exact current installed sources, including declared dirty fesim inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from run import CASES, DGP, EXPECTED, cells, digest, keys, validate_cases


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def source_identity(repo):
    # HEAD records ancestry; the file manifest, rather than HEAD, identifies this
    # runtime because the owner explicitly requested the current fesim changes.
    return dict(head=git(repo, "rev-parse", "HEAD"),
                status_porcelain=git(repo, "status", "--porcelain=v1", "--untracked-files=all").splitlines(),
                tracked_diff_sha256=hashlib.sha256(
                    subprocess.check_output(["git", "-C", str(repo), "diff", "HEAD", "--binary"])).hexdigest())


def package_files(repo, package_name):
    names = {line[2:].strip() for line in (repo / f"{package_name}.pkg").read_text().splitlines()
             if line.startswith("f ")}
    names.update((f"{package_name}.pkg", "stata.toc"))
    for name in names:
        p = Path(name)
        if p.is_absolute() or ".." in p.parts or not (repo / p).is_file():
            raise ValueError(f"invalid or missing package input {name}")
    return names


def main():
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--fesim-repo", type=Path, default=Path("/Users/johannes/Git/fesim"))
    p.add_argument("--cases", type=Path, help="JSON list of variance cases, frozen before the run")
    a = p.parse_args()
    cases = json.loads(a.cases.read_text()) if a.cases else CASES
    validate_cases(cases)
    root = a.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    repo = Path(__file__).resolve().parents[3]
    fesim_repo = a.fesim_repo.resolve()
    identities = dict(fevc=source_identity(repo), fesim=source_identity(fesim_repo))
    for name, source, names in (
        ("fesim", fesim_repo, package_files(fesim_repo, "fesim") | {"LICENSE"}),
        ("fevc", repo / "fevc", package_files(repo / "fevc", "fevc") |
         {"fevc_rust_macos_arm64.plugin", "fevc_rust_linux_x64.plugin"}),
    ):
        destination = root / "input" / name
        for rel in sorted(names):
            if name == "fesim" and Path(rel).suffix in (".mlib", ".mo", ".plugin"):
                raise ValueError("fesim snapshot must use source runtime, not compiled libraries")
            target = destination / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source / rel, target)
    (root / "harness").mkdir()
    for name in ("run.py", "run.sge", "analyze.py", "exact_oracle.py"):
        shutil.copy2(Path(__file__).parent / name, root / "harness" / name)
    (root / "logs").mkdir()
    manifest = dict(
        schema="FEVC_CENTERING_HETERO_MC_V1",
        purpose="exploratory development analysis; no qualification claim",
        source_snapshot="Current installed package files; fesim includes pre-existing logprob work and new heteroskedasticity. Runtime file SHA256s identify the actual inputs; HEAD alone does not.",
        source_identities=identities,
        fevc_commit=identities["fevc"]["head"], fesim_commit=identities["fesim"]["head"],
        dgp=DGP, cases=cases,
        noise="Independent Gaussian y=conditional_mean_true+sqrt(sigma2_true)*z; same z across variance cases. sigma2_true normalized by fesim over all generated employed observations before filtering.",
        targets=["worker variance", "firm variance", "worker-firm covariance", "variance of sum"],
        target_denominator="N retained observations, not N-1", deletion="match", stayers="both",
        expected=EXPECTED,
        estimator=dict(backend="rust", rng="counter_v1", tolerance=1e-10, nativethreads=1, mcse="all"),
        gates=dict(fit_success_rate=1.0, inventory="complete, unique, semantic seeds, exact case and fixture metadata",
                   scientific="positive true covariance; all generated variances positive; mean generated variance 2.25; same design and mean truth across cases; dense heteroskedastic oracle before campaign",
                   calibration="descriptive; no posthoc acceptance threshold"),
        rng="Outcome mt64 and JLA Counter-V1 seeded by semantic SHA256 keys excluding case and centering. Frozen network seed 7102026. Same Gaussian draws and probes across cases/modes; process scheduling does not enter seeds.",
        exclusions="No target excluded. Nonemployment and fevc graph/deletion pruning recorded. No sampling inference intervals.",
        profiles={profile: cells(profile, cases) for profile in ("smoke", "main")},
        expected_results={profile: len(keys(cells(profile, cases))) for profile in ("smoke", "main")},
        output_inventory="output/{profile}/{case}/task-{shard:02d}/{analysis.do,driver.do,driver.log,console.txt,fixture.generated.csv,fixture.csv,results.csv,receipt.json}; task containing sampling rep1 also has oracle-y.csv. Number of nonempty shards per case is min(workers,number of units).",
        local_validation_plugin_sha256=digest(root / "input/fevc/fevc_rust_macos_arm64.plugin"),
        files={str(f.relative_to(root)): digest(f) for f in root.rglob("*")
               if f.is_file() and f.name != "fevc_rust_macos_arm64.plugin"},
    )
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2)+"\n")
    print(root)
    print(digest(root / "manifest.json"))


if __name__ == "__main__":
    main()
