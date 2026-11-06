#!/usr/bin/env python3
"""Verify the unchanged CMG payload and all active FEVC source identities."""
from pathlib import Path
import json
import re
import tomllib

import cmg_vendor

ROOT = Path(__file__).resolve().parents[2]


def check(root: Path = ROOT) -> None:
    record = json.loads((root / 'rust/vendor/CMG-SOURCE.json').read_text())
    revision = record['revision']
    if record['repository'] != cmg_vendor.REPOSITORY or not re.fullmatch('[0-9a-f]{40}', revision):
        raise ValueError('invalid CMG provenance identity')
    if cmg_vendor.inventory(root / 'rust/vendor/cmg') != record['files']:
        raise ValueError('CMG vendor payload differs from its pinned upstream inventory')
    for relative in ('rust/crates/vckss-core/src/full_cmg.rs',
                     'rust/stata_backend/cshim/stata_entry.c',
                     'fevc/fevc.ado', 'fevc/fevc_rust.ado'):
        text = (root / relative).read_text()
        if revision not in text:
            raise ValueError('CMG source identity mismatch: ' + relative)
    vendor = tomllib.loads((root / 'rust/vendor/cmg/Cargo.toml').read_text())
    workspace = tomllib.loads((root / 'rust/Cargo.toml').read_text())['workspace']
    if 'vendor/cmg' in workspace['members'] or 'vendor/cmg' not in workspace['exclude']:
        raise ValueError('upstream CMG must be an excluded standalone dependency')
    for lockfile in ('rust/Cargo.lock', 'rust/stata_backend/Cargo.lock'):
        rows = tomllib.loads((root / lockfile).read_text())['package']
        rows = [row for row in rows if row['name'] == 'cmg']
        if len(rows) != 1 or rows[0]['version'] != vendor['package']['version']:
            raise ValueError('CMG lockfile mismatch: ' + lockfile)


if __name__ == '__main__':
    check()
    print('CMG_VENDOR_CHECK=PASS')
