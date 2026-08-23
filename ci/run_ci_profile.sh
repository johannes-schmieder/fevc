#!/bin/bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
profile="${1:-smoke}"

case "$profile" in
  plugin-build|plugin-load|numerical-small|differential)
    exec "$script_dir/run_plugin_ci.sh" "$profile"
    ;;
  *)
    exec "$script_dir/run_stata_ci.sh" "$profile"
    ;;
esac
