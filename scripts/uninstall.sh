#!/usr/bin/env bash
#
# uninstall.sh - remove the files installed by scripts/install.sh.
#
# Mirrors install.sh: same modes, same --prefix, same layout. User data is
# preserved unless --purge is given.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
. "$SCRIPT_DIR/lib.sh"

MODE=user
PREFIX=""
PREFIX_EXPLICIT=0
PURGE=0

usage() {
  cat <<'EOF'
Usage: scripts/uninstall.sh [OPTIONS]

Remove the files created by scripts/install.sh. Safe to run when LlamaDesk is
not installed (it exits 0 with a message).

Modes (must match how it was installed):
  --user           User install under ~/.local (default).
  --system         System install (/opt/llamadesk + /usr/local/bin, or a
                   native dpkg install), using sudo when not already root.

Options:
  --prefix DIR     The prefix used at install time.
  --purge          Also delete settings and history:
                     ~/.config/com.llamadesk.app/
                     ~/.local/share/com.llamadesk.app/
  --dry-run        Print the actions that would be taken, changing nothing.
  -h, --help       Show this help.
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
    --purge) PURGE=1 ;;
    --dry-run) DRY_RUN=1 ;;
    -*) die "unknown option: $1 (try --help)" ;;
    *) die "unexpected argument: $1 (try --help)" ;;
  esac
  shift
done

# Native dpkg installs are only undone via dpkg when no explicit prefix was
# given; with --prefix we fall back to plain path removal.
DPKG_MODE=0
if [ "$MODE" = system ] && [ "$PREFIX_EXPLICIT" -eq 0 ] && command -v dpkg-query >/dev/null 2>&1; then
  status="$(dpkg-query -W -f='${Status}' llama-desk 2>/dev/null || true)"
  [ "$status" = "install ok installed" ] && DPKG_MODE=1
fi

REMOVED=0
CONFIG_DIR="$HOME/.config/com.llamadesk.app"
DATA_DIR="$HOME/.local/share/com.llamadesk.app"

if [ "$DPKG_MODE" -eq 1 ]; then
  NEED_SUDO=1
  resolve_sudo
  log "Removing packaged install (llama-desk)"
  run $SUDO dpkg -r llama-desk
  DESKTOP_DIR="/usr/share/applications"
  refresh_desktop_database "$DESKTOP_DIR"
  REMOVED=1
else
  resolve_paths "$MODE" "$PREFIX_EXPLICIT" "$PREFIX"
  assert_safe_prefix "$PREFIX"
  maybe_drop_sudo
  resolve_sudo

  DESKTOP_FILE="$DESKTOP_DIR/LlamaDesk.desktop"

  remove_path() { # <path>
    if [ -e "$1" ] || [ -L "$1" ]; then
      run $SUDO rm -f -- "$1"
      REMOVED=1
    fi
  }

  remove_path "$REAL_BIN"
  [ -n "$LAUNCHER" ] && remove_path "$LAUNCHER"
  remove_path "$DESKTOP_FILE"

  for icon in "$ICON_ROOT"/*/apps/llamadesk.png; do
    [ -e "$icon" ] || continue
    run $SUDO rm -f -- "$icon"
    run $SUDO rmdir --ignore-fail-on-non-empty -- "$(dirname -- "$icon")" 2>/dev/null || true
    REMOVED=1
  done

  # Remove the app directory only if it is now empty (never recursively).
  if [ "$MODE" = system ] && [ -n "$APP_DIR" ] && [ "$APP_DIR" != "$BIN_DIR" ]; then
    run $SUDO rmdir --ignore-fail-on-non-empty -- "$APP_DIR" 2>/dev/null || true
  fi

  refresh_desktop_database "$DESKTOP_DIR"

  # Drop the desktop-database cache if it now holds only its header, then prune
  # any directories that became empty. rmdir refuses to touch a non-empty
  # directory, so anything shared with other applications is left intact.
  cache="$DESKTOP_DIR/mimeinfo.cache"
  if [ -f "$cache" ] && ! grep -qv -e '^\[' -e '^$' -- "$cache"; then
    run $SUDO rm -f -- "$cache"
  fi
  for d in \
    "$ICON_ROOT"/* \
    "$ICON_ROOT" \
    "$(dirname -- "$ICON_ROOT")" \
    "$DESKTOP_DIR" \
    "$(dirname -- "$(dirname -- "$ICON_ROOT")")" \
    "$BIN_DIR" \
    "$PREFIX"; do
    [ -d "$d" ] || continue
    run $SUDO rmdir --ignore-fail-on-non-empty -- "$d" 2>/dev/null || true
  done
fi

if [ "$REMOVED" -eq 0 ]; then
  log "Nothing to uninstall: LlamaDesk does not appear to be installed (mode: $MODE, prefix: ${PREFIX:-default})."
  exit 0
fi

# --- user data ----------------------------------------------------------------
if [ "$PURGE" -eq 1 ]; then
  for d in "$CONFIG_DIR" "$DATA_DIR"; do
    [ -n "$d" ] || continue
    [ "$d" != "/" ] || die "refusing to remove /"
    if [ -e "$d" ]; then
      run rm -rf -- "$d"
      info "purged $d"
    fi
  done
else
  info "preserved settings: $CONFIG_DIR"
  info "preserved history:  $DATA_DIR"
fi

if [ "$DRY_RUN" -eq 1 ]; then
  log "Dry run complete; nothing was changed."
  exit 0
fi

log "LlamaDesk uninstalled (mode: $MODE)."
