"""Build the immutable runtime/task snapshot; never copies a dirty fesim tree."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tarfile

from run import DGP, cells, digest

p = argparse.ArgumentParser()
p.add_argument("root", type=Path)
a = p.parse_args()
root = a.root.resolve()
root.mkdir(parents=True, exist_ok=False)
repo = Path(__file__).resolve().parents[3]
fesim_repo = Path("/Users/johannes/Git/fesim")
frozen = root / "input" / "fesim"
frozen.mkdir(parents=True)
archive = root / "fesim-source.tar"
subprocess.run(["git", "-C", str(fesim_repo), "archive", "HEAD", "-o", str(archive)], check=True)
with tarfile.open(archive) as t:
    t.extractall(frozen, filter="data")
archive.unlink()
# Retain only installed package inputs and license for the remote experiment.
names = {line[2:].strip() for line in (frozen / "fesim.pkg").read_text().splitlines() if line.startswith("f ")}
names |= {"fesim.pkg", "stata.toc", "LICENSE"}
for f in frozen.rglob("*"):
    if f.is_file() and str(f.relative_to(frozen)) not in names:
        f.unlink()
package = root / "input" / "fevc"
package.mkdir()
names = [line[2:].strip() for line in (repo / "fevc/fevc.pkg").read_text().splitlines() if line.startswith("f ")]
names += ["fevc.pkg", "stata.toc", "fevc_rust_macos_arm64.plugin", "fevc_rust_linux_x64.plugin"]
for name in names:
    shutil.copy2(repo / "fevc" / name, package / name)
(root / "harness").mkdir()
for name in ("run.py", "run.sge"):
    shutil.copy2(Path(__file__).parent / name, root / "harness" / name)
(root / "logs").mkdir()
manifest = dict(schema="FEVC_CENTERING_MC_V1", purpose="exploratory development analysis; no qualification claim",
    fevc_commit=subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip(),
    fesim_commit=subprocess.check_output(["git", "-C", str(fesim_repo), "rev-parse", "HEAD"], text=True).strip(),
    dgp=DGP, noise="Independent Gaussian, sigma=1.5; network and true effects fixed; signal=lnwage_true-epsilon_true; outcome=signal+noise",
    targets=["worker variance", "firm variance", "worker-firm covariance", "variance of sum"],
    target_denominator="N retained observations, not N-1", deletion="match", stayers="both",
    expected=dict(retained=573, workers=99, firms=15),
    estimator=dict(backend="rust", rng="counter_v1", tolerance=1e-10, nativethreads=1, mcse="all"),
    gates=dict(fit_success_rate=1.0, inventory="complete, unique, semantic seeds", scientific="positive true covariance; plugin bias measured independently", calibration="descriptive; no posthoc acceptance threshold"),
    rng="Outcome mt64 seeded by semantic SHA256 keys; JLA Counter-V1 separate semantic keys; same probes across centerings; target parity folds built into estimator",
    exclusions="No target excluded. Nonemployment and fevc graph/deletion pruning recorded. No sampling inference intervals.",
    profiles={p: cells(p) for p in ("smoke", "main")},
    local_validation_plugin_sha256=digest(package / "fevc_rust_macos_arm64.plugin"),
    files={str(f.relative_to(root)): digest(f) for f in root.rglob("*")
           if f.is_file() and f.name != "fevc_rust_macos_arm64.plugin"})
(root / "manifest.json").write_text(json.dumps(manifest, indent=2)+"\n")
print(root)
print(digest(root / "manifest.json"))
