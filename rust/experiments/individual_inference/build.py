"""Build public-ABI adapters beside byte-verified, unchanged DGP sources."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
TOOLCHAIN=Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin')
FROZEN={
    'observation':('rc_observation_inference','421ac6ef6307556613a830604c7b59c448999dfda14df8abb2921b735f3d0809'),
    'match_q0':('rc_match_q0','1f811ff00683f292448141f00398270dc7ea84f27c42cd6af7fc4e0f2d2e2193'),
    'match_q1':('inference_repair_match','3a2bcf597a7e8456d139ea30da1cfd9766c61ab611969bfa3f22eac4613ab5f1'),
}
def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def build(out):
    out=Path(out).resolve();out.mkdir(parents=True,exist_ok=False)
    sources={str(p.relative_to(ROOT)):sha(p)for directory in ('rust/crates','rust/vendor/cmg')for p in (ROOT/directory).rglob('*')if p.is_file()and p.suffix in('.rs','.toml')}
    sources.update({p:sha(ROOT/p)for p in ('rust/Cargo.toml','rust/Cargo.lock')})
    env=dict(os.environ,PATH=f'{TOOLCHAIN}:{os.environ["PATH"]}')
    with (out/'build.log').open('x') as log:
        compiled=subprocess.run(['cargo','build','--release','--locked','--offline','--message-format=json','-p','vckss-plugin'],cwd=ROOT/'rust',env=env,stdout=subprocess.PIPE,stderr=log,text=True,check=True)
        log.write(compiled.stdout)
        deps=ROOT/'rust/target/release/deps'
        artifacts=[json.loads(line) for line in compiled.stdout.splitlines()]
        libraries={name:[Path(p) for a in artifacts if a.get('reason')=='compiler-artifact' and a['target']['name']==name for p in a['filenames'] if p.endswith('.rlib')] for name in ('vckss_core','vckss_plugin')}
        if any(len(v)!=1 for v in libraries.values()):raise ValueError('ambiguous compiled libraries')
        for family,(name,digest) in FROZEN.items():
            original=ROOT/f'rust/crates/vckss-core/examples/{name}.rs'
            if sha(original)!=digest:raise ValueError('frozen DGP source changed')
            code=original.read_text().replace('#[path = "common/q1_reference.rs"]',f'#[path = "{original.parent}/common/q1_reference.rs"]')
            if family=='observation':
                if code.count('fn main()')!=1:raise ValueError('observation entrypoint')
                code=code.replace('fn main()','fn historical_main()',1)+'\n'+(HERE/'observation.rs').read_text()
            else:
                anchor='    pub fn entry() {'
                if code.count(anchor)!=1:raise ValueError('match insertion anchor')
                insertion=f'const INDIVIDUAL_FAMILY: &str = "{family}";\n'+(HERE/'match.rs').read_text()
                code=code.replace(anchor,insertion+'\n'+anchor,1).replace('    campaign::entry();','    campaign::individual_entry();')
            code+=f'\n#[path = "{HERE}/public_api.rs"] mod public_api;\n'
            source=out/f'{family}.rs'
            with source.open('x') as stream:stream.write(code)
            command=[str(TOOLCHAIN/'rustc'),'--edition=2021','-O','-Awarnings',str(source),'-L',f'dependency={deps}','-o',str(out/family)]
            for lib,paths in libraries.items():command+=['--extern',f'{lib}={paths[0]}']
            subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True)
    with (out/'receipt.json').open('x') as stream:
        if any(sha(ROOT/p)!=h for p,h in sources.items()):raise ValueError('source changed during build')
        json.dump({'status':'build_pass','sources':sources,'binaries':{f:sha(out/f)for f in FROZEN},'generated_sources':{f:sha(out/f'{f}.rs')for f in FROZEN},'adapter_sources':{p.name:sha(p)for p in HERE.glob('*')if p.is_file()},'frozen_dgps':FROZEN},stream,indent=2)
    return out

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('output');args=parser.parse_args()
    print(build(args.output))
