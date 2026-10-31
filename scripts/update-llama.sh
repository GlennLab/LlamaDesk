#!/usr/bin/env bash
#
# update-llama.sh - pull and rebuild llama.cpp, reporting old and new versions.
#
# Usage: ./scripts/update-llama.sh [--clean|-c] [llama_dir]
#
# Environment:
#   LLAMA_DIR           llama.cpp checkout directory (default: $HOME/llama.cpp)
set -euo pipefail

# --- argument parsing --------------------------------------------------------
CLEAN=0
LLAMA_DIR="${LLAMA_DIR:-$HOME/llama.cpp}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    -c | --clean)
      CLEAN=1
      ;;
    --clean=*)
      CLEAN=1
      ;;
    -h | --help)
      cat <<'EOF'
Usage: update-llama.sh [--clean|-c] [llama_dir]

Pulls the latest llama.cpp and rebuilds it with CUDA support, printing the
new version on stdout.

Options:
  --clean, -c   Remove the build directory before configuring (full rebuild).
  llama_dir     Path to the llama.cpp checkout (default: $LLAMA_DIR or $HOME/llama.cpp).
EOF
      exit 0
      ;;
    *)
      LLAMA_DIR="$1"
      ;;
  esac
  shift
done

cd "$LLAMA_DIR"

# --- locate the binary used to report the version ----------------------------
select_binary() {
  if [ -x "build/bin/llama-cli" ]; then
    printf 'build/bin/llama-cli'
  elif [ -x "build/bin/main" ]; then
    printf 'build/bin/main'
  elif [ -x "build/bin/llama-server" ]; then
    printf 'build/bin/llama-server'
  else
    printf ''
  fi
}

# --- report the current version ----------------------------------------------
OLD_BIN="$(select_binary)"
OLD_VERSION=""
if [ -n "$OLD_BIN" ]; then
  OLD_VERSION="$("$OLD_BIN" --version 2>/dev/null | head -n 1)" || true
  if [ -z "$OLD_VERSION" ]; then
    OLD_VERSION="$(strings "$OLD_BIN" | grep -m1 'llama.cpp' || true)"
  fi
fi
echo "Current version: ${OLD_VERSION:-<unknown>}"

# --- pull the latest sources -------------------------------------------------
# `git pull --ff-only` fails when the checkout has uncommitted changes: stash
# local changes first and restore them on exit so the update can proceed.
if ! git diff --quiet --exit-code 2>/dev/null; then
  echo "⚠️  Lokale wijzigingen gedetecteerd, stash aanmaken..." >&2
  git stash push -u -m "auto-stash pre-build $(date +%F)" >/dev/null
  trap 'if ! git stash pop >/dev/null 2>&1; then echo "⚠️ local changes could NOT be restored; they remain in your git stash — run: git stash list / git stash pop" >&2; fi' EXIT
fi
git fetch origin --quiet
# Ensure local master tracks origin/master so future pulls do not need --set-upstream
git branch --set-upstream-to=origin/master master 2>/dev/null || true
git pull --ff-only origin master --quiet

# --- optional clean rebuild --------------------------------------------------
if [ "$CLEAN" -eq 1 ]; then
  rm -rf build
fi

# --- configure and build -----------------------------------------------------
cmake -S . -B build \
  -DGGML_CUDA=ON \
  -DGGML_CUDA_FORCE_MMQ=ON \
  -DGGML_CUDA_GRAPHS=ON \
  -DLLAMA_BUILD_SERVER=ON \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_CUDA_ARCHITECTURES="86;120"

make -C build -j"$(nproc)"

# --- report the new version --------------------------------------------------
NEW_BIN="$(select_binary)"
NEW_VERSION=""
if [ -n "$NEW_BIN" ]; then
  NEW_VERSION="$("$NEW_BIN" --version 2>/dev/null | head -n 1)" || true
  if [ -z "$NEW_VERSION" ]; then
    NEW_VERSION="$(strings "$NEW_BIN" | grep -m1 'llama.cpp' || true)"
  fi
fi
echo "New version: ${NEW_VERSION:-<unknown>}"
