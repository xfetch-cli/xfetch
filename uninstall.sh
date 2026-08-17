#!/usr/bin/env bash
# xfetch - cross-platform system information fetcher
# Uninstaller script
# Usage: curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/uninstall.sh | bash
#        bash uninstall.sh [options]

set -euo pipefail
IFS=$'\n\t'

# ──────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────
REPO_RAW="https://raw.githubusercontent.com/xfetch-cli/xfetch/main"
PROJECT="xfetch"

# Defaults mirror install.sh (may be overridden by flags)
PREFIX="${PREFIX:-${HOME}/.local}"
BIN_DIR="${BIN_DIR:-${PREFIX}/bin}"
CONFIG_DIR="${CONFIG_DIR:-${HOME}/.config/${PROJECT}}"
DATA_DIR="${DATA_DIR:-${PREFIX}/share/${PROJECT}}"
MAC_SUPPORT="${HOME}/Library/Application Support/${PROJECT}"

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
  -h, --help              Show this help message
  -y, --yes               Automatic yes to all prompts
  --purge                 Also remove all config files and data (default: keep config)
  -p, --prefix <dir>      Installation prefix (default: \${HOME}/.local)
  -b, --bin-dir <dir>     Binary install directory (default: \${PREFIX}/bin)
  -c, --config-dir <dir>  Config directory (default: \${HOME}/.config/${PROJECT})

Environment variables:
  PREFIX                  Same as --prefix
  BIN_DIR                 Same as --bin-dir
  CONFIG_DIR              Same as --config-dir
  DATA_DIR                Data files directory

Examples:
  bash uninstall.sh
  bash uninstall.sh --yes
  bash uninstall.sh --purge
  bash uninstall.sh --prefix /usr/local --yes
  bash uninstall.sh --purge --bin-dir /opt/${PROJECT}/bin --config-dir /etc/${PROJECT}

Report issues: https://github.com/xfetch-cli/${PROJECT}/issues
EOF
    exit 0
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help) usage ;;
            -y|--yes) FLAG_YES=1 ;;
            --purge) FLAG_PURGE=1 ;;
            -p|--prefix)
                shift; PREFIX="$1"
                [ -z "${BIN_DIR_OVERRIDE:-}" ] && BIN_DIR="${PREFIX}/bin"
                ;;
            -b|--bin-dir)
                shift; BIN_DIR="$1"; BIN_DIR_OVERRIDE=1
                ;;
            -c|--config-dir)
                shift; CONFIG_DIR="$1"
                ;;
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
# PATH cleanup
# ──────────────────────────────────────────────

remove_path_entries() {
    local os fish_rc rc removed=0
    local -a rc_list=()
    os="$(detect_os)"
    fish_rc="${HOME}/.config/fish/config.fish"

    if [ "${os}" = "macos" ]; then
        rc_list=("${HOME}/.bash_profile" "${HOME}/.zprofile" "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile")
    else
        rc_list=("${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile" "${HOME}/.bash_profile" "${HOME}/.zprofile")
    fi
    rc_list+=("${fish_rc}")

    for rc in "${rc_list[@]}"; do
        [ -f "${rc}" ] || continue
        if grep -qsF "# ${PROJECT} path" "${rc}"; then
            # Remove the marker comment and the line that follows it
            sed -i.bak "/^# ${PROJECT} path$/,+1d" "${rc}"
            rm -f "${rc}.bak"
            ok "Removed PATH entry from ${rc}"
            removed=1
        fi
    done

    if [ "${removed}" -eq 1 ]; then
        ok "PATH entries removed from shell config files."
    fi
    return 0
}

# ──────────────────────────────────────────────
# Main uninstall logic
# ──────────────────────────────────────────────

main() {
    parse_args "$@"

    local binary="${BIN_DIR}/${PROJECT}"

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
    if [ -f "${binary}" ]; then
        rm -f "${binary}"
        ok "Removed binary: ${binary}"
        removed_any=1
    else
        warn "Binary not found at ${binary}"
    fi

    # ── Remove leftover from 'cargo install' (e.g. install.ps1) ──
    local cargo_bin="${HOME}/.cargo/bin/${PROJECT}"
    if [ -f "${cargo_bin}" ] || [ -f "${cargo_bin}.exe" ]; then
        rm -f "${cargo_bin}" "${cargo_bin}.exe"
        ok "Removed binary: ${cargo_bin} (from cargo install)"
        removed_any=1
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
        if [ -d "${DATA_DIR}" ]; then
            rm -rf "${DATA_DIR}"
            ok "Removed data directory: ${DATA_DIR}"
            removed_any=1
        fi
    else
        if [ -d "${CONFIG_DIR}" ]; then
            warn "Config directory preserved: ${CONFIG_DIR}"
            warn "  To remove it later: rm -rf '${CONFIG_DIR}'"
        fi
    fi

    # ── Remove PATH entries added by install.sh ──
    remove_path_entries

    # ── Summary ──
    if [ "${removed_any}" -eq 0 ]; then
        error "${PROJECT} does not appear to be installed."
        exit 1
    fi

    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
${PROJECT} — Uninstall Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Binary:    ${BIN_DIR}/${PROJECT}
  Config:    ${CONFIG_DIR}/

EOF

    if [ "${FLAG_PURGE}" -eq 1 ]; then
        echo "  Config and data directories were removed (--purge)."
    else
        echo "  Config directory was preserved. Re-run with --purge to remove it."
    fi
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

main "$@"
