#!/usr/bin/env python3
"""Bind separate opt-in Stata transport checks to the qualified thin binaries."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('slice', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    evidence = args.slice
    native = dict(line.split('=', 1) for line in
                  (evidence / 'native-receipt-01.txt').read_text().splitlines() if '=' in line)
    assert native['arm64_test_status'] == 'PASS_NATIVE'
    assert native['x86_64_test_status'] == 'PASS_ROSETTA'
    test = Path(__file__).with_name('exact_transport_smoke.do')
    assert sha(test) == 'e399f2de971f7f6d26806600716c134eb8cce7737d260be562d181a04883d607'
    expected = {(d, n, p, t) for d in ['observation', 'match']
                for n in ['joint', 'fixedoffset'] for p in ['movers', 'both']
                for t in ['0', '1', '4', '7']}
    calls = []
    for arch, attempt, receipt_key in [
        ('arm64', 'exact-stata-arm64-06', 'artifact_arm64_sha256'),
        ('x86_64', 'exact-stata-rosetta-01', 'artifact_x86_64_sha256'),
    ]:
        binary = evidence / 'source/fevc' / f'fevc_rust_macos_{arch}.plugin'
        assert sha(binary) == native[receipt_key]
        log = evidence / attempt / 'fevc.log'
        text = log.read_text()
        joined = re.sub(r'\n> ?', '', text)
        actual = re.findall(r'^EXACT_SMOKE deletion=(\w+) nuisance=(\w+) population=(\w+) threads=(\d+)', joined, re.M)
        assert len(actual) == 32 and set(actual) == expected
        assert '\nPASS exact_transport_smoke.do\n' in text
        assert not re.search(r'^r\([1-9][0-9]*\);', text, re.M)
        calls.append(dict(architecture=arch, cases=32, binary_sha256=sha(binary),
                          log=str(log.resolve()), log_sha256=sha(log), status='PASS'))
    failures = []
    for attempt in range(1, 6):
        folder = evidence / f'exact-stata-arm64-{attempt:02d}'
        for log in folder.glob('*.log'):
            failures.append(dict(log=str(log.resolve()), log_sha256=sha(log),
                                 status='FAILED_DIAGNOSTIC_HARNESS'))
    args.output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(test, args.output / test.name)
    result = dict(schema='FEVC-EXACT-OPT-IN-TRANSPORT-DIAGNOSTIC-V1', status='PASS',
                  scope='local thin arm64 and Rosetta x86; not full public-command integration or performance',
                  source_snapshot_sha256=sha(evidence / 'snapshot.json'),
                  native_source_manifest_sha256=native['source_manifest_sha256'],
                  native_qualification_receipt_sha256=sha(evidence / 'native-receipt-01.txt'),
                  smoke_source_sha256=sha(test), calls=calls, preserved_failures=failures,
                  installed_plus_changed=False, ordinary_exact_entrypoints_serial=True,
                  measured_campaign_calls=0)
    (args.output / 'receipt.json').write_text(json.dumps(result, indent=2, sort_keys=True)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
