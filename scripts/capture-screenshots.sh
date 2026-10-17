#!/usr/bin/env bash
#
# capture-screenshots.sh - best-effort helper to capture the five LlamaDesk
# pages into PNGs for the docs.
#
# This is OPTIONAL and interactive. It never fabricates images and is never
# required to build or release LlamaDesk. It drives the operator's live
# desktop: for each page it asks the operator to bring that page up in a
# running LlamaDesk and press Enter, then captures the screen with the first
# available backend. There is no click automation available, so the operator
# must switch pages themselves.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
. "$SCRIPT_DIR/lib.sh"

OUT=""
OPT_DISPLAY=""
CHECK=0

# A real screenshot is far larger than this; a smaller file means the capture
# tool failed rather than produced a usable image.
MIN_PNG_BYTES=1024

# The five pages, and their human-readable labels for the prompts.
PAGES=(dashboard slots performance hardware settings)
LABELS=(Dashboard Slots Performance Hardware Settings)

usage() {
  cat <<'EOF'
Usage: scripts/capture-screenshots.sh [OPTIONS] [OUTPUT_DIR]

Capture the five LlamaDesk pages (Dashboard, Slots, Performance, Hardware,
Settings) as <page>.png files under OUTPUT_DIR (default docs/screenshots/).

This is an optional, interactive helper: it asks you to bring each page up in
a running LlamaDesk and press Enter, then captures the screen. It never
fabricates images and is never required for a release.

Options:
  --display DISPLAY   X display to capture (default: $DISPLAY).
  --check             Detect the capture backend and exit without capturing.
  -h, --help          Show this help.

Backends are tried in order:
  1. GNOME Shell screenshot over D-Bus (gdbus + a running GNOME session).
  2. ffmpeg -f x11grab (ffmpeg + xwininfo).
  3. xwd (converted to PNG with ffmpeg).

If none is available the script exits non-zero; see docs/screenshots/README.md
for how to capture the pages manually.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --display)
      shift
      [ "$#" -gt 0 ] || die "--display requires an argument"
      OPT_DISPLAY="$1"
      ;;
    --display=*) OPT_DISPLAY="${1#*=}" ;;
    --check) CHECK=1 ;;
    -*) die "unknown option: $1 (try --help)" ;;
    *)
      [ -z "$OUT" ] || die "unexpected argument: $1 (try --help)"
      OUT="$1"
      ;;
  esac
  shift
done

DISPLAY="${OPT_DISPLAY:-${DISPLAY:-}}"
ROOT="$(repo_root)"
[ -n "$OUT" ] || OUT="$ROOT/docs/screenshots"

# --- requirement checks ------------------------------------------------------
if [ "$CHECK" -eq 0 ] && [ ! -t 0 ]; then
  die "this helper is interactive; run it from a terminal (or use --check)"
fi

if [ -z "$DISPLAY" ]; then
  cat >&2 <<EOF
error: no display to capture (DISPLAY is empty).

Pass --display :0 (or your session's DISPLAY). Capture is optional and is never
required for a release; see docs/screenshots/README.md for how to capture the
five pages manually as dashboard.png, slots.png, performance.png, hardware.png
and settings.png.
EOF
  exit 1
fi

# --- backend detection -------------------------------------------------------
# True when GNOME Shell's screenshot D-Bus service is running on this session.
gnome_screenshot_service_present() {
  command -v gdbus >/dev/null 2>&1 || return 1
  local out
  out="$(gdbus call --session \
    --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.NameHasOwner \
    org.gnome.Shell.Screenshot 2>/dev/null)" || return 1
  case "$out" in *true*) return 0 ;; *) return 1 ;; esac
}

# Prints the first usable backend name, or returns non-zero if none is usable.
detect_backend() {
  if gnome_screenshot_service_present; then
    printf 'gnome-dbus\n'
    return 0
  fi
  if command -v ffmpeg >/dev/null 2>&1 && command -v xwininfo >/dev/null 2>&1; then
    printf 'ffmpeg-x11grab\n'
    return 0
  fi
  # xwd only produces an .xwd file; ffmpeg is required to convert it to PNG.
  if command -v ffmpeg >/dev/null 2>&1 && command -v xwd >/dev/null 2>&1; then
    printf 'xwd\n'
    return 0
  fi
  return 1
}

die_no_backend() {
  cat >&2 <<EOF
error: no supported screenshot backend is available.

Capture is optional and is never required for a release. Capture each page
manually and save it under the output directory as:
  dashboard.png  slots.png  performance.png  hardware.png  settings.png
See docs/screenshots/README.md for the expected filenames and sizes.

Backends tried, in order:
  1. GNOME Shell screenshot over D-Bus (needs gdbus and a running GNOME session)
  2. ffmpeg -f x11grab (needs ffmpeg and xwininfo)
  3. xwd (needs xwd; converted to PNG with ffmpeg)
EOF
  exit 1
}

BACKEND="$(detect_backend)" || die_no_backend

if [ "$CHECK" -eq 1 ]; then
  log "capture backend: $BACKEND"
  info "display: $DISPLAY"
  info "output dir: $OUT"
  info "pages: ${LABELS[*]}"
  exit 0
fi

# --- capture backends --------------------------------------------------------
TMP_FILES=""
cleanup_tmp() { [ -z "$TMP_FILES" ] || rm -f -- $TMP_FILES; }
trap cleanup_tmp EXIT

# Prints the root window geometry as WxH, or nothing if it cannot be determined.
screen_size() {
  local size
  size="$(xwininfo -root -display "$DISPLAY" 2>/dev/null \
    | awk -F'[ :]+' '/Width:/{w=$NF} /Height:/{h=$NF} END{if (w && h) print w"x"h}')" || true
  printf '%s' "$size"
}

capture_gnome_dbus() { # <outfile>
  local out="$1" abs
  case "$out" in
    /*) abs="$out" ;;
    *) abs="$PWD/$out" ;;
  esac
  rm -f -- "$abs"
  gdbus call --session \
    --dest org.gnome.Shell.Screenshot \
    --object-path /org/gnome/Shell/Screenshot \
    --method org.gnome.Shell.Screenshot.Screenshot \
    false false "$abs" >/dev/null 2>&1 || return 1
  [ -f "$abs" ] || return 1
}

capture_ffmpeg_x11grab() { # <outfile>
  local out="$1" size
  size="$(screen_size)"
  [ -n "$size" ] || { warn "could not determine screen size with xwininfo"; return 1; }
  ffmpeg -hide_banner -loglevel error -y \
    -f x11grab -video_size "$size" -i "$DISPLAY" -frames:v 1 "$out" || return 1
}

capture_xwd() { # <outfile>
  local out="$1" tmp
  tmp="$(mktemp --suffix=.xwd)"
  TMP_FILES="$TMP_FILES $tmp"
  xwd -root -display "$DISPLAY" -silent -out "$tmp" || return 1
  ffmpeg -hide_banner -loglevel error -y -f xwd_pipe -i "$tmp" -frames:v 1 "$out" || return 1
}

capture() { # <backend> <outfile>
  case "$1" in
    gnome-dbus) capture_gnome_dbus "$2" ;;
    ffmpeg-x11grab) capture_ffmpeg_x11grab "$2" ;;
    xwd) capture_xwd "$2" ;;
    *) return 1 ;;
  esac
}

# Returns 0 when $1 is a real, plausibly-sized PNG, non-zero otherwise. Never
# aborts: the caller decides whether one bad page loses the whole run.
verify_png() { # <file>
  local f="$1" size magic
  [ -f "$f" ] || { warn "capture did not produce $f"; return 1; }
  size="$(wc -c <"$f" | tr -d ' ')"
  [ "$size" -ge "$MIN_PNG_BYTES" ] || { warn "captured file is implausibly small (${size} bytes): $f"; return 1; }
  magic="$(od -An -tx1 -N4 -- "$f" | tr -d ' \n')"
  [ "$magic" = "89504e47" ] || { warn "captured file is not a PNG: $f"; return 1; }
}

human_size() { du -h -- "$1" | cut -f1; }

# --- capture -----------------------------------------------------------------
mkdir -p -- "$OUT"
log "Capturing ${#PAGES[@]} pages into $OUT (backend: $BACKEND, display: $DISPLAY)"
info "bring each page up in LlamaDesk and press Enter when prompted"

SUCCEEDED=0
FAILED=""
for i in "${!PAGES[@]}"; do
  page="${PAGES[$i]}"
  label="${LABELS[$i]}"
  printf '\n>>> Show the %s page in LlamaDesk, then press Enter to capture... ' "$label" >&2
  read -r _ || { warn "input closed; stopping."; break; }
  out="$OUT/$page.png"
  if capture "$BACKEND" "$out" && verify_png "$out"; then
    SUCCEEDED=$((SUCCEEDED + 1))
    info "wrote $out ($(human_size "$out"))"
  else
    rm -f -- "$out"
    FAILED="$FAILED $label"
    warn "failed to capture the $label page; continuing with remaining pages"
  fi
done

if [ "$SUCCEEDED" -eq 0 ]; then
  die "no screenshots were captured${FAILED:+; failed:${FAILED# }}"
fi

if [ -n "$FAILED" ]; then
  warn "captured $SUCCEEDED of ${#PAGES[@]} page(s); skipped:${FAILED# }"
else
  log "Done. Captured all ${#PAGES[@]} screenshots into $OUT"
fi
