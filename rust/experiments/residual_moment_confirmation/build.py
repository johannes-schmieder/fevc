"""Build a confirmation-only entrypoint around unchanged native integration."""
from pathlib import Path
import argparse
import importlib.util
import subprocess

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('integration_build',HERE.parent/'residual_moment_integration/build.py')
base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)

def build(directory,tests=False):
    directory=Path(directory).resolve();directory.mkdir(parents=True,exist_ok=True)
    base.build(directory/'base')
    source=(directory/'base/integration.rs').read_text()
    assert source.count('fn main()')==1
    generated=directory/'confirmation.rs'
    generated.write_text(source.replace('fn main()','fn integration_main()',1)+'\n'+(HERE/'adapter.rs').read_text())
    libraries=list((base.base.ROOT/'rust/target/release/deps').glob('libvckss_core-*.rlib'));assert len(libraries)==1
    exe=directory/('confirmation-tests' if tests else 'confirmation')
    subprocess.run([str(base.base.TOOLCHAIN/'rustc'),'--edition=2021','-O','-Awarnings',*(['--test'] if tests else []),str(generated),'--extern',f'vckss_core={libraries[0]}','-L',f'dependency={libraries[0].parent}','-o',str(exe)],check=True)
    return exe

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory');p.add_argument('--tests',action='store_true');a=p.parse_args();print(build(a.directory,a.tests))
