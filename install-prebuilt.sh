#!/usr/bin/env bash
# xfetch - cross-platform system information fetcher
# Prebuilt installer — downloads a precompiled binary from GitHub Releases.
# Fast: no Rust toolchain, no compilation. Requires only curl (and tar/unzip).
# Usage: curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install-prebuilt.sh | bash
#        bash install-prebuilt.sh --version 0.7.0
#        bash install-prebuilt.sh --prefix /usr/local --yes

set -euo pipefail
IFS=$'\n\t'

# ──────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────
REPO="xfetch-cli/xfetch"
REPO_URL="https://github.com/${REPO}"
REPO_API="https://api.github.com/repos/${REPO}"
REPO_RAW="https://raw.githubusercontent.com/${REPO}/main"

PROJECT="xfetch"
PROJECT_DESC="cross-platform system information fetcher (prebuilt binary)"

# Default paths (may be overridden by flags)
PREFIX="${PREFIX:-${HOME}/.local}"
BIN_DIR="${BIN_DIR:-${PREFIX}/bin}"
CONFIG_DIR="${CONFIG_DIR:-${HOME}/.config/${PROJECT}}"

# Behavior flags
FLAG_VERBOSE=1
FLAG_MODIFY_PATH=1
FLAG_YES=0
FLAG_SKIP_CONFIG=0
FLAG_NO_CHECKSUM=0
VERSION=""          # empty = latest release

# Runtime
TEMP_DIR=""
EXISTING_CONFIG=0
OS_NAME=""
ARCH_NAME=""
TARGET_TRIPLE=""

# ──────────────────────────────────────────────
# Utility functions
# ──────────────────────────────────────────────

log()   { printf '\033[1;34m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
ok()    { printf '\033[1;32m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
warn()  { printf '\033[1;33m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
error() { printf '\033[1;31m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
die()   { error "$*"; exit 1; }

has_tty() { [ -t 0 ]; }

cleanup() {
    local exit_code=$?
    if [ -n "${TEMP_DIR}" ] && [ -d "${TEMP_DIR}" ]; then
        rm -rf "${TEMP_DIR}" 2>/dev/null || true
    fi
    if [ "${exit_code}" -ne 0 ]; then
        error "Installation failed (exit code: ${exit_code}). See errors above."
    fi
    exit "${exit_code}"
}
trap cleanup EXIT INT TERM HUP

# ──────────────────────────────────────────────
# OS / Architecture detection
# ──────────────────────────────────────────────

detect_os() {
    case "$(uname -s)" in
        Linux)  echo "linux" ;;
        Darwin) echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)      echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             echo "unknown" ;;
    esac
}

# Map detected OS/arch to the rust target triple used in release assets
resolve_target() {
    case "${OS_NAME}-${ARCH_NAME}" in
        linux-x86_64)  TARGET_TRIPLE="x86_64-unknown-linux-gnu" ;;
        linux-aarch64) TARGET_TRIPLE="aarch64-unknown-linux-gnu" ;;
        macos-x86_64)  TARGET_TRIPLE="x86_64-apple-darwin" ;;
        macos-aarch64) TARGET_TRIPLE="aarch64-apple-darwin" ;;
        windows-x86_64) die "On Windows use PowerShell: irm ${REPO_RAW}/install.ps1 | iex" ;;
        *) die "No prebuilt binary for ${OS_NAME}-${ARCH_NAME}. Use install.sh (builds from source)." ;;
    esac
    [ "${FLAG_VERBOSE}" -eq 1 ] && log "Target: ${TARGET_TRIPLE}"
}

detect_shell_rc() {
    local shell_name
    shell_name="$(basename "${SHELL:-${HOME}}" 2>/dev/null || echo "bash")"
    shell_name="${shell_name%%[0-9]*}"
    case "${shell_name}" in
        zsh)
            if [ -n "${ZDOTDIR:-}" ]; then
                echo "${ZDOTDIR}/.zshrc"
            else
                echo "${HOME}/.zshrc"
            fi
            ;;
        bash)
            if [ "${OS_NAME}" = "macos" ]; then
                echo "${HOME}/.bash_profile"
            else
                echo "${HOME}/.bashrc"
            fi
            ;;
        fish) echo "${HOME}/.config/fish/config.fish" ;;
        *)
            for rc in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
                if [ -f "${rc}" ]; then
                    echo "${rc}"
                    return 0
                fi
            done
            echo "${HOME}/.profile"
            ;;
    esac
}

# ──────────────────────────────────────────────
# Argument parsing
# ──────────────────────────────────────────────

usage() {
    cat <<EOF
${PROJECT} — ${PROJECT_DESC}

Usage:
  curl -fsSL ${REPO_RAW}/install-prebuilt.sh | bash
  bash install-prebuilt.sh [options]

Options:
  -h, --help              Show this help message and exit
  -p, --prefix <dir>      Installation prefix (default: \${HOME}/.local)
  -b, --bin-dir <dir>     Binary install directory (default: \${PREFIX}/bin)
  -c, --config-dir <dir>  Config directory (default: \${HOME}/.config/${PROJECT})
  -n, --no-modify-path    Do not modify shell config files to add to PATH
  -y, --yes               Automatic yes to all prompts
  -s, --skip-config       Skip generating the default config
  -q, --quiet             Quiet mode (minimal output)
      --version <ver>     Install a specific version (e.g. 0.7.0); default: latest release
      --no-checksum       Skip SHA256 checksum verification

Environment variables:
  PREFIX                  Same as --prefix
  BIN_DIR                 Same as --bin-dir
  CONFIG_DIR              Same as --config-dir

Examples:
  # Quick install (latest release, no sudo needed)
  curl -fsSL ${REPO_RAW}/install-prebuilt.sh | bash

  # Install a specific version
  curl -fsSL ${REPO_RAW}/install-prebuilt.sh | bash -s -- --version 0.7.0

  # Non-interactive install
  bash install-prebuilt.sh --yes

  # System-wide install
  bash install-prebuilt.sh --prefix /usr/local --yes

  # Install without PATH modification
  bash install-prebuilt.sh --no-modify-path

Report issues: ${REPO_URL}/issues
EOF
    exit 0
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help) usage ;;
            -n|--no-modify-path) FLAG_MODIFY_PATH=0 ;;
            -y|--yes) FLAG_YES=1 ;;
            -s|--skip-config) FLAG_SKIP_CONFIG=1 ;;
            -q|--quiet) FLAG_VERBOSE=0 ;;
            --no-checksum) FLAG_NO_CHECKSUM=1 ;;
            --version)
                shift; VERSION="$1"
                ;;
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
            --) shift; break ;;
            -*)
                die "Unknown option: $1. Use --help for usage."
                ;;
            *) break ;;
        esac
        shift
    done
}

# ──────────────────────────────────────────────
# Download and checksum
# ──────────────────────────────────────────────

resolve_asset_url() {
    local release_url
    if [ -z "${VERSION}" ]; then
        release_url="${REPO_API}/releases/latest"
    else
        release_url="${REPO_API}/releases/tags/v${VERSION}"
    fi

    log "Resolving ${PROJECT} release..."
    local json
    json="$(curl -fsSL --max-time 30 "${release_url}")" || die "Cannot reach ${release_url}"

    ASSET_URL="$(printf '%s' "${json}" \
        | grep -o "\"browser_download_url\": *\"[^\"]*${TARGET_TRIPLE}[^\"]*\"" \
        | head -n 1 | sed 's/.*"browser_download_url": *"//; s/"$//')"

    if [ -z "${ASSET_URL}" ]; then
        die "No prebuilt binary for ${TARGET_TRIPLE} in this release. Use install.sh (builds from source)."
    fi

    ASSET_FILE="${ASSET_URL##*/}"
    if [ -z "${VERSION}" ]; then
        VERSION="$(printf '%s' "${ASSET_FILE}" \
            | sed "s/^${PROJECT}-//; s/-${TARGET_TRIPLE}\\.\\(tar\\.gz\\|zip\\)$//")"
    fi
    log "Found: ${ASSET_FILE} (v${VERSION})"
}

download_and_verify() {
    TEMP_DIR="$(mktemp -d)"
    log "Downloading ${ASSET_FILE}..."
    curl -fSL --max-time 300 "${ASSET_URL}" -o "${TEMP_DIR}/${ASSET_FILE}" \
        || die "Download failed: ${ASSET_URL}"

    if [ "${FLAG_NO_CHECKSUM}" -eq 1 ]; then
        warn "Skipping checksum verification (--no-checksum)."
        return 0
    fi

    local sums_url="${REPO_URL}/releases/download/v${VERSION}/SHA256SUMS"
    if curl -fsSL --max-time 30 "${sums_url}" -o "${TEMP_DIR}/SHA256SUMS" 2>/dev/null; then
        local expected actual
        expected="$(grep "${ASSET_FILE}" "${TEMP_DIR}/SHA256SUMS" | awk '{print $1}')"
        if [ -z "${expected}" ]; then
            warn "No checksum found for ${ASSET_FILE} in SHA256SUMS."
            return 0
        fi
        actual="$(sha256sum "${TEMP_DIR}/${ASSET_FILE}" | awk '{print $1}')"
        if [ "${expected}" != "${actual}" ]; then
            die "Checksum mismatch for ${ASSET_FILE}. Refusing to install."
        fi
        ok "Checksum verified (${actual:0:12}...)."
    else
        warn "Could not fetch ${sums_url}. Skipping checksum verification."
    fi
}

extract_binary() {
    local archive="${TEMP_DIR}/${ASSET_FILE}"
    case "${ASSET_FILE}" in
        *.tar.gz) tar -xzf "${archive}" -C "${TEMP_DIR}" ;;
        *.zip)    unzip -q "${archive}" -d "${TEMP_DIR}" ;;
        *)        die "Unsupported archive format: ${ASSET_FILE}" ;;
    esac

    BINARY_SRC="${TEMP_DIR}/${PROJECT}"
    if [ ! -f "${BINARY_SRC}" ]; then
        die "Binary not found inside ${ASSET_FILE}. Expected: ${PROJECT}"
    fi
}

# ──────────────────────────────────────────────
# Install
# ──────────────────────────────────────────────

install_binary() {
    mkdir -p "${BIN_DIR}"
    if command -v install >/dev/null 2>&1; then
        install -m 755 "${BINARY_SRC}" "${BIN_DIR}/${PROJECT}"
    else
        cp "${BINARY_SRC}" "${BIN_DIR}/${PROJECT}"
        chmod 755 "${BIN_DIR}/${PROJECT}"
    fi
    ok "Installed binary: ${BIN_DIR}/${PROJECT}"
}

install_config() {
    if [ "${FLAG_SKIP_CONFIG}" -eq 1 ]; then
        log "Skipping config generation (--skip-config)."
        return 0
    fi

    mkdir -p "${CONFIG_DIR}"

    if [ -f "${CONFIG_DIR}/config.jsonc" ]; then
        EXISTING_CONFIG=1
        warn "Config already exists at ${CONFIG_DIR}/config.jsonc — not overwriting."
    else
        if "${BIN_DIR}/${PROJECT}" --gen-config >/dev/null 2>&1; then
            ok "Generated config at ${CONFIG_DIR}/config.jsonc"
        else
            warn "Could not generate config; skipping."
        fi
    fi

    if [ "${OS_NAME}" = "macos" ]; then
        local mac_support="${HOME}/Library/Application Support/${PROJECT}"
        if [ ! -e "${mac_support}" ]; then
            ln -sf "${CONFIG_DIR}" "${mac_support}"
            ok "Created macOS config symlink: ${mac_support} -> ${CONFIG_DIR}"
        fi
    fi
}

# ──────────────────────────────────────────────
# PATH modification
# ──────────────────────────────────────────────

ensure_path_in_file() {
    local file="$1"
    local path_line="$2"
    local comment="$3"

    if [ ! -f "${file}" ]; then
        mkdir -p "$(dirname "${file}")"
        touch "${file}"
    fi

    if grep -qsF "# ${comment}" "${file}" 2>/dev/null; then
        [ "${FLAG_VERBOSE}" -eq 1 ] && ok "PATH already configured in ${file}"
        return 0
    fi

    printf '\n# %s\n%s\n' "${comment}" "${path_line}" >> "${file}"
    ok "Added ${BIN_DIR} to PATH in ${file}"
}

modify_path() {
    local primary_rc
    primary_rc="$(detect_shell_rc)"

    local fish_rc="${HOME}/.config/fish/config.fish"
    local -a rc_list=()

    if [ "${OS_NAME}" = "macos" ]; then
        rc_list=("${HOME}/.bash_profile" "${HOME}/.zprofile" "${HOME}/.zshrc")
        [ -f "${HOME}/.bashrc" ] && rc_list+=("${HOME}/.bashrc")
    else
        rc_list=("${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile")
        [ -f "${HOME}/.bash_profile" ] && rc_list+=("${HOME}/.bash_profile")
    fi
    if [ -f "${fish_rc}" ] || [ "${primary_rc}" = "${fish_rc}" ]; then
        rc_list+=("${fish_rc}")
    fi

    local comment="${PROJECT} path"
    local path_line

    for rc in "${rc_list[@]}"; do
        if [ -f "${rc}" ] || [ "${rc}" = "${primary_rc}" ]; then
            if [ "${rc}" = "${fish_rc}" ]; then
                path_line="fish_add_path ${BIN_DIR}"
            else
                path_line="export PATH=\"${BIN_DIR}:\$PATH\""
            fi
            ensure_path_in_file "${rc}" "${path_line}" "${comment}"
        fi
    done

    if [ "${primary_rc}" = "${fish_rc}" ]; then
        ok "To add ${BIN_DIR} to your current session, run: fish_add_path ${BIN_DIR}"
    else
        ok "To add ${BIN_DIR} to your current session, run: source ${primary_rc}"
    fi
}

# ──────────────────────────────────────────────
# Verification / Summary
# ──────────────────────────────────────────────

verify_installation() {
    local binary="${BIN_DIR}/${PROJECT}"
    if [ ! -f "${binary}" ]; then
        error "Binary not found at ${binary}"
        return 1
    fi
    if [ ! -x "${binary}" ]; then
        error "Binary is not executable: ${binary}"
        return 1
    fi
    if "${binary}" --version >/dev/null 2>&1; then
        local version_output
        version_output="$("${binary}" --version 2>&1)"
        ok "Verified: ${version_output}"
    else
        warn "Binary installed at ${binary} but could not verify version."
    fi
    return 0
}

print_summary() {
    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
${PROJECT} — Installation Complete (prebuilt)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  OS:              ${OS_NAME} (${ARCH_NAME})
  Version:         v${VERSION}
  Binary:          ${BIN_DIR}/${PROJECT}
  Config:          ${CONFIG_DIR}/

EOF

    if [ "${EXISTING_CONFIG}" -eq 1 ]; then
        cat <<EOF
    Existing config preserved at ${CONFIG_DIR}/config.jsonc
EOF
    fi

    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        local shell_rc
        shell_rc="$(detect_shell_rc)"
        if [ "${shell_rc}" = "${HOME}/.config/fish/config.fish" ]; then
            cat <<EOF
  PATH updated in:  ${shell_rc}
  Restart your terminal (or run 'fish_add_path ${BIN_DIR}') to use ${PROJECT}.
EOF
        else
            cat <<EOF
  PATH updated in:  ${shell_rc}
  Restart your terminal or run 'source ${shell_rc}' to use ${PROJECT}.
EOF
        fi
    else
        cat <<EOF
  PATH not modified. Add ${BIN_DIR} to your PATH manually:
    export PATH="${BIN_DIR}:\$PATH"
EOF
    fi

    cat <<EOF

  ${PROJECT} is ready! Run it:
    ${PROJECT}

  For configuration help:
    ${PROJECT} --help
    ${REPO_URL}

  To uninstall:
    rm -f "${BIN_DIR}/${PROJECT}"
    rm -rf "${CONFIG_DIR}"

  Uninstall script available at:
    ${REPO_RAW}/uninstall.sh
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────

main() {
    parse_args "$@"

    OS_NAME="$(detect_os)"
    ARCH_NAME="$(detect_arch)"
    resolve_target

    log "Installing ${PROJECT} (prebuilt) on ${OS_NAME} (${ARCH_NAME})"

    if [ "${EUID:-${UID}}" = "0" ] && [ "${FLAG_YES}" -eq 0 ]; then
        warn "Running as root is not recommended for local installs."
        if has_tty; then
            printf "[%s] Continue as root? [y/N]: " "${PROJECT}"
            read -r response
            case "${response}" in
                y|Y|yes) ;;
                *) die "Aborted. Run as a normal user, or use --yes to skip this check." ;;
            esac
        else
            die "Running as root without an interactive terminal. Re-run with --yes to skip this check."
        fi
    fi

    if ! command -v curl >/dev/null 2>&1; then
        die "curl is required. Install it and try again."
    fi
    if ! command -v sha256sum >/dev/null 2>&1; then
        warn "sha256sum not found; checksum verification will be skipped."
        FLAG_NO_CHECKSUM=1
    fi
    case "${OS_NAME}" in
        linux)   command -v tar >/dev/null 2>&1 || die "tar is required." ;;
        macos)   command -v tar >/dev/null 2>&1 || die "tar is required." ;;
    esac

    if ! mkdir -p "${BIN_DIR}" 2>/dev/null; then
        die "Cannot write to ${BIN_DIR}. Check HOME permissions."
    fi

    resolve_asset_url
    download_and_verify
    extract_binary
    install_binary
    install_config

    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        modify_path
    else
        log "Skipping PATH modification (--no-modify-path)."
    fi

    rm -rf "${TEMP_DIR}" 2>/dev/null || true
    TEMP_DIR=""

    verify_installation || die "Installation verification failed."
    print_summary
}

main "$@"
