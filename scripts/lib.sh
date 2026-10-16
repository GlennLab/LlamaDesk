#!/usr/bin/env bash
# Shared helpers for the LlamaDesk build/install scripts.
#
# This file is sourced by build.sh, install.sh and uninstall.sh. It is not
# meant to be executed on its own. Keeping the install layout here guarantees
# that install.sh and uninstall.sh always agree on the paths they touch.

# --- logging -----------------------------------------------------------------
log()  { printf '==> %s\n' "$*"; }
info() { printf '    %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "$1 is required but was not found in PATH"
}

# --- dry-run aware command runner -------------------------------------------
# run <cmd> [args...] executes the command, or prints it when DRY_RUN=1.
DRY_RUN=0
run() {
  if [ "$DRY_RUN" -eq 1 ]; then
    printf '[dry-run] %s\n' "$*"
  else
    "$@"
  fi
}

# --- repository layout -------------------------------------------------------
script_dir() { cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd; }
repo_root()  { cd -- "$(script_dir)/.." && pwd; }

# --- install layout ----------------------------------------------------------
SUDO=""
NEED_SUDO=0

# resolve_paths <user|system> <prefix-explicit 0|1> <prefix>
# Populates: PREFIX APP_DIR BIN_DIR DESKTOP_DIR ICON_ROOT REAL_BIN LAUNCHER NEED_SUDO
resolve_paths() {
  local mode="$1" explicit="$2" prefix="$3"
  if [ "$mode" = user ]; then
    PREFIX="${prefix:-$HOME/.local}"
    APP_DIR="$PREFIX"
    BIN_DIR="$PREFIX/bin"
    DESKTOP_DIR="$PREFIX/share/applications"
    ICON_ROOT="$PREFIX/share/icons/hicolor"
    REAL_BIN="$BIN_DIR/llamadesk"
    LAUNCHER=""
    NEED_SUDO=0
  elif [ "$explicit" -eq 1 ]; then
    PREFIX="${prefix:-/opt/llamadesk}"
    APP_DIR="$PREFIX"
    BIN_DIR="$PREFIX/bin"
    DESKTOP_DIR="$PREFIX/share/applications"
    ICON_ROOT="$PREFIX/share/icons/hicolor"
    REAL_BIN="$APP_DIR/llamadesk"
    LAUNCHER="$BIN_DIR/llamadesk"
    NEED_SUDO=1
  else
    PREFIX="/opt/llamadesk"
    APP_DIR="/opt/llamadesk"
    BIN_DIR="/usr/local/bin"
    DESKTOP_DIR="/usr/share/applications"
    ICON_ROOT="/usr/share/icons/hicolor"
    REAL_BIN="$APP_DIR/llamadesk"
    LAUNCHER="$BIN_DIR/llamadesk"
    NEED_SUDO=1
  fi
}

# Refuse to touch obviously dangerous prefixes.
assert_safe_prefix() {
  local p="$1"
  [ -n "$p" ] || die "refusing to operate on an empty prefix"
  [ "$p" != "/" ] || die "refusing to operate on /"
  [ "$p" != "$HOME" ] || die "refusing to operate on \$HOME directly; use a subdirectory"
}

# A system install into an explicit, user-writable prefix does not need root;
# this is what makes "system with --prefix" testable without sudo.
maybe_drop_sudo() {
  [ "$NEED_SUDO" -eq 1 ] || return 0
  local p="$PREFIX"
  while [ ! -e "$p" ] && [ "$p" != "/" ]; do p="$(dirname -- "$p")"; done
  [ -w "$p" ] && NEED_SUDO=0
  return 0
}

# resolve_sudo -> SUDO is "", "sudo", or the script dies with a clear message.
resolve_sudo() {
  [ "$NEED_SUDO" -eq 1 ] || return 0
  if [ "$(id -u)" -eq 0 ]; then
    SUDO=""
  elif command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
  elif [ "$DRY_RUN" -eq 1 ]; then
    SUDO="sudo"
    warn "sudo is not available; continuing because this is a dry run"
  else
    die "a system install requires root or sudo, and neither is available"
  fi
}

# --- runtime dependency check (warn only, never fails) -----------------------
check_runtime_dependencies() {
  local ldconfig cache lib pkg missing=""
  ldconfig="$(command -v ldconfig || true)"
  if [ -z "$ldconfig" ] && [ -x /usr/sbin/ldconfig ]; then
    ldconfig=/usr/sbin/ldconfig
  fi
  if [ -z "$ldconfig" ]; then
    warn "ldconfig not found; skipping runtime dependency check"
    return 0
  fi
  cache="$("$ldconfig" -p 2>/dev/null || true)"

  while read -r lib pkg; do
    [ -n "$lib" ] || continue
    # Avoid printf|grep -q: with `set -o pipefail` grep closing the pipe early
    # makes printf exit via SIGPIPE and the library is wrongly reported missing.
    case "$cache" in
      *"$lib"*) ;;
      *) missing="$missing $pkg" ;;
    esac
  done <<'DEPS'
libwebkit2gtk-4.1.so.0 libwebkit2gtk-4.1-0
libgtk-3.so.0 libgtk-3-0
libsoup-3.0.so.0 libsoup-3.0-0
libayatana-appindicator3.so.1 libayatana-appindicator3-1
DEPS

  if [ -n "$missing" ]; then
    warn "missing runtime libraries:$missing"
    warn "install them with: sudo apt install$missing"
  else
    info "runtime libraries present (WebKitGTK 4.1, GTK 3, libsoup 3, appindicator)"
  fi
}

# --- XDG desktop database ----------------------------------------------------
refresh_desktop_database() {
  [ -d "$1" ] || return 0
  if command -v update-desktop-database >/dev/null 2>&1; then
    run $SUDO update-desktop-database "$1" 2>/dev/null || warn "update-desktop-database failed for $1"
  else
    info "update-desktop-database not found; skipping desktop cache refresh"
  fi
}
