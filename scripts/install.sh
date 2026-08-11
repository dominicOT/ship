#!/usr/bin/env bash
set -euo pipefail

REPO="dominicOT/ship"
PROG="ship"
DEFAULT_PREFIX="${HOME}/.local/bin"
PREFIX="${PREFIX:-$DEFAULT_PREFIX}"
DOWNLOAD_BASE="https://github.com/${REPO}/releases/latest/download"

usage() {
  cat <<'EOF'
Usage: install.sh [--prefix DIR]

Install the latest ship release from GitHub.

Options:
  --prefix DIR  install destination directory (default: ~/.local/bin)
  --help        show this help message
EOF
}

err() {
  printf 'Error: %s\n' "$1" >&2
  exit 1
}

if [ "$#" -gt 0 ]; then
  while [ $# -gt 0 ]; do
    case "$1" in
      --help|-h)
        usage
        exit 0
        ;;
      --prefix)
        shift
        [ $# -eq 0 ] && err 'missing argument for --prefix'
        PREFIX="$1"
        shift
        ;;
      *)
        err "unknown option: $1"
        ;;
    esac
  done
fi

if command -v curl >/dev/null 2>&1; then
  downloader="curl"
elif command -v wget >/dev/null 2>&1; then
  downloader="wget"
else
  err 'curl or wget is required to download the release asset'
fi

uname_os=$(uname -s)
case "$uname_os" in
  Linux) os=linux ;;
  *) err "Pre-built binaries currently only support Linux. For $uname_os, please install from source with: cargo install --path ." ;;
esac

uname_arch=$(uname -m)
case "$uname_arch" in
  x86_64|amd64) arch=x86_64 ;;
  aarch64|arm64) arch=aarch64 ;;
  *) err "unsupported architecture: $uname_arch" ;;
esac

case "$os" in
  windows)
    archive=zip
    binary_name="$PROG.exe"
    ;;
  *)
    archive=tar.gz
    binary_name="$PROG"
    ;;
esac

asset_name="$PROG-$os-$arch.$archive"
asset_url="$DOWNLOAD_BASE/$asset_name"

echo "Installing $PROG from $asset_url"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
asset_path="$tmpdir/$asset_name"

if [ "$downloader" = "curl" ]; then
  curl -fSL "$asset_url" -o "$asset_path"
else
  wget -qO "$asset_path" "$asset_url"
fi

mkdir -p "$PREFIX"

case "$archive" in
  tar.gz)
    tar -xzf "$asset_path" -C "$tmpdir"
    ;;
  zip)
    if command -v unzip >/dev/null 2>&1; then
      unzip -q "$asset_path" -d "$tmpdir"
    elif command -v python3 >/dev/null 2>&1; then
      python3 -c "import zipfile, sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "$asset_path" "$tmpdir"
    else
      err 'unzip or python3 is required to extract the zip archive'
    fi
    ;;
  *)
    err "unsupported archive format: $archive"
    ;;
esac

binary_source="$tmpdir/$binary_name"
[ -f "$binary_source" ] || err "downloaded archive does not contain $binary_name"
install_path="$PREFIX/$binary_name"

install -m 755 "$binary_source" "$install_path"

echo "Installed $PROG to $install_path"
if ! printf '%s' ":$PATH:" | grep -q ":$PREFIX:"; then
  echo
  echo "Add $PREFIX to your PATH, for example:"
  echo "  export PATH=\"$PREFIX:\$PATH\""
fi

echo "Done. Run '$PROG --help' to verify the installation."
