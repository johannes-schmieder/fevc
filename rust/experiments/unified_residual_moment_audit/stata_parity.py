"""Public Stata parity on already captured, source-bound native saved draws."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
STATA = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
FAMILIES = ("observation", "match_q0", "match_q1")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)


def driver(fixture, row, family):
    q1 = family == "match_q1"
    deletion = "deletion(observation)" if family == "observation" else "deletion(match) deletionid(deletion) nuisance(fixedoffset)"
    weights = "" if family == "observation" else "[fw=frequency]"
    reference = "q1" if q1 else "highrank"
    lines = ["version 18.0", "clear all", "set more off", "set type double", "set processors 1",
             f'adopath ++ "{ROOT}/fevc"', f'quietly run "{ROOT}/fevc/fevc.ado"',
             f'import delimited using "{fixture}", clear asdouble',
             f'quietly fevc outcome {weights}, worker(worker) firm(firm) {deletion} stayers(movers) backend(rust) engine(generic) rng(counter_v1) algorithm(jla) preconditioner(diagonal) batch(16) targetweight(target) inferencemodel(structured_common) inference({reference})',
             'assert "`e(inference_variance_fit)\'"=="residual_moments"',
             "assert e(probes)==200", "assert e(inference_simulations)==1000", "assert e(inference_gram_probes)==512",
             f'assert e(inference_joint_status)=={row["joint"]}',
             f'assert e(inference_computed_targets)=={row["computed"]}',
             f'assert e(inference_independent_units)=={row["units"]}',
             f'assert e(inference_critical_draws)=={row["critical_draws"]}',
             f'assert e(inference_counter_atoms)=={row["counter_atoms"]}',
             f'assert e(inference_counter_words)=={row["counter_words"]}']

    def close(expression, value):
        if value is None:
            lines.append(f"assert missing({expression})")
        else:
            lines.append(f"assert abs({expression}-({value:.17e}))<=1e-7*max(1,abs({value:.17e}))")

    close("e(variance_gram_rcond)", row["gram_rcond"][0])
    close("e(variance_floor_share)", row["floored"]/row["units"])
    for i in range(4):
        close(f"e(kss)[1,{i+1}]", row["point"][i])
        close(f"e(q0_status)[{i+1},1]", row["targets"][2*i+1])
        if q1:
            for j in range(20):
                close(f"e(component_q1_diagnostics)[{i+1},{j+1}]", row["q1"][20*i+j])
        elif row["targets"][2*i+1] == 0:
            close(f"e(component_inference)[{i+1},2]^2", row["targets"][2*i])
    if q1 or row["joint"] != 0 or row["computed"] != 4:
        lines += ["local returned_matrices : e(matrices)", 'assert !strpos(" `returned_matrices\' "," V ")']
    else:
        for i, value in enumerate(row["covariance"]):
            close(f"e(V)[{i//4+1},{i%4+1}]", value)
    lines += ["local returned_matrices : e(matrices)", 'assert !strpos(" `returned_matrices\' "," structured_variance_cv ")',
              "quietly fevc_rust snapshot", "assert r(state)==0", 'display "UNIFIED SAVED NATIVE STATA PARITY PASS"', "exit, clear"]
    return "\n".join(lines)+"\n"


def check(pipeline, output):
    manifest = json.loads((pipeline / "manifest.json").read_text())
    result = json.loads((pipeline / "result.json").read_text())
    if result["status"] != "PIPELINE_PASS" or result["manifest_sha256"] != sha(pipeline / "manifest.json"):
        raise ValueError("pipeline identity")
    if any(sha(ROOT / p) != h for p, h in manifest["source_files"].items()):
        raise ValueError("tested source changed")
    output.mkdir(parents=True, exist_ok=False)
    plugin = ROOT / "fevc/fevc_rust_macos_arm64.plugin"
    frozen = {"pipeline_manifest_sha256": sha(pipeline / "manifest.json"),
              "pipeline_result_sha256": sha(pipeline / "result.json"), "plugin_sha256": sha(plugin),
              "script_sha256": sha(Path(__file__)), "new_outcome_draws": 0, "cases": {}}
    fixtures = {}
    for family in FAMILIES:
        source = pipeline / "pipeline-checks" / f"{family}-unified-0"
        receipt = json.loads((source / "receipt.json").read_text())
        if receipt["status"] != "VALIDATED" or receipt["output_sha256"] != sha(source / "rows.jsonl.gz"):
            raise ValueError("saved native receipt")
        records = [json.loads(line) for line in gzip.open(source / "rows.jsonl.gz", "rt")]
        if len(records) != 3 or records[2]["status"] != "success":
            raise ValueError("saved fixture inventory")
        fixtures[family] = source / "fixture.csv", records[2]
        frozen["cases"][family] = {"fixture_sha256": sha(source / "fixture.csv"),
                                  "native_sha256": sha(source / "rows.jsonl.gz"), "seed": records[2]["seed"]}
    write(output / "manifest.json", frozen)
    results = []
    for family, (fixture, row) in fixtures.items():
        folder = output / family
        folder.mkdir()
        (folder / "check.do").write_text(driver(fixture, row, family))
        completed = subprocess.run([str(STATA), "-b", "do", "check.do"], cwd=folder, timeout=180)
        log = folder / "check.log"
        passed = completed.returncode == 0 and log.exists() and "UNIFIED SAVED NATIVE STATA PARITY PASS\n" in log.read_text()
        results.append({"family": family, "status": "PASS" if passed else "FAIL", "returncode": completed.returncode,
                        "driver_sha256": sha(folder / "check.do"), "log_sha256": sha(log) if log.exists() else None})
        print(family, results[-1]["status"], flush=True)
    unchanged = sha(plugin) == frozen["plugin_sha256"] and all(sha(ROOT / p) == h for p, h in manifest["source_files"].items())
    receipt = {"status": "PASS" if unchanged and all(r["status"] == "PASS" for r in results) else "FAIL",
               "manifest_sha256": sha(output / "manifest.json"), "source_unchanged": unchanged,
               "tolerance": "1e-7 * max(1,abs(native)); status and counts exact", "results": results}
    write(output / "receipt.json", receipt)
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("pipeline", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    receipt = check(args.pipeline.resolve(), args.output.resolve())
    print(receipt["status"])
    raise SystemExit(0 if receipt["status"] == "PASS" else 1)
