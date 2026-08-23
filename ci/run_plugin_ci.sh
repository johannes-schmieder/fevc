#!/bin/bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
exec /usr/bin/python3 "$script_dir/run_plugin_ci.py" "${1:-plugin-load}"
