#!/usr/bin/env bash
# xfetch - cross-platform system information fetcher
# Installer script — supports remote (curl | bash) and local installation
# Usage: curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash
#        bash install.sh --local
#        bash install.sh --prefix /usr/local

set -euo pipefail
IFS=$'\n\t'

# ──────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────
REPO_URL="https://github.com/xfetch-cli/xfetch.git"
REPO_RAW="https://raw.githubusercontent.com/xfetch-cli/xfetch/main"

VERSION="0.1.1"
PROJECT="xfetch"
PROJECT_DESC="cross-platform system information fetcher"

# Default paths (may be overridden by flags)
PREFIX="${PREFIX:-${HOME}/.local}"
BIN_DIR="${BIN_DIR:-${PREFIX}/bin}"
CONFIG_DIR="${CONFIG_DIR:-${HOME}/.config/${PROJECT}}"
DATA_DIR="${DATA_DIR:-${PREFIX}/share/${PROJECT}}"

# Behavior flags
FLAG_LOCAL=0
FLAG_MODIFY_PATH=1
FLAG_YES=0
FLAG_VERBOSE=0
FLAG_SKIP_CONFIG=0
FLAG_SKIP_CARGO_INSTALL=0

# Runtime
TEMP_DIR=""
EXISTING_CONFIG=0
SHELL_RC_FILES=""

# ──────────────────────────────────────────────
# Utility functions
# ──────────────────────────────────────────────

log()   { printf '\033[1;34m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
ok()    { printf '\033[1;32m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
warn()  { printf '\033[1;33m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
error() { printf '\033[1;31m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
die()   { error "$*"; exit 1; }

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
        FreeBSD) echo "freebsd" ;;
        OpenBSD) echo "openbsd" ;;
        NetBSD)  echo "netbsd" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)       echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        armv7l|armhf)  echo "armv7" ;;
        i686|i386)     echo "i686" ;;
        riscv64)       echo "riscv64" ;;
        *)             echo "unknown" ;;
    esac
}

detect_shell_rc() {
    # Detect the user's preferred shell config file
    local shell_name
    shell_name="$(basename "${SHELL:-${HOME}}" 2>/dev/null || echo "bash")"
    # Strip trailing version suffixes like zsh5 -> zsh
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
            if [ "$(detect_os)" = "macos" ]; then
                echo "${HOME}/.bash_profile"
            else
                echo "${HOME}/.bashrc"
            fi
            ;;
        fish) echo "${HOME}/.config/fish/config.fish" ;;
        *)
            # Fallback: check common rc files in order of preference
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
${PROJECT} ${VERSION} — ${PROJECT_DESC}

Usage:
  curl -fsSL ${REPO_RAW}/install.sh | bash
  bash install.sh [options]

Options:
  -h, --help              Show this help message and exit
  -V, --version           Show version and exit
  -l, --local             Install from local source (skip git clone; run from repo root)
  -p, --prefix <dir>      Installation prefix (default: \${HOME}/.local)
  -b, --bin-dir <dir>     Binary install directory (default: \${PREFIX}/bin)
  -c, --config-dir <dir>  Config directory (default: \${HOME}/.config/${PROJECT})
  -n, --no-modify-path    Do not modify shell config files to add to PATH
  -y, --yes               Automatic yes to all prompts
  -s, --skip-config       Skip copying default config files
  -q, --quiet             Quiet mode (minimal output)
  -v, --verbose           Verbose output
  --no-cargo-install      Skip cargo install (assume binary already built)

Environment variables:
  PREFIX                  Same as --prefix
  BIN_DIR                 Same as --bin-dir
  CONFIG_DIR              Same as --config-dir
  DATA_DIR                Data files directory

Examples:
  # Quick install (remote)
  curl -fsSL ${REPO_RAW}/install.sh | bash

  # Local install from cloned repo
  bash install.sh --local

  # System-wide install
  bash install.sh --prefix /usr/local --yes

  # Install without PATH modification
  bash install.sh --no-modify-path

Report issues: ${REPO_URL}/issues
EOF
    exit 0
}

version() {
    echo "${PROJECT} installer version ${VERSION}"
    exit 0
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help) usage ;;
            -V|--version) version ;;
            -l|--local) FLAG_LOCAL=1 ;;
            -n|--no-modify-path) FLAG_MODIFY_PATH=0 ;;
            -y|--yes) FLAG_YES=1 ;;
            -s|--skip-config) FLAG_SKIP_CONFIG=1 ;;
            -v|--verbose) FLAG_VERBOSE=1 ;;
            -q|--quiet) FLAG_VERBOSE=0 ;;
            --no-cargo-install) FLAG_SKIP_CARGO_INSTALL=1 ;;
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
# Dependency checks
# ──────────────────────────────────────────────

check_deps() {
    local missing=0

    if [ "${FLAG_LOCAL}" -eq 0 ]; then
        if ! command -v git >/dev/null 2>&1; then
            warn "git is required for remote installation."
            warn "Install git, or use --local to install from a local clone."
            missing=1
        fi
    fi

    if [ "${FLAG_SKIP_CARGO_INSTALL}" -eq 0 ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            if [ "${FLAG_YES}" -eq 1 ]; then
                warn "Rust (cargo) is not installed. Will attempt to install Rust via rustup..."
            else
                warn "Rust (cargo) is not installed."
                warn "The installer can install Rust via rustup for you."
                printf "[%s] Install Rust now? [Y/n]: " "${PROJECT}"
                read -r response
                case "${response}" in
                    n|N|no) die "Aborted. Please install Rust manually: https://rustup.rs/" ;;
                    *) ;;
                esac
            fi
            install_rust
        fi
    fi

    # Check for the 'install' command (coreutils) on Linux
    local os
    os="$(detect_os)"
    if [ "${os}" != "macos" ]; then
        if ! command -v install >/dev/null 2>&1; then
            warn "coreutils 'install' command not found. Will use cp fallback."
        fi
    fi

    if [ "${missing}" -eq 1 ]; then
        die "Missing required dependencies. Please install them and try again."
    fi
}

install_rust() {
    log "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y 2>/dev/null || die "Failed to install Rust."
    # Source the env so cargo is available immediately
    if [ -f "${HOME}/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "${HOME}/.cargo/env"
    fi
    ok "Rust installed successfully."
}

# ──────────────────────────────────────────────
# PATH modification
# ──────────────────────────────────────────────

ensure_path_in_file() {
    local file="$1"
    local path_line="$2"
    local comment="$3"

    if [ ! -f "${file}" ]; then
        [ "${FLAG_VERBOSE}" -eq 1 ] && log "Creating ${file}..."
        touch "${file}"
    fi

    # Check if the path line already exists in any form
    if grep -qsF "${BIN_DIR}" "${file}" 2>/dev/null; then
        [ "${FLAG_VERBOSE}" -eq 1 ] && ok "PATH already configured in ${file}"
        return 0
    fi

    printf '\n# %s\n%s\n' "${comment}" "${path_line}" >> "${file}"
    ok "Added ${BIN_DIR} to PATH in ${file}"
}

modify_path() {
    local primary_rc
    local os
    os="$(detect_os)"

    # Determine which rc files to update
    if [ "${os}" = "macos" ]; then
        SHELL_RC_FILES="${HOME}/.bash_profile ${HOME}/.zprofile ${HOME}/.zshrc"
        # Also check for .bashrc on macOS (common with iTerm2)
        [ -f "${HOME}/.bashrc" ] && SHELL_RC_FILES="${SHELL_RC_FILES} ${HOME}/.bashrc"
    else
        SHELL_RC_FILES="${HOME}/.bashrc ${HOME}/.zshrc ${HOME}/.profile"
        [ -f "${HOME}/.bash_profile" ] && SHELL_RC_FILES="${SHELL_RC_FILES} ${HOME}/.bash_profile"
    fi

    local path_line="export PATH=\"${BIN_DIR}:\$PATH\""
    local comment="${PROJECT} path"

    for rc in ${SHELL_RC_FILES}; do
        # Only modify files that exist (or the primary shell rc)
        if [ -f "${rc}" ] || [ "${rc}" = "$(detect_shell_rc)" ]; then
            ensure_path_in_file "${rc}" "${path_line}" "${comment}"
        fi
    done

    local shell_rc
    shell_rc="$(detect_shell_rc)"
    ok "To add ${BIN_DIR} to your current session, run: source ${shell_rc}"
}

# ──────────────────────────────────────────────
# Build and install
# ──────────────────────────────────────────────

build_project() {
    local src_dir="$1"

    log "Building ${PROJECT} (release mode)..."
    (cd "${src_dir}" && cargo build --release)
    ok "Build completed successfully."
}

install_binary() {
    local src_dir="$1"

    mkdir -p "${BIN_DIR}"

    local binary_src="${src_dir}/target/release/${PROJECT}"
    if [ ! -f "${binary_src}" ]; then
        # Try alternate location
        binary_src="${src_dir}/target/release/${PROJECT}.exe"
    fi
    if [ ! -f "${binary_src}" ]; then
        die "Binary not found after build. Expected: ${binary_src}"
    fi

    if command -v install >/dev/null 2>&1; then
        install -m 755 "${binary_src}" "${BIN_DIR}/${PROJECT}"
    else
        cp "${binary_src}" "${BIN_DIR}/${PROJECT}"
        chmod 755 "${BIN_DIR}/${PROJECT}"
    fi

    ok "Installed binary: ${BIN_DIR}/${PROJECT}"
}

install_config() {
    local src_dir="$1"

    if [ "${FLAG_SKIP_CONFIG}" -eq 1 ]; then
        log "Skipping config installation (--skip-config)."
        return 0
    fi

    mkdir -p "${CONFIG_DIR}"

    # Check if config already exists
    if [ -f "${CONFIG_DIR}/config.jsonc" ]; then
        EXISTING_CONFIG=1
        warn "Config already exists at ${CONFIG_DIR}/config.jsonc — not overwriting."
    else
        if [ -f "${src_dir}/configs/config.jsonc" ]; then
            cp "${src_dir}/configs/config.jsonc" "${CONFIG_DIR}/config.jsonc"
            ok "Installed default config to ${CONFIG_DIR}/config.jsonc"
        else
            warn "No default config found in source; skipping."
        fi
    fi

    # Install logos
    if [ -d "${src_dir}/logos" ]; then
        mkdir -p "${CONFIG_DIR}/logos"
        cp -r "${src_dir}/logos/"* "${CONFIG_DIR}/logos/" 2>/dev/null || true
        ok "Installed logos to ${CONFIG_DIR}/logos/"
    fi

    # macOS: create symlink for Library/Application Support
    local os
    os="$(detect_os)"
    if [ "${os}" = "macos" ]; then
        local mac_support="${HOME}/Library/Application Support/${PROJECT}"
        if [ ! -e "${mac_support}" ]; then
            ln -sf "${CONFIG_DIR}" "${mac_support}"
            ok "Created macOS config symlink: ${mac_support} -> ${CONFIG_DIR}"
        fi
    fi
}

# ──────────────────────────────────────────────
# Verification
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

    # Try running the binary
    if "${binary}" --version >/dev/null 2>&1; then
        local version_output
        version_output="$("${binary}" --version 2>&1)"
        ok "Verified: ${version_output}"
    else
        warn "Binary installed at ${binary} but could not verify version."
        warn "It may still work; run '${binary}' directly to test."
    fi

    return 0
}

print_summary() {
    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
${PROJECT} ${VERSION} — Installation Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  OS:              ${os} (${arch})
  Binary:          ${BIN_DIR}/${PROJECT}
  Config:          ${CONFIG_DIR}/
  Data:            ${DATA_DIR}/

EOF

    if [ "${EXISTING_CONFIG}" -eq 1 ]; then
        cat <<EOF
  ⚠  Existing config preserved at ${CONFIG_DIR}/config.jsonc
EOF
    fi

    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        local shell_rc
        shell_rc="$(detect_shell_rc)"
        cat <<EOF
  PATH updated in:  ${shell_rc}
  Restart your terminal or run 'source ${shell_rc}' to use ${PROJECT}.
EOF
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

    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    log "Installing ${PROJECT} ${VERSION} on ${os} (${arch})"

    # Check we're not running as root unnecessarily (for local installs)
    if [ "${EUID:-${UID}}" = "0" ] && [ "${FLAG_YES}" -eq 0 ]; then
        warn "Running as root is not recommended for local installs."
        printf "[%s] Continue as root? [y/N]: " "${PROJECT}"
        read -r response
        case "${response}" in
            y|Y|yes) ;;
            *) die "Aborted. Run as a normal user, or use --yes to skip this check." ;;
        esac
    fi

    check_deps

    # Determine source directory
    local src_dir=""
    if [ "${FLAG_LOCAL}" -eq 1 ]; then
        src_dir="$(pwd)"
        if [ ! -f "${src_dir}/Cargo.toml" ]; then
            die "No Cargo.toml found in ${src_dir}. Run --local from the project root."
        fi
        log "Using local source: ${src_dir}"
    else
        TEMP_DIR="$(mktemp -d)"
        src_dir="${TEMP_DIR}/${PROJECT}"

        log "Cloning repository from ${REPO_URL}..."
        git clone --depth 1 "${REPO_URL}" "${src_dir}"
        ok "Repository cloned."
    fi

    # Build
    if [ "${FLAG_SKIP_CARGO_INSTALL}" -eq 1 ]; then
        log "Skipping cargo build (--no-cargo-install). Checking for pre-built binary..."
        if [ ! -f "${src_dir}/target/release/${PROJECT}" ]; then
            die "No pre-built binary found. Remove --no-cargo-install to build from source."
        fi
        ok "Using pre-built binary."
    else
        build_project "${src_dir}"
    fi

    # Install binary
    install_binary "${src_dir}"

    # Install config
    install_config "${src_dir}"

    # Modify PATH
    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        modify_path
    else
        log "Skipping PATH modification (--no-modify-path)."
    fi

    # Clean up temp dir (if we cloned)
    if [ -n "${TEMP_DIR}" ] && [ "${FLAG_LOCAL}" -eq 0 ]; then
        rm -rf "${TEMP_DIR}" 2>/dev/null || true
        TEMP_DIR=""
    fi

    # Verify
    verify_installation || die "Installation verification failed."

    # Summary
    print_summary
}

main "$@"
