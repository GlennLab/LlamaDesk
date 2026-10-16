#!/usr/bin/env bash
#
# build.sh - build the LlamaDesk frontend and application bundles.
#
# Produces the release binary and installer artifacts under the workspace-root
# target/release/ directory (the root Cargo.toml is a virtual workspace).
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
. "$SCRIPT_DIR/lib.sh"

BUNDLES=""

usage() {
  cat <<'EOF'
Usage: scripts/build.sh [OPTIONS]

Build the LlamaDesk frontend and application.

Options:
  --bundles LIST   Comma-separated bundle targets to produce
                   (e.g. deb,appimage,rpm). Overrides the default.
  --all            Build every bundle the host supports (equivalent to
                   --bundles all).
  -h, --help       Show this help.

Defaults:
  Debian/Ubuntu   deb,appimage
  other Linux     appimage

RPM bundling requires `rpmbuild`; when it is missing, `rpm` is left out of the
default set so the build still succeeds. Use `--bundles rpm` only if rpmbuild
is installed.
EOF
}

# Read ID / ID_LIKE from /etc/os-release without sourcing it.
default_bundles() {
  local ids=""
  if [ -r /etc/os-release ]; then
    ids="$(sed -n 's/^\(ID\|ID_LIKE\)=//p' /etc/os-release | tr -d '"' | tr '\n' ' ')"
  fi
  case "$ids" in
    *debian* | *ubuntu*) printf 'deb,appimage\n' ;;
    *) printf 'appimage\n' ;;
  esac
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --bundles)
      shift
      [ "$#" -gt 0 ] || die "--bundles requires a comma-separated list"
      BUNDLES="$1"
      ;;
    --bundles=*) BUNDLES="${1#*=}" ;;
    --all) BUNDLES="all" ;;
    -*) die "unknown option: $1 (try --help)" ;;
    *) die "unexpected argument: $1 (try --help)" ;;
  esac
  shift
done

[ -n "$BUNDLES" ] || BUNDLES="$(default_bundles)"

ROOT="$(repo_root)"

# --- preflight ---------------------------------------------------------------
[ -f "./src-tauri/tauri.conf.json" ] && [ -f "./package.json" ] \
  || die "run this script from the repository root (expected ./src-tauri/tauri.conf.json)"
require_cmd npm
require_cmd cargo
[ -d "$ROOT/node_modules" ] || die "node_modules is missing; run: npm install"

# Validate that the requested bundles can actually be produced.
case "$BUNDLES" in
  all) BUNDLES="all" ;;
esac
if [ "$BUNDLES" != "all" ]; then
  for b in ${BUNDLES//,/ }; do
    case "$b" in
      deb) require_cmd dpkg-deb ;;
      rpm) require_cmd rpmbuild || die "rpm bundling requires rpmbuild; drop 'rpm' or install rpm/rpm-build" ;;
      appimage) : ;;
      "") die "empty bundle name in --bundles '$BUNDLES'" ;;
      *) die "unknown bundle '$b' (expected deb, rpm or appimage)" ;;
    esac
  done
fi

cd "$ROOT"

# --- build --------------------------------------------------------------------
log "Building frontend and application (bundles: $BUNDLES)"
cargo tauri build --bundles "$BUNDLES"

# --- verify the binary was produced ------------------------------------------
BIN=""
for candidate in "$ROOT/target/release/llamadesk" "$ROOT/src-tauri/target/release/llamadesk"; do
  if [ -x "$candidate" ]; then
    BIN="$candidate"
    break
  fi
done
[ -n "$BIN" ] || die "the build did not produce the llamadesk binary"

# --- report artifacts ---------------------------------------------------------
human_size() { du -h -- "$1" | cut -f1; }

print_match() { # <dir> <glob>
  local dir="$1" pat="$2" f
  [ -d "$dir" ] || return 0
  for f in "$dir"/$pat; do
    [ -e "$f" ] || continue
    printf '  %-64s %s\n' "$f" "$(human_size "$f")"
  done
}

log "Build artifacts:"
printf '  %-64s %s\n' "$BIN" "$(human_size "$BIN")"
print_match "$ROOT/target/release/bundle/deb" '*.deb'
print_match "$ROOT/target/release/bundle/rpm" '*.rpm'
print_match "$ROOT/target/release/bundle/appimage" '*.AppImage'

log "Done."
