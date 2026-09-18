"""Stage an isolated development package; never installs into Stata PLUS."""
import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def stage(root, output, plugin=None):
    if output.exists():
        raise ValueError('preserve previous attempts: output already exists')
    plugin_override = plugin is not None
    plugin = plugin or root / 'rust/stata_backend/target/aarch64-apple-darwin/release/libvckss_stata.dylib'
    if not plugin.is_file():
        raise ValueError('build the pinned source-local arm64 plugin first')
    package = output / 'package'
    package.mkdir(parents=True)
    for source in sorted((root / 'fevc').iterdir()):
        if source.is_file() and source.suffix in ('.ado', '.mata', '.sthlp', '.pkg', '.toc'):
            shutil.copy2(source, package / source.name)
    binary = package / 'fevc_rust_macos_arm64.plugin'
    shutil.copy2(plugin, binary)
    subprocess.run(['codesign', '--force', '--sign', '-', str(binary)], check=True)
    records = {p.name: sha(p) for p in sorted(package.iterdir()) if p.is_file()}
    receipt = dict(schema='FEVC-PARITY-LOCAL-DEVELOPMENT-PACKAGE-V1',
                   qualified=False, installed_plus_changed=False,
                   plugin_override=plugin_override,
                   unsigned_build_sha256=sha(plugin), files=records)
    (output / 'package.json').write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    print(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--plugin', type=Path,
                        help='Explicit artifact for isolated old-runtime rejection tests; not qualification')
    args = parser.parse_args()
    stage(args.root.resolve(), args.output.resolve(),
          args.plugin.resolve() if args.plugin else None)
