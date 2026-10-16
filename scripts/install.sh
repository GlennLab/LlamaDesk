#!/usr/bin/env bash
#
# install.sh - install an already-built LlamaDesk into a user or system prefix.
#
# Run scripts/build.sh first to produce the artifacts this script consumes.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
. "$SCRIPT_DIR/lib.sh"

MODE=user
PREFIX=""
PREFIX_EXPLICIT=0

usage() {
  cat <<'EOF'
Usage: scripts/install.sh [OPTIONS]

Install LlamaDesk from a built artifact (run scripts/build.sh first).

Modes:
  --user           Install for the current user (default). No sudo.
                   Files go under ~/.local, or under --prefix if given.
  --system         Install system-wide: /opt/llamadesk plus a /usr/local/bin
                   launcher, using sudo when not already root. A .deb payload
                   is installed natively with `dpkg -i` instead.

Options:
  --prefix DIR     Install under DIR instead of the mode default. This is the
                   supported way to test an install without touching the real
                   system.
  --dry-run        Print the actions that would be taken, changing nothing.
  -h, --help       Show this help.

Payload selection (first that exists):
  1. target/release/bundle/deb/*.deb        -> dpkg -i (system) or extracted
  2. target/release/bundle/appimage/*.AppImage
  3. target/release/llamadesk               (plain binary)

The installer writes an XDG .desktop entry and icon, validates and refreshes
the desktop database when the tools are available, and is idempotent. It never
deletes user data and never restarts a running server.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --user) MODE=user ;;
    --system) MODE=system ;;
    --prefix)
      shift
      [ "$#" -gt 0 ] || die "--prefix requires a directory"
      PREFIX="$1"
      PREFIX_EXPLICIT=1
      ;;
    --prefix=*)
      PREFIX="${1#*=}"
      PREFIX_EXPLICIT=1
      ;;
    --dry-run) DRY_RUN=1 ;;
    -*) die "unknown option: $1 (try --help)" ;;
    *) die "unexpected argument: $1 (try --help)" ;;
  esac
  shift
done

ROOT="$(repo_root)"

# --- locate the best available payload ---------------------------------------
first_match() { # <dir> <glob>
  local dir="$1" pat="$2" f
  [ -d "$dir" ] || return 1
  for f in "$dir"/$pat; do
    [ -e "$f" ] || continue
    printf '%s\n' "$f"
    return 0
  done
  return 1
}

DEB="$(first_match "$ROOT/target/release/bundle/deb" '*.deb' || true)"
[ -n "$DEB" ] || DEB="$(first_match "$ROOT/src-tauri/target/release/bundle/deb" '*.deb' || true)"
APPIMAGE="$(first_match "$ROOT/target/release/bundle/appimage" '*.AppImage' || true)"
[ -n "$APPIMAGE" ] || APPIMAGE="$(first_match "$ROOT/src-tauri/target/release/bundle/appimage" '*.AppImage' || true)"
BINARY=""
for candidate in "$ROOT/target/release/llamadesk" "$ROOT/src-tauri/target/release/llamadesk"; do
  [ -x "$candidate" ] && BINARY="$candidate" && break
done

PAYLOAD=""
if [ -n "$DEB" ] && [ "$MODE" = system ] && [ "$PREFIX_EXPLICIT" -eq 0 ]; then
  PAYLOAD=dpkg
elif [ -n "$DEB" ]; then
  PAYLOAD=extract-deb
elif [ -n "$APPIMAGE" ]; then
  PAYLOAD=appimage
elif [ -n "$BINARY" ]; then
  PAYLOAD=binary
else
  die "no build artifact found; run scripts/build.sh first"
fi

# --- resolve the install layout ----------------------------------------------
if [ "$PAYLOAD" = dpkg ]; then
  BIN_DIR="/usr/bin"
  DESKTOP_DIR="/usr/share/applications"
  ICON_ROOT="/usr/share/icons/hicolor"
  REAL_BIN="/usr/bin/llamadesk"
  LAUNCHER=""
  APP_DIR=""
  NEED_SUDO=1
else
  resolve_paths "$MODE" "$PREFIX_EXPLICIT" "$PREFIX"
  assert_safe_prefix "$PREFIX"
  maybe_drop_sudo
fi
resolve_sudo

DESKTOP_FILE="$DESKTOP_DIR/LlamaDesk.desktop"

WORK=""
cleanup() { [ -n "$WORK" ] && rm -rf -- "$WORK"; }
trap cleanup EXIT

# --- helpers -----------------------------------------------------------------
mkdirs() { # <dir>...
  local d
  for d in "$@"; do run $SUDO mkdir -p -- "$d"; done
}

install_icon() { # <src> <size>
  local src="$1" size="$2"
  [ -f "$src" ] || return 0
  mkdirs "$ICON_ROOT/$size/apps"
  run $SUDO install -m 0644 -- "$src" "$ICON_ROOT/$size/apps/llamadesk.png"
}

write_desktop_entry() { # <dest> <exec> <icon>
  local dest="$1" exec_path="$2" icon_path="$3" tmp
  tmp="$(mktemp)"
  cat >"$tmp" <<EOF
[Desktop Entry]
Type=Application
Name=LlamaDesk
Comment=A desktop monitor for local LLM inference servers
Exec=$exec_path
Icon=$icon_path
Terminal=false
Categories=System;Monitor;
StartupNotify=true
StartupWMClass=llamadesk
EOF
  mkdirs "$(dirname -- "$dest")"
  run $SUDO install -m 0644 -- "$tmp" "$dest"
  rm -f -- "$tmp"
}

# --- stage the payload --------------------------------------------------------
BIN_SRC=""
REPO_ICONS="$ROOT/src-tauri/icons"

case "$PAYLOAD" in
  dpkg)
    log "Installing .deb with dpkg: $DEB"
    run $SUDO dpkg -i "$DEB"
    ;;
  extract-deb)
    log "Extracting .deb: $DEB"
    WORK="$(mktemp -d)"
    # Confined to $WORK (removed on exit); also run under --dry-run so the
    # preview reflects the real .deb layout.
    dpkg-deb -x "$DEB" "$WORK"
    BIN_SRC="$WORK/usr/bin/llamadesk"
    [ -f "$BIN_SRC" ] || die "unexpected .deb layout: $BIN_SRC not found"
    for apps in "$WORK"/usr/share/icons/hicolor/*/apps; do
      [ -d "$apps" ] || continue
      install_icon "$apps/llamadesk.png" "$(basename -- "$(dirname -- "$apps")")"
    done
    ;;
  appimage)
    log "Installing AppImage: $APPIMAGE"
    BIN_SRC="$APPIMAGE"
    install_icon "$REPO_ICONS/32x32.png" 32x32
    install_icon "$REPO_ICONS/128x128.png" 128x128
    install_icon "$REPO_ICONS/128x128@2x.png" 256x256@2
    ;;
  binary)
    log "Installing binary: $BINARY"
    BIN_SRC="$BINARY"
    install_icon "$REPO_ICONS/32x32.png" 32x32
    install_icon "$REPO_ICONS/128x128.png" 128x128
    install_icon "$REPO_ICONS/128x128@2x.png" 256x256@2
    ;;
esac

# --- install the binary and launcher -----------------------------------------
if [ "$PAYLOAD" != dpkg ]; then
  mkdirs "$(dirname -- "$REAL_BIN")" "$BIN_DIR"
  run $SUDO install -m 0755 -- "$BIN_SRC" "$REAL_BIN"
  if [ -n "$LAUNCHER" ]; then
    run $SUDO ln -sf -- "$REAL_BIN" "$LAUNCHER"
  fi
fi

# --- desktop entry ------------------------------------------------------------
DESKTOP_ICON="$ICON_ROOT/128x128/apps/llamadesk.png"
case "$PAYLOAD" in
  appimage | binary) [ -f "$REPO_ICONS/128x128.png" ] || DESKTOP_ICON="llamadesk" ;;
  extract-deb) [ -f "$WORK/usr/share/icons/hicolor/128x128/apps/llamadesk.png" ] || DESKTOP_ICON="llamadesk" ;;
esac

DESKTOP_EXEC="$REAL_BIN"
[ -n "$LAUNCHER" ] && DESKTOP_EXEC="$LAUNCHER"

write_desktop_entry "$DESKTOP_FILE" "$DESKTOP_EXEC" "$DESKTOP_ICON"

if command -v desktop-file-validate >/dev/null 2>&1; then
  run desktop-file-validate "$DESKTOP_FILE" || warn "desktop-file-validate reported issues for $DESKTOP_FILE"
else
  info "desktop-file-validate not found; skipping desktop entry validation"
fi
refresh_desktop_database "$DESKTOP_DIR"

# --- runtime dependencies (warn only) ----------------------------------------
check_runtime_dependencies

# --- summary ------------------------------------------------------------------
if [ "$DRY_RUN" -eq 1 ]; then
  log "Dry run complete; nothing was changed."
  exit 0
fi

log "Installed LlamaDesk"
info "mode:     $MODE"
info "binary:   $REAL_BIN"
[ -n "$LAUNCHER" ] && info "launcher: $LAUNCHER"
info "desktop:  $DESKTOP_FILE"
info "icon:     $DESKTOP_ICON"
if [ "$MODE" = user ]; then
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) warn "$BIN_DIR is not on your PATH; add it to run 'llamadesk' directly" ;;
  esac
fi
info "Run it with: $DESKTOP_EXEC"
