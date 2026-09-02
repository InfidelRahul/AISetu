#!/usr/bin/env bash
set -euo pipefail

# Cross-platform release builder for AISetu.
# Produces binaries under dist/ for each target that the current host can build.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DIST="${ROOT}/dist"
mkdir -p "${DIST}"

TARGETS=(
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
  "x86_64-pc-windows-gnu"
  "aarch64-apple-darwin"
  "x86_64-apple-darwin"
  "aarch64-pc-windows-msvc"
  "x86_64-pc-windows-msvc"
)

HOST="$(rustc -vV | awk '/host:/ {print $2}')"
echo "host: ${HOST}"

built=0
for target in "${TARGETS[@]}"; do
  if rustup target list --installed | grep -qx "${target}" || [ "${target}" = "${HOST}" ]; then
    echo "building ${target}"
    cargo build --release --target "${target}" -p aisetu
    out_dir="${DIST}/${target}"
    mkdir -p "${out_dir}"
    if [ -f "target/${target}/release/aisetu.exe" ]; then
      cp "target/${target}/release/aisetu.exe" "${out_dir}/"
    else
      cp "target/${target}/release/aisetu" "${out_dir}/"
    fi
    built=$((built + 1))
  else
    echo "skip ${target} (toolchain not installed)"
  fi
done

echo "built ${built} target(s) into ${DIST}"
