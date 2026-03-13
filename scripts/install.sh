#!/usr/bin/env bash
#
# FineType installer
# https://meridian.online
#
# Detects OS and architecture, downloads the correct pre-built binary from
# GitHub Releases, verifies its SHA256 checksum, and installs it to a
# versioned directory with a latest symlink.
#
# Usage:
#   curl -fsSL https://install.meridian.online/finetype | bash
#   curl -fsSL https://install.meridian.online/finetype | bash -s -- v0.6.11
#
set -euo pipefail

REPO="meridian-online/finetype"
INSTALL_DIR="${HOME}/.finetype/cli"
LOCAL_BIN="${HOME}/.local/bin"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

info()  { printf '  %s\n' "$*"; }
warn()  { printf '  \033[33mwarning:\033[0m %s\n' "$*" >&2; }
error() { printf '  \033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  if ! command -v "$1" &>/dev/null; then
    error "need '$1' (command not found)"
  fi
}

# ---------------------------------------------------------------------------
# Detect platform
# ---------------------------------------------------------------------------

detect_target() {
  local os arch target

  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-gnu" ;;
    *)      error "unsupported operating system: ${os}" ;;
  esac

  case "${arch}" in
    x86_64|amd64)   arch="x86_64" ;;
    aarch64|arm64)   arch="aarch64" ;;
    *)               error "unsupported architecture: ${arch}" ;;
  esac

  target="${arch}-${os}"
  echo "${target}"
}

# ---------------------------------------------------------------------------
# Resolve version
# ---------------------------------------------------------------------------

resolve_version() {
  local version="$1"

  if [[ -n "${version}" ]]; then
    # Ensure the version starts with 'v'
    if [[ "${version}" != v* ]]; then
      version="v${version}"
    fi
    echo "${version}"
    return
  fi

  # Query GitHub API for the latest release tag
  local api_url="https://api.github.com/repos/${REPO}/releases/latest"
  local tag

  tag="$(curl -fsSL "${api_url}" | grep '"tag_name"' | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')"

  if [[ -z "${tag}" ]]; then
    error "could not determine latest version from GitHub API"
  fi

  echo "${tag}"
}

# ---------------------------------------------------------------------------
# Download and verify
# ---------------------------------------------------------------------------

download_and_verify() {
  local version="$1"
  local target="$2"
  local tmp_dir="$3"

  local archive="finetype-${version}-${target}.tar.gz"
  local checksum_file="${archive}.sha256"
  local base_url="https://github.com/${REPO}/releases/download/${version}"

  info "downloading ${archive}..."
  curl -fSL --progress-bar -o "${tmp_dir}/${archive}" "${base_url}/${archive}" \
    || error "failed to download ${archive} -- does release ${version} exist?"

  info "downloading checksum..."
  curl -fsSL -o "${tmp_dir}/${checksum_file}" "${base_url}/${checksum_file}" \
    || error "failed to download checksum file"

  # Verify SHA256
  info "verifying checksum..."
  local expected actual
  expected="$(awk '{print $1}' "${tmp_dir}/${checksum_file}")"

  if command -v shasum &>/dev/null; then
    actual="$(shasum -a 256 "${tmp_dir}/${archive}" | awk '{print $1}')"
  elif command -v sha256sum &>/dev/null; then
    actual="$(sha256sum "${tmp_dir}/${archive}" | awk '{print $1}')"
  else
    error "no SHA256 tool found (need shasum or sha256sum)"
  fi

  if [[ "${expected}" != "${actual}" ]]; then
    error "checksum mismatch!\n  expected: ${expected}\n  actual:   ${actual}"
  fi

  info "checksum verified."
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

install_binary() {
  local version="$1"
  local tmp_dir="$2"
  local target="$3"

  local archive="finetype-${version}-${target}.tar.gz"
  local version_dir="${INSTALL_DIR}/${version}"
  local latest_link="${INSTALL_DIR}/latest"

  # Create versioned directory
  mkdir -p "${version_dir}"

  # Extract binary
  info "extracting to ${version_dir}..."
  tar xzf "${tmp_dir}/${archive}" -C "${version_dir}"
  chmod +x "${version_dir}/finetype"

  # Update latest symlink
  ln -sfn "${version_dir}" "${latest_link}"
  info "updated ${latest_link} -> ${version_dir}"

  # Optionally symlink into ~/.local/bin
  if [[ -d "${LOCAL_BIN}" && -w "${LOCAL_BIN}" ]]; then
    ln -sf "${latest_link}/finetype" "${LOCAL_BIN}/finetype"
    info "symlinked ${LOCAL_BIN}/finetype -> ${latest_link}/finetype"
  fi
}

# ---------------------------------------------------------------------------
# PATH guidance
# ---------------------------------------------------------------------------

print_path_help() {
  local on_path=false

  # Check if either location is already on PATH
  if command -v finetype &>/dev/null; then
    on_path=true
  fi

  if [[ "${on_path}" == true ]]; then
    return
  fi

  echo ""
  warn "finetype is not on your PATH."
  echo ""

  if [[ -d "${LOCAL_BIN}" ]]; then
    info "Add ~/.local/bin to your PATH:"
  else
    info "Add the FineType install directory to your PATH:"
  fi

  echo ""

  local shell_name
  shell_name="$(basename "${SHELL:-bash}")"

  local target_dir
  if [[ -d "${LOCAL_BIN}" ]]; then
    target_dir="\$HOME/.local/bin"
  else
    target_dir="\$HOME/.finetype/cli/latest"
  fi

  case "${shell_name}" in
    zsh)
      info "  echo 'export PATH=\"${target_dir}:\$PATH\"' >> ~/.zshrc"
      info "  source ~/.zshrc"
      ;;
    fish)
      info "  fish_add_path ${target_dir}"
      ;;
    *)
      info "  echo 'export PATH=\"${target_dir}:\$PATH\"' >> ~/.bashrc"
      info "  source ~/.bashrc"
      ;;
  esac

  echo ""
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
  echo ""
  echo "  FineType installer"
  echo "  =================="
  echo ""

  # Check dependencies
  need_cmd curl
  need_cmd tar
  need_cmd uname

  # Detect platform
  local target
  target="$(detect_target)"
  info "detected platform: ${target}"

  # Resolve version (use first argument if provided)
  local version
  version="$(resolve_version "${1:-}")"
  info "version: ${version}"
  echo ""

  # Create temp directory for downloads
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT

  # Download, verify, install
  download_and_verify "${version}" "${target}" "${tmp_dir}"
  install_binary "${version}" "${tmp_dir}" "${target}"

  echo ""
  echo "  FineType ${version} installed successfully!"
  echo ""

  # Verify the binary runs
  local installed_bin="${INSTALL_DIR}/${version}/finetype"
  if "${installed_bin}" --version &>/dev/null; then
    info "$(${installed_bin} --version)"
  fi

  print_path_help
}

main "$@"
