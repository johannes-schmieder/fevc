#!/bin/bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
rustup_bin="${RUSTUP_BIN:-/opt/homebrew/bin/rustup}"
toolchain="${RUST_TOOLCHAIN:-1.81.0}"

if [[ ! -x "$rustup_bin" ]]; then
  echo "Rust quick check could not find rustup at $rustup_bin" >&2
  exit 127
fi

if ! "$rustup_bin" toolchain list | grep -Eq "^${toolchain}(-|[[:space:]])"; then
  echo "Required Rust toolchain $toolchain is not installed" >&2
  exit 127
fi

export CARGO_TERM_COLOR=always
export RUST_BACKTRACE=1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${RUNNER_TEMP:-/private/tmp}/varcomp-kss-rust-target}"
toolchain_root="$("$rustup_bin" run "$toolchain" rustc --print sysroot)"
export PATH="$toolchain_root/bin:$PATH"

cd "$repo_root/rust"
"$rustup_bin" run "$toolchain" cargo fmt --all -- --check
"$rustup_bin" run "$toolchain" cargo clippy --workspace --all-targets --locked -- -D warnings
"$rustup_bin" run "$toolchain" cargo test --workspace --all-targets --locked
