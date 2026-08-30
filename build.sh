#!/usr/bin/env bash
# AISetu — generate installable / runnable apps.
# Usage:
#   ./build.sh              # web + portable node app + current-OS desktop if tools exist
#   ./build.sh web
#   ./build.sh desktop
#   ./build.sh linux|win|mac
#   ./build.sh android
#   ./build.sh all

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

VERSION="$(node -p "require('./package.json').version" 2>/dev/null || echo 1.0.0)"
OUT="$ROOT/release"
WEB_DIST="$ROOT/app/dist"
PORTABLE="$OUT/aisetu-$VERSION-portable"

log() { printf '\n\033[1m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[skip]\033[0m %s\n' "$*"; }
die() { printf '\033[31m[fail]\033[0m %s\n' "$*" >&2; exit 1; }

need_node() {
  command -v node >/dev/null || die "Node.js is required"
  command -v npm >/dev/null || die "npm is required"
}

install_deps() {
  if [[ ! -d node_modules/vite || ! -d node_modules/react ]]; then
    log "Installing npm dependencies"
    npm install --no-fund --no-audit --legacy-peer-deps
  fi
}

build_web() {
  log "Building React Native web UI"
  npx vite build
  mkdir -p "$OUT/web"
  rm -rf "$OUT/web"/*
  cp -a "$WEB_DIST"/. "$OUT/web/"
  log "Web app → $OUT/web/"
}

build_portable() {
  log "Packaging portable Node app (Linux / macOS / Windows with Node)"
  rm -rf "$PORTABLE"
  mkdir -p "$PORTABLE/app/dist" "$PORTABLE/src" "$PORTABLE/desktop" "$PORTABLE/ui"
  cp -a "$WEB_DIST"/. "$PORTABLE/app/dist/"
  cp -a src/. "$PORTABLE/src/"
  cp -a desktop/. "$PORTABLE/desktop/" 2>/dev/null || true
  cp package.json README.md "$PORTABLE/"
  cat > "$PORTABLE/AISetu" << 'LAUNCH'
#!/usr/bin/env bash
DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"
export AISETU_HOST="${AISETU_HOST:-0.0.0.0}"
export AISETU_PORT="${AISETU_PORT:-8787}"
exec node src/index.js
LAUNCH
  chmod +x "$PORTABLE/AISetu"
  cat > "$PORTABLE/AISetu.cmd" << 'WIN'
@echo off
set AISETU_HOST=0.0.0.0
set AISETU_PORT=8787
cd /d "%~dp0"
node src\index.js
WIN
  cat > "$PORTABLE/AISetu.desktop" << DESK
[Desktop Entry]
Name=AISetu
Comment=Local gateway from IDE to web AI chat
Exec=$PORTABLE/AISetu
Path=$PORTABLE
Terminal=false
Type=Application
Categories=Development;
DESK
  chmod +x "$PORTABLE/AISetu.desktop"
  (cd "$OUT" && tar -czf "aisetu-$VERSION-portable.tar.gz" "aisetu-$VERSION-portable")
  log "Portable app → $PORTABLE"
  log "Archive      → $OUT/aisetu-$VERSION-portable.tar.gz"
}

build_electron() {
  local target="${1:-}"
  if ! command -v npx >/dev/null; then
    warn "npx missing; cannot run electron-builder"
    return 0
  fi
  if [[ ! -x "$ROOT/node_modules/.bin/electron" ]] && [[ ! -f "$ROOT/node_modules/electron/index.js" ]]; then
    log "Adding electron + electron-builder (optional desktop packages)"
    npm install --no-save --no-fund --no-audit --legacy-peer-deps electron@32.2.0 electron-builder@25.1.8 || {
      warn "Could not install electron-builder"
      return 0
    }
  fi
  case "$target" in
    win|windows) npx electron-builder --win --config electron-builder.yml || warn "Windows package failed (needs Windows or wine)" ;;
    mac|macos|darwin) npx electron-builder --mac --config electron-builder.yml || warn "macOS package failed (needs macOS)" ;;
    linux) npx electron-builder --linux --config electron-builder.yml || warn "Linux Electron package failed (binary download may be blocked)" ;;
    *)
      case "$(uname -s)" in
        Linux) npx electron-builder --linux --config electron-builder.yml || warn "Linux Electron package failed" ;;
        Darwin) npx electron-builder --mac --config electron-builder.yml || warn "macOS package failed" ;;
        MINGW*|MSYS*|CYGWIN*) npx electron-builder --win --config electron-builder.yml || warn "Windows package failed" ;;
        *) warn "Unknown OS for Electron: $(uname -s)" ;;
      esac
      ;;
  esac
}

build_android() {
  if ! command -v java >/dev/null && [[ -z "${ANDROID_HOME:-}" ]]; then
    warn "Android SDK / Java not found — exporting web bundle for EAS instead"
  fi
  log "Expo export (Android-capable JS bundle)"
  mkdir -p "$OUT/android-export"
  if npx expo export --platform android --output-dir "$OUT/android-export" 2>/dev/null; then
    log "Android JS export → $OUT/android-export"
  else
    warn "expo export failed. On a machine with Android Studio run: npx expo prebuild --platform android && cd android && ./gradlew assembleRelease"
    mkdir -p "$OUT/android-export"
    printf '%s\n' "Run on a machine with Android SDK:" "  npx expo prebuild --platform android" "  cd android && ./gradlew assembleRelease" > "$OUT/android-export/README.txt"
  fi
}

run_tests() {
  log "Tests"
  node --test tests/*.test.js
}

usage() {
  cat << EOF
AISetu build.sh

  ./build.sh           web UI + portable app + tests
  ./build.sh web       production web UI only
  ./build.sh portable  Node-based app folder + .tar.gz
  ./build.sh desktop   Electron for this OS
  ./build.sh linux     Electron Linux
  ./build.sh win       Electron Windows
  ./build.sh mac       Electron macOS
  ./build.sh android   Expo Android export / instructions
  ./build.sh all       everything this machine can produce
EOF
}

TARGET="${1:-default}"

need_node
install_deps
mkdir -p "$OUT"

case "$TARGET" in
  -h|--help|help) usage ;;
  web)
    build_web
    ;;
  portable)
    build_web
    build_portable
    ;;
  desktop)
    build_web
    build_electron
    ;;
  linux|win|windows|mac|macos|darwin)
    build_web
    build_electron "$TARGET"
    ;;
  android)
    build_android
    ;;
  all)
    run_tests
    build_web
    build_portable
    build_electron
    build_android
    ;;
  default)
    run_tests
    build_web
    build_portable
    ;;
  *)
    usage
    die "Unknown target: $TARGET"
    ;;
esac

log "Done. Artifacts in $OUT"
ls -la "$OUT" || true
