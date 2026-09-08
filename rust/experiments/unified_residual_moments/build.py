"""Build paired public-ABI adapters without changing frozen outcome generators."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LEGACY = HERE.parent / "individual_inference"
ARMS = ("baseline", "unified")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    with Path(path).open("x") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)


def replace_once(code, before, after):
    if code.count(before) != 1:
        raise ValueError(f"adapter source anchor changed: {before}")
    return code.replace(before, after)


def adapt(code, arm):
    if arm not in ARMS:
        raise ValueError("unknown comparison arm")
    code = replace_once(code, "vckss_rust_component_inference_interface_version(), 2",
                        "vckss_rust_component_inference_interface_version(), 3")
    # Both arms use the current q0 iteration repair, not the historical budget.
    code = replace_once(code, "if observation{512}else{128}",
                        "if observation || !q1 {512}else{128}")
    if arm == "unified":
        for name in ("component", "match_component"):
            before = f"vckss_rust_engine_augment_{name}_inference_interrupt_v2"
            code = replace_once(code, before, before[:-1] + "3")
    code = replace_once(
        code, r'\"gram_rcond\":{},\"floored\":{}',
        r'\"gram_rcond\":{},\"ordering\":{},\"gram_inverse_relres\":{},'
        r'\"fit_relres\":{},\"positivity_floor\":{},\"nonpositive\":{},\"floored\":{}')
    code = replace_once(
        code, "numbers(&[r.receipt.gram_rcond]),r.receipt.floored_predictions",
        "numbers(&[r.receipt.gram_rcond]),r.receipt.ordering,"
        "numbers(&[r.receipt.gram_inverse_relres]),numbers(&[r.receipt.variance_fit_relres]),"
        "numbers(&[r.receipt.positivity_floor]),r.receipt.nonpositive_predictions,"
        "r.receipt.floored_predictions")
    return code


def source_files():
    # Include templates (.rs.in), build scripts, manifests, locks and vendored
    # inputs; the predecessor's extension filter omitted the CMG source template.
    files = set()
    for folder in ("rust/crates", "rust/vendor/cmg", "rust/experiments/individual_inference",
                   "rust/experiments/unified_residual_moments"):
        files.update(p for p in (ROOT / folder).rglob("*") if p.is_file()
                     and "target" not in p.relative_to(ROOT).parts
                     and "__pycache__" not in p.parts)
    files.update(ROOT / p for p in ("rust/Cargo.toml", "rust/Cargo.lock", "rust/rust-toolchain.toml")
                 if (ROOT / p).is_file())
    for folder in (ROOT / ".cargo", ROOT / "rust/.cargo"):
        if folder.is_dir():
            files.update(p for p in folder.rglob("*") if p.is_file())
    return {str(p.relative_to(ROOT)): sha(p) for p in sorted(files)}


def build(output):
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    sources = source_files()
    if "rust/crates/vckss-core/src/cmg_impl.rs.in" not in sources:
        raise ValueError("missing build-input template")
    spec = importlib.util.spec_from_file_location("frozen_individual_builder", LEGACY / "build.py")
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    receipts = {}
    for arm in ARMS:
        adapter = output / arm / "adapter"
        adapter.mkdir(parents=True)
        for name in ("observation.rs", "match.rs", "public_api.rs"):
            code = (LEGACY / name).read_text()
            if name == "public_api.rs":
                code = adapt(code, arm)
            with (adapter / name).open("x") as stream:
                stream.write(code)
        builder.HERE = adapter
        binaries = builder.build(output / arm / "bin")
        receipts[arm] = {
            "receipt_sha256": sha(binaries / "receipt.json"),
            "binaries": {family: sha(binaries / family) for family in builder.FROZEN},
            "adapters": {p.name: sha(p) for p in sorted(adapter.iterdir())},
        }
    if source_files() != sources:
        raise ValueError("source changed during paired build")
    receipt = {"schema": "unified-paired-build-v1", "status": "BUILD_PASS",
               "sources": sources, "arms": receipts,
               "rustc": subprocess.check_output([str(builder.TOOLCHAIN / "rustc"), "-vV"], text=True),
               "frozen_dgps": builder.FROZEN}
    write_json(output / "receipt.json", receipt)
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(json.dumps(build(args.output)["arms"], indent=2))
