#!/usr/bin/env bash
# xfetch - cross-platform system information fetcher
# Uninstaller script
# Usage: curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/uninstall.sh | bash
#        bash uninstall.sh

set -euo pipefail
IFS=$'\n\t'

# ──────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────
REPO_RAW="https://raw.githubusercontent.com/xfetch-cli/xfetch/main"
PROJECT="xfetch"
INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/${PROJECT}"
MAC_SUPPORT="${HOME}/Library/Application Support/${PROJECT}"
BINARY="${INSTALL_DIR}/${PROJECT}"

FLAG_YES=0
FLAG_PURGE=0

# ──────────────────────────────────────────────
# Utility functions
# ──────────────────────────────────────────────

log()   { printf '\033[1;34m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
ok()    { printf '\033[1;32m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
warn()  { printf '\033[1;33m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
error() { printf '\033[1;31m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
die()   { error "$*"; exit 1; }

usage() {
    cat <<EOF
${PROJECT} Uninstaller

Usage:
  curl -fsSL ${REPO_RAW}/uninstall.sh | bash
  bash uninstall.sh [options]

Options:
  -h, --help     Show this help message
  -y, --yes      Automatic yes to all prompts
  --purge        Also remove all config files and data (default: keep config)

Examples:
  bash uninstall.sh
  bash uninstall.sh --yes
  bash uninstall.sh --purge

Report issues: https://github.com/xscriptor/${PROJECT}/issues
EOF
    exit 0
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help) usage ;;
            -y|--yes) FLAG_YES=1 ;;
            --purge) FLAG_PURGE=1 ;;
            *) die "Unknown option: $1. Use --help for usage." ;;
        esac
        shift
    done
}

detect_os() {
    case "$(uname -s)" in
        Darwin) echo "macos" ;;
        Linux)  echo "linux" ;;
        *)      echo "other" ;;
    esac
}

# ──────────────────────────────────────────────
# Main uninstall logic
# ──────────────────────────────────────────────

main() {
    parse_args "$@"

    log "Uninstalling ${PROJECT}..."

    # Confirm
    if [ "${FLAG_YES}" -eq 0 ]; then
        printf "[%s] This will remove ${PROJECT} from your system. Continue? [y/N]: " "${PROJECT}"
        read -r response
        case "${response}" in
            y|Y|yes) ;;
            *) die "Aborted." ;;
        esac
    fi

    local removed_any=0

    # ── Remove binary ──
    if [ -f "${BINARY}" ]; then
        rm -f "${BINARY}"
        ok "Removed binary: ${BINARY}"
        removed_any=1
    else
        warn "Binary not found at ${BINARY}"
    fi

    # ── Remove macOS symlink ──
    if [ -L "${MAC_SUPPORT}" ] || [ -e "${MAC_SUPPORT}" ]; then
        rm -rf "${MAC_SUPPORT}" 2>/dev/null || true
        ok "Removed macOS config symlink: ${MAC_SUPPORT}"
        removed_any=1
    fi

    # ── Remove config (only on --purge) ──
    if [ "${FLAG_PURGE}" -eq 1 ]; then
        if [ -d "${CONFIG_DIR}" ]; then
            rm -rf "${CONFIG_DIR}"
            ok "Removed config directory: ${CONFIG_DIR}"
            removed_any=1
        else
            warn "Config directory not found at ${CONFIG_DIR}"
        fi
    else
        if [ -d "${CONFIG_DIR}" ]; then
            warn "Config directory preserved: ${CONFIG_DIR}"
            warn "  To remove it later: rm -rf '${CONFIG_DIR}'"
        fi
    fi

    # ── Summary ──
    if [ "${removed_any}" -eq 0 ]; then
        error "${PROJECT} does not appear to be installed."
        exit 1
    fi

    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
${PROJECT} — Uninstall Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Note: PATH modifications in your shell config files were NOT removed.
To manually clean up, remove these lines from your shell config:

  # ${PROJECT} path
  export PATH="${INSTALL_DIR}:\$PATH"

Common shell config files:
  ~/.bashrc  ~/.zshrc  ~/.bash_profile  ~/.zprofile  ~/.profile
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
}

main "$@"
