from pathlib import Path
import argparse,hashlib,json,subprocess,shutil
p=argparse.ArgumentParser()
for name in ['source','plugin','output','stata','arch','label']:p.add_argument('--'+name,required=True)
p.add_argument('--processors',type=int,default=2)
a=p.parse_args()
source=Path(a.source); plugin=Path(a.plugin); out=Path(a.output)
out.mkdir(parents=True,exist_ok=False)
package=out/'package';package.mkdir()
manifest=(source/'fevc/fevc.pkg').read_text()
for line in manifest.splitlines():
 if line.startswith('f '):
  name=line[2:].strip()
  if '/' in name or '\\' in name:raise ValueError('unexpected manifest path')
  shutil.copy2(source/'fevc'/name,package/name)
for name in ['fevc.pkg','stata.toc']:shutil.copy2(source/'fevc'/name,package/name)
target={'arm64':'fevc_rust_macos_arm64.plugin','x86_64':'fevc_rust_macos_x86_64.plugin','linux':'fevc_rust_linux_x64.plugin'}[a.arch]
shutil.copy2(plugin,package/target)
(package/'fevc.pkg').chmod((package/'fevc.pkg').stat().st_mode | 0o200)
(package/'fevc.pkg').write_text(manifest+'f '+target+'\n')
digest=hashlib.sha256(plugin.read_bytes()).hexdigest()
probe=out/'test_centering_capability.do'
probe.write_text('version 18\nclear all\nset more off\nadopath ++ "'+str(package)+'"\nquietly fevc_rust probe\nassert r(centering_api)==1\nassert r(numerical_api)==2\ndi "CENTERING_CAPABILITY_PASS"\n')
tests=[(probe,'CENTERING_CAPABILITY_PASS')]
for name,marker in [('mean','SIMPLE_MEAN_PASS'),('exact','SIMPLE_EXACT_ORACLE_PASS'),('jla','SIMPLE_CORRECTED_MCSE_PASS'),('options','SIMPLE_CENTERING_OPTIONS_PASS')]:
 tests.append((source/'fevc/tests/stata'/('test_centering_'+name+'.do'),marker))
results=[]
for do,marker in tests:
 run=out/do.stem;run.mkdir()
 driver=run/'driver.do'
 driver.write_text('version 18\nset processors '+str(a.processors)+'\ndo "'+str(do)+'" "'+str(package)+'" rust\n')
 cmd=([ '/usr/bin/arch','-'+a.arch] if a.arch!='linux' else [])+[a.stata,'-b','do',str(driver)]
 console=run/'console.raw'
 with console.open('wb') as handle:r=subprocess.run(cmd,cwd=run,stdout=handle,stderr=subprocess.STDOUT)
 logs=list(run.glob('*.log'))
 if len(logs)>1:raise ValueError('unexpected log inventory')
 log=logs[0] if logs else run/(do.stem+'.log')
 raw=log.read_text(errors='replace') if log.exists() else ''
 started=False;clean=[]
 for line in raw.splitlines():
  if line.startswith('. '):started=True
  if started and 'Licensed to:' not in line and 'Serial number:' not in line:clean.append(line)
 sanitized=run/'stata.sanitized.log';sanitized.write_text('\n'.join(clean)+'\n')
 if log.exists():log.unlink()
 console.unlink()
 passed=r.returncode==0 and any(line.startswith(marker) for line in clean)
 results.append(dict(test=do.name,marker=marker,passed=passed,returncode=r.returncode,log_sha256=hashlib.sha256(sanitized.read_bytes()).hexdigest()))
 print(a.label,do.name,'PASS' if passed else 'FAIL',flush=True)
 if not passed:
  (out/'receipt.json').write_text(json.dumps(dict(schema='FEVC-CENTERING-CANDIDATE-CHECKS-V1',status='FAIL',label=a.label,arch=a.arch,plugin_sha256=digest,tests=results),indent=2)+'\n')
  raise SystemExit(1)
assert hashlib.sha256(plugin.read_bytes()).hexdigest()==digest
assert hashlib.sha256((package/target).read_bytes()).hexdigest()==digest
(out/'receipt.json').write_text(json.dumps(dict(schema='FEVC-CENTERING-CANDIDATE-CHECKS-V1',status='PASS',label=a.label,arch=a.arch,plugin_sha256=digest,tests=results),indent=2)+'\n')
print('CENTERING_CANDIDATE_PASS',a.label,digest,flush=True)
