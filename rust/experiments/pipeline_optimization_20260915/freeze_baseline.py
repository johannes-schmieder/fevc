#!/usr/bin/env python3
"""Freeze public source and the accepted Linux runtime, never private inputs."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[3]
ACCEPTED = REPO.parent / 'fevc-paper/replication/results/raw/light_refresh_20260915'
DEST = REPO / '.local/pipeline-optimization-20260915/baseline'
NATIVE_MANIFEST = '6d8e86aafbe1224e29217ce4565a7c91c22fcc60583bd97b2805b910298ae18d'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    manifest = ACCEPTED / 'native-source-manifest.sha256'
    assert sha(manifest) == NATIVE_MANIFEST
    for line in manifest.read_text().splitlines():
        digest, name = line.split(None, 1)
        assert sha(REPO / name) == digest, f'Current source differs: {name}'
    names = subprocess.check_output(
        ['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z', '--',
         'rust', 'fevc', 'ci', 'tests', 'Cargo.toml', 'rust-toolchain.toml',
         'pyproject.toml', 'pytest.ini', 'AGENTS.md'], cwd=REPO,
    ).decode().split('\0')
    extensions = {'.rs', '.toml', '.lock', '.c', '.h', '.py', '.do', '.ado',
                  '.mata', '.sh', '.sge', '.json', '.txt', '.md', '.yml',
                  '.yaml', '.sthlp', '.pkg', '.ini', '.bib'}
    source = {}
    for name in sorted(set(filter(None, names))):
        path = REPO / name
        if path.suffix not in extensions or not path.is_file():
            continue
        assert not path.is_symlink(), f'Symlink requires explicit review: {name}'
        assert path.stat().st_size < 10_000_000, f'Oversize source: {name}'
        source[name] = sha(path)
    assert source and not DEST.exists()
    DEST.mkdir(parents=True)
    for name, digest in source.items():
        target = DEST / 'source' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO / name, target)
        assert sha(target) == digest
    shutil.copytree(ACCEPTED / 'package', DEST / 'accepted-linux-package')
    package = {p.name: sha(p) for p in sorted((DEST / 'accepted-linux-package').iterdir()) if p.is_file()}
    shutil.copy2(manifest, DEST / manifest.name)
    receipt = dict(native_manifest_sha256=NATIVE_MANIFEST, source=source,
                   linux_package=package, mac_binary='Build from frozen source; do not assume staged plugins match.',
                   git_head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip())
    output = DEST / 'snapshot.json'
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(files=len(source), snapshot=str(output), sha256=sha(output))))


if __name__ == '__main__':
    main()
