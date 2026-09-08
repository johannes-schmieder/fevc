"""Build a research adapter around the immutable independent RC oracle.

The original example is not edited. Only its entrypoint and relative module
path are mechanically rebound in a generated build file, then the adapter is
appended in the same module so the independent private oracle remains intact.
"""
from pathlib import Path
import argparse
import hashlib
import os
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[3]
HOST = {'Darwin': 'apple-darwin', 'Linux': 'unknown-linux-gnu'}[platform.system()]
ARCH = {'arm64': 'aarch64', 'aarch64': 'aarch64', 'x86_64': 'x86_64'}[platform.machine()]
TOOLCHAIN = Path(os.environ.get('FEVC_RUST_TOOLCHAIN_BIN', Path.home()/f'.rustup/toolchains/1.85.1-{ARCH}-{HOST}/bin'))

def build(directory, tests=False):
    directory = Path(directory).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    source = ROOT / 'rust/crates/vckss-core/examples/rc_observation_inference.rs'
    raw = source.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == '421ac6ef6307556613a830604c7b59c448999dfda14df8abb2921b735f3d0809'
    text = raw.decode()
    assert text.count('fn main()') == 1
    text = text.replace('fn main()', 'fn legacy_main()', 1)
    old = '#[path = "common/q1_reference.rs"]'
    assert text.count(old) == 1
    text = text.replace(old, f'#[path = "{source.parent / "common/q1_reference.rs"}"]')
    text += '\n' + (Path(__file__).parent / 'adapter.rs').read_text()
    generated = directory / 'followup.rs'
    generated.write_text(text)
    env = dict(os.environ, PATH=f'{TOOLCHAIN}:{os.environ["PATH"]}')
    subprocess.run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'vckss-core'], cwd=ROOT/'rust', env=env, check=True)
    libraries = list((ROOT/'rust/target/release/deps').glob('libvckss_core-*.rlib'))
    assert len(libraries) == 1, libraries
    executable = directory/('followup-tests' if tests else 'followup')
    subprocess.run([str(TOOLCHAIN/'rustc'), '--edition=2021', '-O', '-Awarnings', *(['--test'] if tests else []), str(generated), '--extern', f'vckss_core={libraries[0]}', '-L', f'dependency={ROOT}/rust/target/release/deps', '-o', str(executable)], check=True)
    return executable

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('directory')
    parser.add_argument('--tests', action='store_true')
    args=parser.parse_args()
    print(build(args.directory,args.tests))
