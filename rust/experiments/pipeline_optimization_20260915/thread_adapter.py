#!/usr/bin/env python3
"""Build isolated all-path benchmark packages from hash-verified snapshots.

No production command reads this contract. The baseline's native executors
are not changed: in particular, its explicit and auto-exact paths stay serial.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil

SCHEMA = 'FEVC-PIPELINE-THREAD-ADAPTER-V1'
CONTRACT = 'FEVC-PIPELINE-THREADS-V1'
BASELINE_SNAPSHOT = '1d9968830ee27d5e801b7a4f27181a2fa240e1a9a84578fa2c8e53e464fb0a7e'
BASELINE_LINUX = '6937b0b1045383b2f1c6b7b7840d7f33f7f54b1dc580ba3f0a1c819b8e540a2a'

PUBLIC_CALL = '''program define fevc__rust_public_call, rclass
    version 18.0
    fevc_rust `0'
    return add
end
'''
TIMED_CALL = r'''program define fevc__rust_public_call, rclass
    version 18.0
    gettoken operation remainder : 0, parse(" ,")
    local clock = 0
    if "`operation'"=="solve" local clock = 81
    if "`operation'"=="prepare" local clock = 82
    if inlist("`operation'","result","componentresult","componentresultv5","projectionresult","stayerresult") local clock = 83
    if substr("`operation'",1,7)=="augment" local clock = 84
    if `clock' quietly timer on `clock'
    capture noisily fevc_rust `0'
    local rc = _rc
    return add
    if `clock' quietly timer off `clock'
    exit `rc'
end
'''

HELPER = r'''
program define _fevc_pipeline_threads
    version 18.0
    args destination
    local contract : environment PF_THREAD_CONTRACT
    local value : environment PF_NATIVE_THREADS
    local slots : environment PF_ASSIGNED_SLOTS
    local scheduler_slots : environment NSLOTS
    local t = real("`value'")
    local s = real("`slots'")
    if "`contract'"!="FEVC-PIPELINE-THREADS-V1" | missing(`t',`s') | ///
        `t'!=floor(`t') | `s'!=floor(`s') | !inrange(`t',1,64) | ///
        `s'<`t' | c(processors)!=min(4,`t') {
        di as error "invalid isolated native-thread contract"
        exit 198
    }
    if "`scheduler_slots'"!="" {
        if missing(real("`scheduler_slots'")) | real("`scheduler_slots'")!=`s' {
            di as error "native-thread allocation disagrees with scheduler"
            exit 198
        }
    }
    c_local `destination' "`t'"
end
'''


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def once(text: str, old: str, new: str) -> str:
    if text.count(old) != 1:
        raise ValueError(f'stale adapter anchor: {old!r}')
    return text.replace(old, new, 1)


def transform(ado: str, component: str, variant: str) -> tuple[str, str]:
    if variant not in {'baseline', 'candidate'}:
        raise ValueError('unknown variant')
    if '_fevc_pipeline_threads' in ado:
        raise ValueError('source is already adapted')
    # Validate the contract before any public setup/preparation, not only when
    # a particular engine happens to be selected.
    ado = once(ado, 'program define fevc, eclass\n    version 18.0\n',
        'program define fevc, eclass\n    version 18.0\n'
        '    _fevc_pipeline_threads pipeline_entry_threads\n')
    if variant == 'candidate':
        anchor = '    local native_threads = c(processors)\n'
        if ado.count(anchor) != 2:
            raise ValueError('stale candidate planned/legacy exact context anchors')
        ado = ado.replace(anchor, '    _fevc_pipeline_threads native_threads\n')
        if '`generic_work\'[1,5]==`native_threads\'' not in ado:
            raise ValueError('candidate generic receipt lost native context')
        if "`generic_work' `native_threads'" not in ado:
            raise ValueError('component batch caller lacks native context')
        if 'args handle covariance_probes gram_probes work native_threads' not in component:
            raise ValueError('component batch checker lacks native context')
    else:
        # Only thread-dispatch and native receipt anchors, never a global
        # c(processors) substitution or a change to Stata-facing metadata.
        ado = once(ado, '    local projection_requested =',
            '    _fevc_pipeline_threads native_threads\n    local projection_requested =')
        for old, new in (
            ("threads(`=c(processors)')", "threads(`native_threads')"),
            ("`cmg_threads_requested'==c(processors)", "`cmg_threads_requested'==`native_threads'"),
            ("`cmg_threads_used'==c(processors)", "`cmg_threads_used'==`native_threads'"),
            ("`generic_work'[1,5]==c(processors)", "`generic_work'[1,5]==`native_threads'"),
            ("`generic_work'[1,6]<=c(processors)", "`generic_work'[1,6]<=`native_threads'"),
            ("`inferencesimulations' `inferencegramprobes' `generic_work'\n",
             "`inferencesimulations' `inferencegramprobes' `generic_work' `native_threads'\n"),
        ):
            ado = once(ado, old, new)
        component = once(component, 'args handle covariance_probes gram_probes work\n',
            'args handle covariance_probes gram_probes work native_threads\n')
        component = once(component, '8*c(processors)', "8*`native_threads'")
        component = once(component, "`batch'[1,11]==c(processors)", "`batch'[1,11]==`native_threads'")
        if 'exactexecution(' in ado or 'exactlegacy(' in ado:
            raise ValueError('historical baseline unexpectedly contains parallel exact')
    return ado + HELPER, component


def build(snapshot: Path, expected_snapshot: str, plugin: Path,
          expected_plugin: str, output: Path, variant: str) -> dict:
    if output.exists() or sha(snapshot) != expected_snapshot:
        raise ValueError('existing output or source snapshot mismatch')
    if variant == 'baseline' and expected_snapshot != BASELINE_SNAPSHOT:
        raise ValueError('not the accepted paper baseline')
    if sha(plugin) != expected_plugin:
        raise ValueError('binary identity mismatch')
    if variant == 'baseline' and plugin.name.endswith('linux_x64.plugin') and expected_plugin != BASELINE_LINUX:
        raise ValueError('not the accepted baseline Linux binary')
    inventory = json.loads(snapshot.read_text())['source']
    root = snapshot.parent / 'source'
    if variant == 'baseline':
        manifest = snapshot.parent/'native-source-manifest.sha256'
        if sha(manifest)!='6d8e86aafbe1224e29217ce4565a7c91c22fcc60583bd97b2805b910298ae18d':
            raise ValueError('baseline native manifest mismatch')
        for line in manifest.read_text().splitlines():
            digest,name=line.split(None,1)
            if name in inventory and inventory[name]!=digest: raise ValueError('conflicting baseline input')
            inventory[name]=digest
    for name, digest in inventory.items():
        path = root / name
        if path.is_symlink() or not path.is_file() or sha(path) != digest:
            raise ValueError(f'source mismatch: {name}')
    ado, component = transform((root/'fevc/fevc.ado').read_text(),
        (root/'fevc/fevc__rust_comp_batch_receipt.ado').read_text(), variant)
    if (root/'fevc/fevc__rust_public_call.ado').read_text()!=PUBLIC_CALL:
        raise ValueError('stale public-call timer anchor')
    output.mkdir(parents=True)
    for name in inventory:
        path = Path(name)
        if path.parent == Path('fevc') and path.suffix in {'.ado','.mata','.sthlp','.pkg','.toc'}:
            # Deployed sources are read-only; edits belong only to fresh copies.
            shutil.copyfile(root/name, output/path.name)
    (output/'fevc.ado').write_text(ado)
    (output/'fevc__rust_comp_batch_receipt.ado').write_text(component)
    (output/'fevc__rust_public_call.ado').write_text(TIMED_CALL)
    shutil.copy2(plugin, output/plugin.name)
    receipt = dict(schema=SCHEMA, variant=variant, source_snapshot_sha256=expected_snapshot,
        plugin_sha256=expected_plugin, contract=CONTRACT, stata_processors='min(4,T)',
        native_threads='T bounded by allocation', exact_execution='serial historical' if variant=='baseline' else 'additive parallel',
        boundary_timers='81 solve; 82 native preparation; 83 result fetch; 84 native attachment preparation',
        adapter_sha256=sha(Path(__file__)),
        files={p.name:sha(p) for p in sorted(output.iterdir())})
    (output/'adapter.json').write_text(json.dumps(receipt, indent=2, sort_keys=True)+'\n')
    return receipt


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('snapshot', type=Path)
    p.add_argument('snapshot_sha256')
    p.add_argument('plugin', type=Path)
    p.add_argument('plugin_sha256')
    p.add_argument('output', type=Path)
    p.add_argument('variant', choices=('baseline','candidate'))
    a = p.parse_args()
    print(json.dumps(build(a.snapshot,a.snapshot_sha256,a.plugin,a.plugin_sha256,a.output,a.variant)))
