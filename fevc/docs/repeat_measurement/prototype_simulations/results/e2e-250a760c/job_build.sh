#!/bin/bash -l
#$ -P welfgr
#$ -N fevc_e2e_build
#$ -pe omp 4
#$ -l h_rt=01:30:00
#$ -l mem_per_core=4G
#$ -j y
#$ -m n
# Build the Linux plugin from the source bundle of the branch under test, stage a copy of the package with it, and
# run a small end-to-end smoke through the Stata command (both deletion modes, automatic interval, a deliberate
# failure). Development build for an end-to-end check: not a qualification and not a release artifact.
set -euo pipefail
umask 077
BASE=${BASE:?BASE is required}
case "$BASE" in <scc-private>/20261111-se-longpanel/e2e-*) ;; *) echo "invalid base" >&2; exit 2 ;; esac
test -f "$BASE/source.tar.gz"; test -f "$BASE/source_commit.txt"; test -f "$BASE/stata-spi/stplugin.h"; test -f "$BASE/stata-spi/stplugin.c"
test ! -e "$BASE/pkg" || { echo "$BASE/pkg exists; refusing to overwrite" >&2; exit 2; }
echo "job ${JOB_ID:-} host ${HOSTNAME:-} $(date --iso-8601=seconds) commit $(cat "$BASE/source_commit.txt")"
sha256sum "$BASE/source.tar.gz"
module purge
module load stata-mp/19
toolchain=<scc-project>/toolchains/rustup/toolchains/1.85.1-x86_64-unknown-linux-gnu
stata=${SCC_STATA_MP_BIN:?the Stata module did not expose SCC_STATA_MP_BIN}/stata-mp
export PATH=$toolchain/bin:$SCC_STATA_MP_BIN:/usr/bin:/bin
export RUSTC=$toolchain/bin/rustc CARGO_HOME=<scc-project>/toolchains/cargo-home
work=$TMPDIR/fevc-e2e-build; mkdir -p "$work/source"; tar -xzf "$BASE/source.tar.gz" -C "$work/source"
# The plugin interface files must be the reviewed ones.
( cd "$BASE/stata-spi" && sha256sum stplugin.c stplugin.h ) > "$work/spi.sha256"
for f in stplugin.c stplugin.h; do
    want=$(awk -v f=$f '$2 == f {print $1}' "$work/source/rust/stata_backend/stata-spi.sha256"); have=$(awk -v f=$f '$2 == f {print $1}' "$work/spi.sha256")
    test -n "$want" && test "$want" = "$have" || { echo "$f does not match the tracked hash" >&2; exit 3; }
done
export VCKSS_STATA_SPI_DIR=$BASE/stata-spi CARGO_TARGET_DIR=$work/target
cargo build --manifest-path "$work/source/rust/stata_backend/Cargo.toml" --locked --offline --release 2>&1 | tail -5
test -f "$work/target/release/libvckss_stata.so"
mkdir -p "$work/pkg"; cp -R "$work/source/fevc" "$work/pkg/fevc"
install -m 0755 "$work/target/release/libvckss_stata.so" "$work/pkg/fevc/fevc_rust_linux_x64.plugin"
# Smoke through the real entry point before anything is kept.
export STATATMP=$TMPDIR/statatmp; mkdir -p "$STATATMP" "$work/smoke"; cd "$work/smoke"
FEVC_DEFAULT_INFERENCE=none "$stata" -b do "$BASE/code/smoke.do" "$work/pkg/fevc" 4
cat ./*.log > "$work/smoke_all.log"                      # the batch log's name differs between platforms
grep -q "^E2E SMOKE PASS" "$work/smoke_all.log" || { echo "smoke failed" >&2; tail -40 "$work/smoke_all.log" >&2; exit 4; }
grep -E "^SMOKE|^E2E" "$work/smoke_all.log" | tail -12
mv "$work/pkg" "$BASE/pkg"
{ echo "schema=FEVC-E2E-BUILD-V1"; echo "source_commit=$(cat "$BASE/source_commit.txt")"; echo "source_tar_sha256=$(sha256sum "$BASE/source.tar.gz" | awk '{print $1}')"
  echo "plugin_sha256=$(sha256sum "$BASE/pkg/fevc/fevc_rust_linux_x64.plugin" | awk '{print $1}')"; echo "rustc=$("$RUSTC" --version)"; echo "job_id=${JOB_ID:-}"; echo "host=${HOSTNAME:-}"
  echo "smoke=PASS"; echo "finished=$(date --iso-8601=seconds)"; } > "$BASE/build_receipt.txt.tmp"
mv "$BASE/build_receipt.txt.tmp" "$BASE/build_receipt.txt"
cat "$BASE/build_receipt.txt"
