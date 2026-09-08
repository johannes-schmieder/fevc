"""Build the V4 engineering adapter with byte-verified historical generators."""
import argparse
import importlib.util
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    return value


PREVIOUS = module('completion_previous_builder', HERE.parent / 'unified_residual_moments/build.py')
LEGACY = module('completion_original_builder', HERE.parent / 'individual_inference/build.py')
sha = PREVIOUS.sha
write = PREVIOUS.write_json


def source_files():
    sources = PREVIOUS.source_files()
    for directory in (HERE, ROOT / 'rust/stata_backend'):
        for path in directory.rglob('*'):
            if path.is_file() and not {'target', '__pycache__', 'stata-spi'} & set(path.parts):
                sources[str(path.relative_to(ROOT))] = sha(path)
    for path in (ROOT / 'fevc').glob('*.ado'):
        sources[str(path.relative_to(ROOT))] = sha(path)
    return sources


def adapt(code):
    code = PREVIOUS.adapt(code, 'unified')
    code = PREVIOUS.replace_once(code, 'vckss_rust_component_inference_interface_version(), 3',
                                 'vckss_rust_component_inference_interface_version(), 4')
    for name in ('component', 'match_component'):
        code = PREVIOUS.replace_once(code,
            f'vckss_rust_engine_augment_{name}_inference_interrupt_v3(generation,&attachment)',
            f'vckss_rust_engine_augment_{name}_inference_interrupt_v4(generation,&attachment,2048)')
    return code


def build(output):
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    sources = source_files()
    adapter = output / 'adapter'
    adapter.mkdir()
    for name in ('observation.rs', 'match.rs', 'public_api.rs'):
        code = (PREVIOUS.LEGACY / name).read_text()
        (adapter / name).write_text(adapt(code) if name == 'public_api.rs' else code)
    LEGACY.HERE = adapter
    LEGACY.build(output / 'bin')
    if source_files() != sources:
        raise ValueError('source changed during adapter build')
    receipt = dict(schema='FEVC-INFERENCE-COMPLETION-BUILD-V1', status='BUILD_PASS',
                   sources=sources, binaries={f: sha(output / 'bin' / f) for f in LEGACY.FROZEN},
                   adapter_receipt_sha256=sha(output / 'bin/receipt.json'),
                   frozen_dgps=LEGACY.FROZEN, gram_probes=2048)
    write(output / 'receipt.json', receipt)
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    print(json.dumps(build(args.output), indent=2))
