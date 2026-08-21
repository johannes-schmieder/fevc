#!/usr/bin/env bash
set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
destination=${VCKSS_STATA_SPI_DIR:-"${script_dir}/stata-spi"}
base_url=https://www.stata.com/plugins
hash_manifest=${script_dir}/stata-spi.sha256

destination_parent=$(dirname -- "${destination}")
mkdir -p "${destination_parent}"
temporary=$(mktemp -d "${destination_parent}/stata-spi.tmp.XXXXXX")
trap 'rm -rf "${temporary}"' EXIT
curl -L --proto '=https' --proto-redir '=https' --fail --show-error --silent \
  "${base_url}/stplugin.c" -o "${temporary}/stplugin.c"
curl -L --proto '=https' --proto-redir '=https' --fail --show-error --silent \
  "${base_url}/stplugin.h" -o "${temporary}/stplugin.h"

expected_c=$(awk '$2 == "stplugin.c" {print $1}' "${hash_manifest}")
expected_h=$(awk '$2 == "stplugin.h" {print $1}' "${hash_manifest}")
sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

actual_c=$(sha256_file "${temporary}/stplugin.c")
actual_h=$(sha256_file "${temporary}/stplugin.h")

if [[ "${actual_c}" != "${expected_c}" || "${actual_h}" != "${expected_h}" ]]; then
  echo "Downloaded Stata SPI files do not match the reviewed SPI 3.0 hashes." >&2
  echo "stplugin.c: ${actual_c}" >&2
  echo "stplugin.h: ${actual_h}" >&2
  exit 1
fi

mkdir -p "${destination}"
mv "${temporary}/stplugin.c" "${destination}/.stplugin.c.new"
mv "${temporary}/stplugin.h" "${destination}/.stplugin.h.new"
mv "${destination}/.stplugin.c.new" "${destination}/stplugin.c"
mv "${destination}/.stplugin.h.new" "${destination}/stplugin.h"

printf 'Stata SPI 3.0 ready in %s\n' "${destination}"
