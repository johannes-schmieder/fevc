"""Build the internal native-inference adapter without editing frozen oracles."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import os
import subprocess

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('followup_build', HERE.parent/'residual_moment_followup/build.py')
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)

def build(directory, tests=False):
    directory = Path(directory).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    source = base.ROOT/'rust/crates/vckss-core/examples/rc_observation_inference.rs'
    raw = source.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == '421ac6ef6307556613a830604c7b59c448999dfda14df8abb2921b735f3d0809'
    text = raw.decode().replace('fn main()', 'fn legacy_main()', 1)
    text = text.replace('#[path = "common/q1_reference.rs"]', f'#[path = "{source.parent / "common/q1_reference.rs"}"]')
    adapter = (HERE.parent/'residual_moment_followup/adapter.rs').read_text()
    assert adapter.count('fn main()') == 1
    text += '\n' + adapter.replace('fn main()', 'fn followup_main()', 1)
    text += '\n' + (HERE/'adapter.rs').read_text()
    generated = directory/'integration.rs'
    generated.write_text(text)
    env = dict(os.environ, PATH=f'{base.TOOLCHAIN}:{os.environ["PATH"]}')
    subprocess.run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'vckss-core'], cwd=base.ROOT/'rust', env=env, check=True)
    libraries = list((base.ROOT/'rust/target/release/deps').glob('libvckss_core-*.rlib'))
    assert len(libraries) == 1
    executable = directory/('integration-tests' if tests else 'integration')
    subprocess.run([str(base.TOOLCHAIN/'rustc'), '--edition=2021', '-O', '-Awarnings', *(['--test'] if tests else []), str(generated), '--extern', f'vckss_core={libraries[0]}', '-L', f'dependency={base.ROOT}/rust/target/release/deps', '-o', str(executable)], check=True)
    return executable

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('directory')
    parser.add_argument('--tests', action='store_true')
    args = parser.parse_args()
    print(build(args.directory, args.tests))
