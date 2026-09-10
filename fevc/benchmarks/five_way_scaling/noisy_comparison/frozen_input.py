"""Serve hash-verified frozen inputs through the established wrapper interface."""
import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--rows", type=int)
parser.add_argument("--input", type=Path)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--receipt", type=Path)
args = parser.parse_args()
inputs = Path(os.environ["FW_RUN_DIR"])/"input"
receipt = json.loads((inputs/"fixture.json").read_text())
source = inputs/"fixture.csv"
assert hashlib.sha256(source.read_bytes()).hexdigest() == receipt["sha256"]
if args.input:
    assert hashlib.sha256(args.input.read_bytes()).hexdigest() == receipt["sha256"]
    shutil.copyfile(inputs/"oracle.json", args.output)
else:
    assert args.rows == receipt["rows"] == 3840 and args.receipt is not None
    shutil.copyfile(source, args.output)
    shutil.copyfile(inputs/"fixture.json", args.receipt)
print("FEVC_NOISY_FROZEN_INPUT_PASS")
